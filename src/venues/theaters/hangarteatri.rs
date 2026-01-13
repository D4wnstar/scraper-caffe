use std::collections::HashSet;

use anyhow::Result;
use chrono::NaiveDate;
use convert_case::{Case, Casing};
use reqwest::Client;
use scraper::{Html, Selector};

use crate::{
    dates::{DateRange, italian_month_to_number},
    events::{Event, Locations},
};

pub async fn fetch(client: &Client, current_week: &DateRange) -> Result<Vec<Event>> {
    let mut events: HashSet<Event> = HashSet::new();

    let url = "https://www.hangarteatri.com/eventi/";
    let html_body = client.get(url).send().await?.text().await?;
    let document = Html::parse_document(&html_body);

    let shows_sel =
        Selector::parse("li.tribe-common-g-row.tribe-events-calendar-list__event-row").unwrap();
    let title_sel = Selector::parse("h4.tribe-events-calendar-list__event-title > a").unwrap();
    let date_sel =
        Selector::parse("time.tribe-events-calendar-list__event-datetime > span").unwrap();

    for show in document.select(&shows_sel) {
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
        let date_range = parse_date(&date_str).expect("Date should be in a standardized format");

        // Skip events not in the current week
        if !date_range.overlaps(&current_week) {
            continue;
        }

        let event = Event::new(
            &title,
            Locations::from_loc("Hangar Teatri".to_string()),
            "Teatri",
        )
        .date(Some(date_range));
        events.insert(event);
    }

    Ok(events.into_iter().collect())
}

/// Parse a date string from Hangar Teatri data and return a DateRange
///
/// This function handles these formats:
/// - Single dates with time: "9 Gennaio 2026 @ 20:30"
/// - Date ranges with time: "9 Gennaio 2026 @ 20:30 - 22:00"
/// - Single dates without time: "10 Gennaio 2026 @ 19:00"
fn parse_date(date_str: &str) -> Option<DateRange> {
    let trimmed = date_str.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Extract just the date part (before @)
    let date_part = trimmed.split('@').next().unwrap().trim();

    // Check if it's a single date or a range
    if date_part.contains('-') {
        return parse_date_range(date_part);
    } else {
        return parse_single_date(date_part);
    }
}

/// Parse a single date string (e.g., "9 Gennaio 2026")
fn parse_single_date(date_str: &str) -> Option<DateRange> {
    let parts: Vec<&str> = date_str.split_whitespace().collect();

    // Expected format: [day] [month] [year]
    // Indexes:         0     1       2
    if parts.len() != 3 {
        return None;
    }

    let day = parts[0].parse::<u32>().ok()?;
    let month = italian_month_to_number(parts[1])?;
    let year = parts[2].parse::<i32>().ok()?;

    let date = NaiveDate::from_ymd_opt(year, month, day)?;

    // For single dates, create a date range that spans one day
    return Some(DateRange::new(date, date));
}

/// Parse a date range string (e.g., "9 Gennaio 2026 - 10 Gennaio 2026")
fn parse_date_range(date_str: &str) -> Option<DateRange> {
    let parts: Vec<&str> = date_str.split('-').collect();

    // Expected format: [start_date] - [end_date]
    // Indexes:         0           1
    if parts.len() != 2 {
        return None;
    }

    let start_date = parse_single_date(parts[0].trim())?;
    let end_date = parse_single_date(parts[1].trim())?;

    return Some(DateRange::new(start_date.start_date, end_date.end_date));
}

#[cfg(test)]
mod tests {
    use chrono::Datelike;

    use super::*;

    #[test]
    fn test_single_date() {
        let range = parse_date("9 Gennaio 2026 @ 20:30").unwrap();
        assert_eq!(range.start_date.day(), 9);
        assert_eq!(range.end_date.day(), 9); // Single date = same start and end
        assert_eq!(range.start_date.month(), 1);
        assert_eq!(range.start_date.year(), 2026);
    }

    #[test]
    fn test_single_date_without_time() {
        let range = parse_date("10 Gennaio 2026 @ 19:00").unwrap();
        assert_eq!(range.start_date.day(), 10);
        assert_eq!(range.end_date.day(), 10);
        assert_eq!(range.start_date.month(), 1);
        assert_eq!(range.start_date.year(), 2026);
    }

    #[test]
    fn test_date_range() {
        let result = parse_date("9 Gennaio 2026 @ 20:30 - 22:00").unwrap();
        assert_eq!(result.start_date.day(), 9);
        assert_eq!(result.start_date.month(), 1);
        assert_eq!(result.start_date.year(), 2026);
        assert_eq!(result.end_date.day(), 9);
        assert_eq!(result.end_date.month(), 1);
        assert_eq!(result.end_date.year(), 2026);
    }

    #[test]
    fn test_date_range_contains() {
        let range = parse_date("9 Gennaio 2026 @ 20:30 - 22:00").unwrap();
        let test_date = NaiveDate::from_ymd_opt(2026, 1, 9).unwrap();
        assert!(range.contains(test_date));

        let test_date2 = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        assert!(!range.contains(test_date2));
    }
}
