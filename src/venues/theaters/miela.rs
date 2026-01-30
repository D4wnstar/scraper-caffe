use std::{collections::HashSet, time::Duration};

use anyhow::Result;
use chrono::NaiveDate;
use convert_case::Case;
use indicatif::{ProgressBar, ProgressFinish, ProgressIterator, ProgressStyle};
use reqwest::Client;
use scraper::{Html, Selector};

use crate::{
    INFERENCE_SERVICE,
    dates::{DateRange, DateSet, TimeFrame},
    events::Event,
    utils::PROGRESS_BAR_TEMPLATE,
    venues::{StandardCasing, theaters::SUMMARY_PROMPT},
};

pub async fn fetch(client: &Client, date_range: &DateRange) -> Result<Vec<Event>> {
    let mut events: HashSet<Event> = HashSet::new();

    let url = "https://www.miela.it/calendario/";
    let html_body = client
        .get(url)
        .send()
        .await
        .inspect_err(|e| println!("GET request failed: {e}"))?
        .text()
        .await?;

    let document = Html::parse_document(&html_body);
    let shows_sel = Selector::parse("div.calendar-day").unwrap();
    let link_sel = Selector::parse("a.calendar-show").unwrap();
    let title_sel = Selector::parse("a.calendar-show > p > span.font-bold").unwrap();

    let show_count = document.select(&shows_sel).count();
    let progress = ProgressBar::new(show_count as u64)
        .with_style(ProgressStyle::with_template(PROGRESS_BAR_TEMPLATE).unwrap())
        .with_message("Fetching Miela")
        .with_finish(ProgressFinish::AndLeave);

    for show in document.select(&shows_sel).progress_with(progress) {
        let link_el = show.select(&link_sel).next();
        let title_el = show.select(&title_sel).next();
        if let None = link_el {
            // Since Miela uses a full calendar, many calendar boxes are empty
            continue;
        }

        let title = title_el
            .and_then(|el| el.text().next())
            .map(|t| t.trim().standardize_case(Some(Case::Upper)))
            .expect("Each event card should have text");

        let date_str = show
            .attr("data-calendar-day")
            .expect("Each calendar day should have a date");

        let dates = parse_date(&date_str).expect("Date should be in a standardized format");

        // Skip events not in the current week
        if !dates.as_range().overlaps(&date_range) {
            continue;
        }
        let time_frame = TimeFrame::Dates(dates);

        let location = HashSet::from_iter(["Miela".to_string()]);

        let (description, summary) =
            get_description(client, link_el.unwrap().attr("href").unwrap())
                .await
                .unwrap_or((None, None));

        let event = Event::new(&title, location, "Teatri")
            .with_time_frame(Some(time_frame))
            .with_description(description)
            .with_summary(summary);

        // Merge time frames if needed
        if let Some(mut ext_event) = events.take(&event) {
            let old_tf = ext_event.time_frame.unwrap();
            let new_tf = old_tf.merge(event.time_frame.unwrap());
            ext_event.time_frame = Some(new_tf);
            events.insert(ext_event);
        } else {
            events.insert(event);
        }

        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    Ok(events.into_iter().collect())
}

/// Parse a date string from Miela data and return a DateRange
///
/// This function handles the format: "20260109" (YYYYMMDD)
/// which is stored in the data-calendar-day attribute
fn parse_date(date_str: &str) -> Option<DateSet> {
    // The date_str is in format "20260109" (YYYYMMDD)
    // Extract year, month, and day
    if date_str.len() != 8 {
        return None;
    }

    let year_str = &date_str[0..4];
    let month_str = &date_str[4..6];
    let day_str = &date_str[6..8];

    let year = year_str.parse::<i32>().ok()?;
    let month = month_str.parse::<u32>().ok()?;
    let day = day_str.parse::<u32>().ok()?;

    let date = NaiveDate::from_ymd_opt(year, month, day)?;

    // For single dates, create a date range that spans one day
    return Some(DateSet::new(vec![date]).unwrap());
}

async fn get_description(client: &Client, url: &str) -> Result<(Option<String>, Option<String>)> {
    let desc_sel = Selector::parse("div.article__body.prose").unwrap();

    let html_body = client.get(url).send().await?.text().await?;
    let document = Html::parse_document(&html_body);
    let desc_el = document.select(&desc_sel).next();

    if let None = desc_el {
        println!("No desc_el");
        return Ok((None, None));
    }

    let description = desc_el
        .unwrap()
        .text()
        .fold(String::new(), |acc, t| {
            format!("{acc} {t}").trim().to_string()
        })
        .replace("\n", "");

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
    fn test_parse_miela_date() {
        let range = parse_date("20260109").unwrap();
        assert_eq!(range.first().day(), 9);
        assert_eq!(range.last().day(), 9);
        assert_eq!(range.first().month(), 1);
        assert_eq!(range.first().year(), 2026);
    }
}
