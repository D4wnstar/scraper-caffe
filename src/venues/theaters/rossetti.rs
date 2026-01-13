use std::collections::HashSet;

use anyhow::Result;
use chrono::NaiveDate;
use convert_case::{Case, Casing};
use reqwest::Client;
use scraper::{Html, Selector};

use crate::{
    dates::{DateRange, italian_month_to_number},
    events::Event,
};

pub async fn fetch(client: &Client, current_week: &DateRange) -> Result<Vec<Event>> {
    let mut events: HashSet<Event> = HashSet::new();

    let url = "https://www.ilrossetti.it/it/stagione/cartellone";
    let html_body = client.get(url).send().await?.text().await?;
    let document = Html::parse_document(&html_body);

    let shows_sel = Selector::parse("div.single-show:not(.single-show--disabled)").unwrap();
    let title_sel = Selector::parse("div.single-show__title > a").unwrap();
    let date_sel = Selector::parse("div.single-show__date").unwrap();
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
            .from_case(Case::Upper)
            .to_case(Case::Title);

        let date_el = show.select(&date_sel).next();
        if let None = date_el {
            continue;
        }
        let date_str = date_el
            .unwrap()
            .text()
            .skip(1) // First text elem is an empty string (due to the icon probably)
            .next()
            .expect("Second text element should always be the date")
            .trim()
            .to_string();
        let date_range = parse_date(&date_str).expect("Date should be in a standardized format");

        // Skip events not in the current week
        if !date_range.overlaps(&current_week) {
            continue;
        }

        let event = Event::new(
            &title,
            HashSet::from_iter(["Rossetti".to_string()]),
            "Teatri",
        )
        .date(Some(date_range));
        events.insert(event);
    }

    Ok(events.into_iter().collect())
}

/// Parse a date string from Rossetti data and return a DateRange
///
/// This function handles these formats:
/// - Single dates: "22 Set 2025"
/// - Date ranges with same month: "23 - 24 Set 2025"
/// - Date ranges spanning months: "8 - 19 Ott 2025", "27/2 - 1/3 2026"
/// - Date ranges with different year formats: "30/12/2025 - 1/1/2026"
fn parse_date(date_str: &str) -> Option<DateRange> {
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
fn parse_single_date(date_str: &str) -> Option<DateRange> {
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
    return Some(DateRange::new(date, date));
}

/// Parse a date range string
fn parse_date_range(date_str: &str) -> Option<DateRange> {
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
fn parse_same_month_range(date_str: &str) -> Option<DateRange> {
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

    return Some(DateRange::new(start_date, end_date));
}

/// Parse date range with slash format (e.g., "27/2 - 1/3 2026")
fn parse_slash_date_range(date_str: &str) -> Option<DateRange> {
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

    return Some(DateRange::new(start_date, end_date));
}

/// Parse date range with full date format (e.g., "30/12/2025 - 1/1/2026")
fn parse_full_date_range(date_str: &str) -> Option<DateRange> {
    let parts: Vec<&str> = date_str.split(" - ").collect();

    // Expected format: [start_day]/[start_month]/[start_year] - [end_day]/[end_month]/[end_year]
    // Indexes:         0                                        1
    if parts.len() != 2 {
        return None;
    }

    let start_date = NaiveDate::parse_from_str(parts[0], "%d/%m/%Y").ok()?;
    let end_date = NaiveDate::parse_from_str(parts[1], "%d/%m/%Y").ok()?;

    return Some(DateRange::new(start_date, end_date));
}

#[cfg(test)]
mod tests {
    use chrono::Datelike;

    use super::*;

    #[test]
    fn test_single_date() {
        let range = parse_date("22 Set 2025").unwrap();
        assert_eq!(range.start_date.day(), 22);
        assert_eq!(range.end_date.day(), 22); // Single date = same start and end
        assert_eq!(range.start_date.month(), 9);
        assert_eq!(range.start_date.year(), 2025);
    }

    #[test]
    fn test_same_month_range() {
        let result = parse_date("23 - 24 Set 2025").unwrap();
        assert_eq!(result.start_date.day(), 23);
        assert_eq!(result.start_date.month(), 9);
        assert_eq!(result.start_date.year(), 2025);
        assert_eq!(result.end_date.day(), 24);
        assert_eq!(result.end_date.month(), 9);
        assert_eq!(result.end_date.year(), 2025);
    }

    #[test]
    fn test_slash_date_range() {
        let result = parse_date("27/2 - 1/3 2026").unwrap();
        assert_eq!(result.start_date.day(), 27);
        assert_eq!(result.start_date.month(), 2);
        assert_eq!(result.start_date.year(), 2026);
        assert_eq!(result.end_date.day(), 1);
        assert_eq!(result.end_date.month(), 3);
        assert_eq!(result.end_date.year(), 2026);
    }

    #[test]
    fn test_full_date_range() {
        let result = parse_date("30/12/2025 - 1/1/2026").unwrap();
        assert_eq!(result.start_date.day(), 30);
        assert_eq!(result.start_date.month(), 12);
        assert_eq!(result.start_date.year(), 2025);
        assert_eq!(result.end_date.day(), 1);
        assert_eq!(result.end_date.month(), 1);
        assert_eq!(result.end_date.year(), 2026);
    }

    #[test]
    fn test_date_range_contains() {
        let range = parse_date("23 - 24 Set 2025").unwrap();
        let test_date = NaiveDate::from_ymd_opt(2025, 9, 23).unwrap();
        assert!(range.contains(test_date));

        let test_date2 = NaiveDate::from_ymd_opt(2025, 9, 30).unwrap();
        assert!(!range.contains(test_date2));
    }

    #[test]
    fn test_date_range_overlaps() {
        let range1 = parse_date("23 - 24 Set 2025").unwrap();
        let range2 = parse_date("24 - 25 Set 2025").unwrap();
        let range3 = parse_date("26 - 27 Set 2025").unwrap();

        assert!(range1.overlaps(&range2)); // Overlapping
        assert!(!range1.overlaps(&range3)); // Not overlapping
    }
}
