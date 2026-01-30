use std::{collections::HashSet, time::Duration};

use anyhow::Result;
use chrono::NaiveDate;
use indicatif::{ProgressBar, ProgressFinish, ProgressIterator, ProgressStyle};
use reqwest::Client;
use scraper::{Html, Selector};

use crate::{
    INFERENCE_SERVICE,
    dates::{DateRange, DateSet, TimeFrame, italian_month_to_number},
    events::Event,
    inference::SUMMARY_PROMPT,
    utils::PROGRESS_BAR_TEMPLATE,
};

pub async fn fetch(client: &Client, date_range: &DateRange) -> Result<Vec<Event>> {
    let mut events: HashSet<Event> = HashSet::new();

    let url = "https://www.teatroverdi-trieste.com/it/calendario-spettacoli/";
    let html_body = client
        .get(url)
        .send()
        .await
        .inspect_err(|e| println!("GET request failed: {e}"))?
        .text()
        .await?;

    let document = Html::parse_document(&html_body);
    let shows_sel = Selector::parse("ul.spettacolo-list div.list-text").unwrap();
    let link_sel = Selector::parse("h2.spettacolo-list-title > a").unwrap();
    let date_sel = Selector::parse("span.spettacolo-list-date > strong").unwrap();

    let show_count = document.select(&shows_sel).count();
    let progress = ProgressBar::new(show_count as u64)
        .with_style(ProgressStyle::with_template(PROGRESS_BAR_TEMPLATE).unwrap())
        .with_message("Fetching Verdi")
        .with_finish(ProgressFinish::AndLeave);

    for show in document.select(&shows_sel).progress_with(progress) {
        let link_el = show.select(&link_sel).next();
        if let None = link_el {
            continue;
        }
        let title = link_el
            .and_then(|el| el.text().next())
            .map(|t| t.to_string())
            .expect("Each link element should have text");

        let date_el = show.select(&date_sel).next();
        if let None = date_el {
            continue;
        }
        let date_str = date_el.and_then(|el| el.text().next()).unwrap();
        let dates = parse_date(date_str).expect("Date should be in a standardized format");

        // Skip events not in the current week
        if !dates.as_range().overlaps(&date_range) {
            continue;
        }
        let time_frame = TimeFrame::Dates(dates);

        let location = HashSet::from_iter(["Verdi".to_string()]);

        let (description, summary) =
            get_description(client, &link_el.unwrap().attr("href").unwrap())
                .await
                .unwrap_or((None, None));

        let event = Event::new(&title, location, "Teatri")
            .with_time_frame(Some(time_frame))
            .with_description(description)
            .with_summary(summary);

        events.insert(event);

        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    return Ok(events.into_iter().collect());
}

/// Parse a date string from Verdi data and return a DateRange
///
/// This function handles these formats:
/// - Single dates: "Martedì 23 dicembre 2025 ore 19.30"
/// - Multiple dates: "28, 30 novembre, 5, 7, 11, 13 dicembre 2025"
fn parse_date(date_str: &str) -> Option<DateSet> {
    let trimmed = date_str.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.contains(',') {
        return parse_multiple_dates(trimmed);
    } else {
        return parse_single_date(trimmed);
    }
}

/// Parse a single date string (e.g., "Martedì 23 dicembre 2025 ore 19.30")
fn parse_single_date(date_str: &str) -> Option<DateSet> {
    let parts: Vec<&str> = date_str.split_whitespace().collect();

    // Expected format: [day_name] [day] [month] [year] ore [time]
    // Indexes:         0          1     2       3      4   5
    if parts.len() < 4 {
        return None;
    }

    let month = italian_month_to_number(parts[2])?;
    let date_str = format!("{}/{}/{}", parts[1], month, parts[3]); // e.g. 23/12/2025
    let date = NaiveDate::parse_from_str(&date_str, "%d/%m/%Y").ok()?;

    // For single dates, create a date range that spans one day
    return Some(DateSet::new(vec![date]).unwrap());
}

/// Parse multiple dates string (e.g., "28, 30 novembre, 5, 7, 11, 13 dicembre 2025")
fn parse_multiple_dates(date_str: &str) -> Option<DateSet> {
    // Split by comma to get individual date parts
    let parts: Vec<&str> = date_str.split(',').collect();

    if parts.len() < 2 {
        return None;
    }

    // Create a vector of 3-tuples: (Option<day>, Option<month>, Option<year>)
    // We add data as we find it in the string, so that months and years can be deferred until later
    // By the end of the string, all fields should be Some
    let mut date_tuples: Vec<(Option<u32>, Option<u32>, Option<&str>)> = Vec::new();

    for (i, part) in parts.iter().enumerate() {
        let trimmed_part = part.trim();

        if i == parts.len() - 1 {
            // Last part is "[day] [month] [year]"
            let sub_parts: Vec<&str> = trimmed_part.split_whitespace().collect();
            if sub_parts.len() < 3 {
                return None;
            }

            let day = Some(sub_parts[0].parse::<u32>().unwrap());
            let year = Some(sub_parts[2]);
            if let Some(month) = italian_month_to_number(sub_parts[1]) {
                date_tuples.push((day, Some(month), year));
            } else {
                eprintln!("Failed to parse month name {}", sub_parts[1]);
            }
        } else {
            // Regular date part is either "[day]" or "[day] [month]"
            let sub_parts: Vec<&str> = trimmed_part.split_whitespace().collect();

            if sub_parts.len() == 1 {
                // [day]
                let day = Some(sub_parts[0].parse::<u32>().unwrap());
                date_tuples.push((day, None, None));
            } else if sub_parts.len() == 2 {
                // [day] [month]
                let day = Some(sub_parts[0].parse::<u32>().unwrap());
                if let Some(month) = italian_month_to_number(sub_parts[1]) {
                    date_tuples.push((day, Some(month), None));
                } else {
                    eprintln!("Failed to parse month name {}", sub_parts[1]);
                }
            }
        }
    }

    // Tuple vector is now partially completed. We now go backwards to fill every missing field
    let mut current_month = None;
    let mut current_year = None;
    for (_day, month, year) in date_tuples.iter_mut().rev() {
        // Each time we find a new month or year, set that as what needs to be propagated
        // Otherwise, fill the blank space with the currently propagating month or year
        if month.is_some() {
            current_month = *month
        } else {
            *month = current_month
        }
        if year.is_some() {
            current_year = *year
        } else {
            *year = current_year
        }
    }

    // Now convert all tuples to actual dates
    let mut dates = Vec::new();
    for (day, month, year) in date_tuples {
        if let (Some(d), Some(m), Some(y)) = (day, month, year) {
            let date_str = format!("{}/{}/{}", d, m, y);
            if let Ok(date) = NaiveDate::parse_from_str(&date_str, "%d/%m/%Y") {
                dates.push(date);
            }
        }
    }

    if dates.is_empty() {
        return None;
    } else {
        return Some(DateSet::new(dates).unwrap());
    }
}

async fn get_description(client: &Client, url: &str) -> Result<(Option<String>, Option<String>)> {
    let desc_sel = Selector::parse("section.mnk-block.spettacolo-block:not([id]) div").unwrap();

    let html_body = client.get(url).send().await?.text().await?;
    let document = Html::parse_document(&html_body);
    let desc_els = document.select(&desc_sel);

    if desc_els.clone().count() == 0 {
        println!("No desc_els");
        return Ok((None, None));
    }

    let description = desc_els.fold(String::new(), |acc, el| {
        let text = el
            .text()
            .filter(|t| !t.trim().is_empty())
            .fold(String::new(), |acc, t| format!("{acc}. {t}"))
            .trim()
            .replace("\n", "");
        format!("{acc}. {text}",)
    });

    let prompt = format!("{SUMMARY_PROMPT}\n\n{description}");
    let summary = INFERENCE_SERVICE
        .infer(&prompt)
        .await
        .inspect_err(|err| eprintln!("Failed to generate summary: {err}"))
        .ok();

    return Ok((Some(description), summary));
}

#[cfg(test)]
mod tests {
    use chrono::Datelike;

    use super::*;

    #[test]
    fn test_single_date() {
        let range = parse_date("Martedì 23 dicembre 2025 ore 19.30").unwrap();
        assert_eq!(range.first().day(), 23);
        assert_eq!(range.last().day(), 23); // Single date = same start and end
        assert_eq!(range.first().month(), 12);
        assert_eq!(range.first().year(), 2025);
    }

    #[test]
    fn test_single_date_without_time() {
        let range = parse_date("Mercoledì 31 dicembre 2025").unwrap();
        assert_eq!(range.first().day(), 31);
        assert_eq!(range.last().day(), 31);
        assert_eq!(range.first().month(), 12);
        assert_eq!(range.first().year(), 2025);
    }

    #[test]
    fn test_multiple_dates_same_month() {
        let result = parse_date("9, 10, 11, 13 gennaio 2026").unwrap();
        assert_eq!(result.first().day(), 9);
        assert_eq!(result.first().month(), 1);
        assert_eq!(result.first().year(), 2026);
        assert_eq!(result.last().day(), 13);
        assert_eq!(result.last().month(), 1);
        assert_eq!(result.last().year(), 2026);
    }

    #[test]
    fn test_multiple_dates_diff_month() {
        let result = parse_date("28, 30 novembre, 5, 7, 11, 13 dicembre 2025").unwrap();
        assert_eq!(result.first().day(), 28);
        assert_eq!(result.first().month(), 11);
        assert_eq!(result.first().year(), 2025);
        assert_eq!(result.last().day(), 13);
        assert_eq!(result.last().month(), 12);
        assert_eq!(result.last().year(), 2025);
    }
}
