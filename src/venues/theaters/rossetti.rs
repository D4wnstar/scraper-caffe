use std::{collections::HashSet, time::Duration};

use anyhow::Result;
use chrono::{Datelike, NaiveDate};
use convert_case::Case;
use indicatif::{ProgressBar, ProgressFinish, ProgressIterator, ProgressStyle};
use reqwest::Client;
use scraper::{Html, Selector};

use crate::{
    INFERENCE_SERVICE,
    dates::{DateRange, DateSet, TimeFrame, italian_month_to_number},
    events::{Event, Location},
    inference::SUMMARY_PROMPT,
    utils::PROGRESS_BAR_TEMPLATE,
    venues::{CATEGORY_THEATRES, StandardCasing},
};

pub async fn fetch(client: &Client, date_range: &DateRange) -> Result<Vec<Event>> {
    let mut events: HashSet<Event> = HashSet::new();

    let url = "https://www.ilrossetti.it/it/stagione/cartellone";
    let html_body = client
        .get(url)
        .send()
        .await
        .inspect_err(|e| println!("GET request failed: {e}"))?
        .text()
        .await?;

    let document = Html::parse_document(&html_body);
    let shows_sel = Selector::parse("div.single-show:not(.single-show--disabled)").unwrap();
    let link_sel = Selector::parse("div.single-show__title > a").unwrap();
    let date_sel = Selector::parse("div.single-show__date").unwrap();

    let show_count = document.select(&shows_sel).count();
    let progress = ProgressBar::new(show_count as u64)
        .with_style(ProgressStyle::with_template(PROGRESS_BAR_TEMPLATE).unwrap())
        .with_message("Fetching Rossetti")
        .with_finish(ProgressFinish::AndLeave);

    for show in document.select(&shows_sel).progress_with(progress) {
        let link_el = show.select(&link_sel).next();
        let date_el = show.select(&date_sel).next();
        if link_el.is_none() || date_el.is_none() {
            continue;
        }

        // The date is selected just to check if the event is in the current week
        // The real dates in selected in the event's page later
        let date_str = date_el
            // First text elem is an empty string (due to the icon probably)
            .and_then(|el| el.text().skip(1).next())
            .map(|t| t.trim().to_string())
            .expect("Second text element should always be the date");
        let dates = parse_date(&date_str).expect("Date should be in a standardized format");
        if !dates.as_range().overlaps(&date_range) {
            continue;
        }

        let title = link_el
            .and_then(|el| el.text().next())
            .map(|t| t.trim().standardize_case(Some(Case::Upper)))
            .expect("Each event card should have text");

        let event_url = format!(
            "https://www.ilrossetti.it{}",
            link_el.unwrap().attr("href").unwrap()
        );
        let location = Location::new("Rossetti", Some(event_url.clone()));
        let locations = HashSet::from_iter([location]);

        let (description, summary, dates) = get_description_and_dates(client, &event_url)
            .await
            .unwrap_or((None, None, DateSet::today()));
        let time_frame = TimeFrame::Dates(dates);

        let event = Event::new(&title, locations, CATEGORY_THEATRES)
            .with_time_frame(Some(time_frame))
            .with_description(description)
            .with_summary(summary);

        events.insert(event);

        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    Ok(events.into_iter().collect())
}

/// Parse a date string from the Rossetti calendar and return a [DateSet]
///
/// This function handles these formats:
/// - Single dates: "22 Set 2025"
/// - Date ranges with same month: "23 - 24 Set 2025"
/// - Date ranges spanning months: "8 - 19 Ott 2025", "27/2 - 1/3 2026"
/// - Date ranges with different year formats: "30/12/2025 - 1/1/2026"
fn parse_date(date_str: &str) -> Option<DateSet> {
    let trimmed = date_str.trim();
    if trimmed.is_empty() {
        return None;
    }

    if !trimmed.contains('-') {
        // Case 1: Single date format (e.g., "22 Set 2025")
        return parse_single_date(trimmed);
    } else {
        // Case 2: Date range format (e.g., "23 - 24 Set 2025")
        return parse_date_range(trimmed);
    }
}

/// Parse a single date string (e.g., "22 Set 2025")
fn parse_single_date(date_str: &str) -> Option<DateSet> {
    let parts: Vec<&str> = date_str.split_whitespace().collect();

    // Expected format: [day] [month] [year]
    // Indexes:         0     1       2
    if parts.len() != 3 {
        return None;
    }

    let month = italian_month_to_number(parts[1])?;
    let date_str = format!("{}/{}/{}", parts[0], month, parts[2]); // e.g. 22/9/2025
    let date = NaiveDate::parse_from_str(&date_str, "%d/%m/%Y").ok()?;

    // For single dates, create a date range that spans one day
    return Some(DateSet::new(vec![date]).unwrap());
}

/// Parse a date range string
fn parse_date_range(date_str: &str) -> Option<DateSet> {
    // Handle different date range formats

    // Format 1: "23 - 24 Set 2025" (same month)
    if date_str.contains(" - ") && !date_str.contains('/') {
        return parse_same_month_range(date_str);
    }

    // Format 2: "27/2 - 1/3 2026" (different month same year; day/month format)
    let slashes = date_str.chars().filter(|&c| c == '/').count();
    if date_str.contains('/') && slashes == 2 {
        return parse_slash_date_range(date_str);
    }

    // Format 3: "30/12/2025 - 1/1/2026" (different year; full date format)
    if date_str.contains('/') && slashes == 4 {
        return parse_full_date_range(date_str);
    }

    return None;
}

/// Parse date range with same month (e.g., "23 - 24 Set 2025")
fn parse_same_month_range(date_str: &str) -> Option<DateSet> {
    let parts: Vec<&str> = date_str.split_whitespace().collect();

    // Expected format: [start_day] - [end_day] [month] [year]
    // Indexes:         0           1 2         3       4
    if parts.len() != 5 {
        return None;
    }

    let month = italian_month_to_number(parts[3])?;
    let start_str = format!("{}/{}/{}", parts[0], month, parts[4]); // e.g. 23/9/2025
    let start_date = NaiveDate::parse_from_str(&start_str, "%d/%m/%Y").ok()?;
    let end_str = format!("{}/{}/{}", parts[2], month, parts[4]); // e.g. 24/9/2025
    let end_date = NaiveDate::parse_from_str(&end_str, "%d/%m/%Y").ok()?;

    return Some(DateSet::new(vec![start_date, end_date]).unwrap());
}

/// Parse date range with slash format (e.g., "27/2 - 1/3 2026")
fn parse_slash_date_range(date_str: &str) -> Option<DateSet> {
    let parts: Vec<&str> = date_str.split_whitespace().collect();

    // Expected format: [start_day]/[start_month] - [end_day]/[end_month] [year]
    // Indexes:         0                         1 2                     3
    if parts.len() != 4 {
        return None;
    }

    let start_str = format!("{}/{}", parts[0], parts[3]); // e.g. 27/2/2026
    let start_date = NaiveDate::parse_from_str(&start_str, "%d/%m/%Y").ok()?;
    let end_str = format!("{}/{}", parts[2], parts[3]); // e.g. 1/3/2026
    let end_date = NaiveDate::parse_from_str(&end_str, "%d/%m/%Y").ok()?;

    return Some(DateSet::new(vec![start_date, end_date]).unwrap());
}

/// Parse date range with full date format (e.g., "30/12/2025 - 1/1/2026")
fn parse_full_date_range(date_str: &str) -> Option<DateSet> {
    let parts: Vec<&str> = date_str.split(" - ").collect();

    // Expected format: [start_day]/[start_month]/[start_year] - [end_day]/[end_month]/[end_year]
    // Indexes:         0                                        1
    if parts.len() != 2 {
        return None;
    }

    let start_date = NaiveDate::parse_from_str(parts[0], "%d/%m/%Y").ok()?;
    let end_date = NaiveDate::parse_from_str(parts[1], "%d/%m/%Y").ok()?;

    return Some(DateSet::new(vec![start_date, end_date]).unwrap());
}

async fn get_description_and_dates(
    client: &Client,
    url: &str,
) -> Result<(Option<String>, Option<String>, DateSet)> {
    let desc_paras_sel = Selector::parse("div.section div.u-unknown-content p").unwrap();
    let dates_sel = Selector::parse("div.recite__date").unwrap();

    let html_body = client.get(url).send().await?.text().await?;
    let document = Html::parse_document(&html_body);
    let desc_el = document.select(&desc_paras_sel);
    let date_els = document.select(&dates_sel);

    let description;
    let summary;
    if desc_el.clone().count() == 0 {
        eprintln!("No desc_el in {url}");
        description = None;
        summary = None;
    } else {
        let desc = desc_el
            .filter_map(|el| {
                if el.child_elements().count() > 0 {
                    None
                } else {
                    Some(el.text().fold(String::new(), |acc, t| format!("{acc} {t}")))
                }
            })
            .fold(String::new(), |acc, t| format!("{acc} {t}"))
            .trim()
            .to_string();

        let prompt = format!("{}\n\n{}", SUMMARY_PROMPT, desc);

        description = Some(desc);
        summary = INFERENCE_SERVICE
            .infer(&prompt)
            .await
            .inspect_err(|err| eprintln!("Failed to generate summary in {url}: {err}"))
            .ok();
    }

    let dates;
    if date_els.clone().count() == 0 {
        eprintln!("No dates found in {url}");
        dates = DateSet::today();
    } else {
        let naive_dates: Vec<NaiveDate> = date_els
            .filter_map(|el| el.text().next())
            .map(|t| {
                let split: Vec<&str> = t.split_whitespace().collect();
                let day: u32 = split[1].parse().unwrap();
                let month = italian_month_to_number(split[2]).unwrap();
                let year = chrono::Local::now().year();
                NaiveDate::from_ymd_opt(year, month, day).unwrap()
            })
            .collect();
        dates = DateSet::new(naive_dates).unwrap();
    }

    return Ok((description, summary, dates));
}

#[cfg(test)]
mod tests {
    use chrono::Datelike;

    use super::*;

    #[test]
    fn test_single_date() {
        let set = parse_date("22 Set 2025").unwrap();
        assert_eq!(set.first().day(), 22);
        assert_eq!(set.last().day(), 22); // Single date = same start and end
        assert_eq!(set.first().month(), 9);
        assert_eq!(set.first().year(), 2025);
    }

    #[test]
    fn test_same_month_set() {
        let result = parse_date("23 - 24 Set 2025").unwrap();
        assert_eq!(result.first().day(), 23);
        assert_eq!(result.first().month(), 9);
        assert_eq!(result.first().year(), 2025);
        assert_eq!(result.last().day(), 24);
        assert_eq!(result.last().month(), 9);
        assert_eq!(result.last().year(), 2025);
    }

    #[test]
    fn test_slash_date_set() {
        let result = parse_date("27/2 - 1/3 2026").unwrap();
        assert_eq!(result.first().day(), 27);
        assert_eq!(result.first().month(), 2);
        assert_eq!(result.first().year(), 2026);
        assert_eq!(result.last().day(), 1);
        assert_eq!(result.last().month(), 3);
        assert_eq!(result.last().year(), 2026);
    }

    #[test]
    fn test_full_date_set() {
        let result = parse_date("30/12/2025 - 1/1/2026").unwrap();
        assert_eq!(result.first().day(), 30);
        assert_eq!(result.first().month(), 12);
        assert_eq!(result.first().year(), 2025);
        assert_eq!(result.last().day(), 1);
        assert_eq!(result.last().month(), 1);
        assert_eq!(result.last().year(), 2026);
    }
}
