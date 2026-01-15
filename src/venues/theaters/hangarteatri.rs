use std::collections::HashSet;

use anyhow::Result;
use chrono::NaiveDate;
use convert_case::{Case, Casing};
use indicatif::{ProgressBar, ProgressFinish, ProgressIterator, ProgressStyle};
use reqwest::Client;
use scraper::{Html, Selector};

use crate::{
    dates::{DateRange, DateSet, TimeFrame, italian_month_to_number},
    events::Event,
    utils::PROGRESS_BAR_TEMPLATE,
};

pub async fn fetch(client: &Client, date_range: &DateRange) -> Result<Vec<Event>> {
    let mut events: HashSet<Event> = HashSet::new();

    let url = "https://www.hangarteatri.com/eventi/";
    let html_body = client.get(url).send().await?.text().await?;
    let document = Html::parse_document(&html_body);

    let shows_sel =
        Selector::parse("li.tribe-common-g-row.tribe-events-calendar-list__event-row").unwrap();
    let title_sel = Selector::parse("h4.tribe-events-calendar-list__event-title > a").unwrap();
    let date_sel =
        Selector::parse("time.tribe-events-calendar-list__event-datetime > span").unwrap();

    let show_count = document.select(&shows_sel).count();
    let progress = ProgressBar::new(show_count as u64)
        .with_style(ProgressStyle::with_template(PROGRESS_BAR_TEMPLATE).unwrap())
        .with_message("Fetching Hangar Teatri")
        .with_finish(ProgressFinish::AndLeave);

    for show in document.select(&shows_sel).progress_with(progress) {
        let title_el = show.select(&title_sel).next();
        if let None = title_el {
            continue;
        }
        let title = title_el
            .unwrap()
            .text()
            .next()
            .expect("Each event card should have text")
            .trim()
            .from_case(Case::Title)
            .to_case(Case::Title);

        let date_el = show.select(&date_sel).next();
        if let None = date_el {
            continue;
        }
        let date_str = date_el
            .unwrap()
            .text()
            .next()
            .expect("Each event date should have text")
            .to_string();

        // Parse the date from the datetime attribute
        let dates = parse_date(&date_str).expect("Date should be in a standardized format");

        // Skip events not in the current week
        if !dates.as_range().overlaps(&date_range) {
            continue;
        }

        let location = HashSet::from_iter(["Hangar Teatri".to_string()]);
        let time_frame = TimeFrame::Dates(dates);
        let event = Event::new(&title, location, "Teatri").with_time_frame(Some(time_frame));
        events.insert(event);
    }

    Ok(events.into_iter().collect())
}

/// Parse a date string from Hangar Teatri data and return a DateRange
///
/// This function handles these formats:
/// - Single dates with time: "9 Gennaio 2026 @ 20:30"
/// - Single dates with time ranges: "9 Gennaio 2026 @ 20:30 - 22:00"
fn parse_date(date_str: &str) -> Option<DateSet> {
    let trimmed = date_str.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Extract just the date part (before @)
    let date_part = trimmed.split('@').next().unwrap().trim();

    // Expected format: [day] [month] [year]
    // Indexes:         0     1       2
    let parts: Vec<&str> = date_part.split_whitespace().collect();
    if parts.len() != 3 {
        return None;
    }

    let day = parts[0].parse::<u32>().ok()?;
    let month = italian_month_to_number(parts[1])?;
    let year = parts[2].parse::<i32>().ok()?;

    let date = NaiveDate::from_ymd_opt(year, month, day)?;

    // For single dates, create a date range that spans one day
    return Some(DateSet::new(vec![date]).unwrap());
}

#[cfg(test)]
mod tests {
    use chrono::Datelike;

    use super::*;

    #[test]
    fn test_single_date() {
        let set = parse_date("9 Gennaio 2026 @ 20:30").unwrap();
        assert_eq!(set.first().day(), 9);
        assert_eq!(set.last().day(), 9); // Single date = same start and end
        assert_eq!(set.first().month(), 1);
        assert_eq!(set.first().year(), 2026);
    }

    #[test]
    fn test_single_date_without_time() {
        let range = parse_date("10 Gennaio 2026 @ 19:00").unwrap();
        assert_eq!(range.first().day(), 10);
        assert_eq!(range.last().day(), 10);
        assert_eq!(range.first().month(), 1);
        assert_eq!(range.first().year(), 2026);
    }
}
