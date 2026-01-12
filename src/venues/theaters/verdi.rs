use std::collections::HashSet;

use anyhow::Result;
use chrono::NaiveDate;
use reqwest::Client;
use scraper::{Html, Selector};

use crate::{
    dates::{DateRange, italian_month_to_number},
    events::{Event, Locations},
};

pub async fn fetch(client: &Client, current_week: &DateRange) -> Result<Vec<Event>> {
    let mut events: HashSet<Event> = HashSet::new();

    let url = "https://www.teatroverdi-trieste.com/it/calendario-spettacoli/";
    let html_body = client.get(url).send().await?.text().await?;
    let document = Html::parse_document(&html_body);

    let shows_sel = Selector::parse("ul.spettacolo-list div.list-text").unwrap();
    let title_sel = Selector::parse("h2.spettacolo-list-title > a").unwrap();
    let date_sel = Selector::parse("span.spettacolo-list-date > strong").unwrap();
    for show in document.select(&shows_sel) {
        let title_el = show.select(&title_sel).next();
        if let None = title_el {
            continue;
        }
        let title = title_el.unwrap().text().next().unwrap().to_string();

        let date_el = show.select(&date_sel).next();
        if let None = date_el {
            continue;
        }
        let date_str = date_el.unwrap().text().next().unwrap();
        let date_range = parse_date(date_str).expect("Date should be in a standardized format");

        // Skip events not in the current week
        if !date_range.overlaps(&current_week) {
            continue;
        }

        let event = Event {
            title,
            date: Some(date_range),
            locations: Locations::from_loc("Verdi".to_string()),
            category: "Teatri".to_string(),
        };
        events.insert(event);
    }

    return Ok(events.into_iter().collect());
}

/// Parse a date string from Verdi data and return a DateRange
///
/// This function handles these formats:
/// - Single dates: "Martedì 23 dicembre 2025 ore 19.30"
/// - Multiple dates: "28, 30 novembre, 5, 7, 11, 13 dicembre 2025"
fn parse_date(date_str: &str) -> Option<DateRange> {
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
fn parse_single_date(date_str: &str) -> Option<DateRange> {
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
    return Some(DateRange::new(date, date));
}

/// Parse multiple dates string (e.g., "28, 30 novembre, 5, 7, 11, 13 dicembre 2025")
fn parse_multiple_dates(date_str: &str) -> Option<DateRange> {
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
    }

    // Sort dates to find the earliest and latest
    dates.sort();
    let start_date = dates[0];
    let end_date = dates[dates.len() - 1];

    return Some(DateRange::new(start_date, end_date));
}

#[cfg(test)]
mod tests {
    use chrono::Datelike;

    use super::*;

    #[test]
    fn test_single_date() {
        let range = parse_date("Martedì 23 dicembre 2025 ore 19.30").unwrap();
        assert_eq!(range.start_date.day(), 23);
        assert_eq!(range.end_date.day(), 23); // Single date = same start and end
        assert_eq!(range.start_date.month(), 12);
        assert_eq!(range.start_date.year(), 2025);
    }

    #[test]
    fn test_single_date_without_time() {
        let range = parse_date("Mercoledì 31 dicembre 2025").unwrap();
        assert_eq!(range.start_date.day(), 31);
        assert_eq!(range.end_date.day(), 31);
        assert_eq!(range.start_date.month(), 12);
        assert_eq!(range.start_date.year(), 2025);
    }

    #[test]
    fn test_multiple_dates_same_month() {
        let result = parse_date("9, 10, 11, 13 gennaio 2026").unwrap();
        assert_eq!(result.start_date.day(), 9);
        assert_eq!(result.start_date.month(), 1);
        assert_eq!(result.start_date.year(), 2026);
        assert_eq!(result.end_date.day(), 13);
        assert_eq!(result.end_date.month(), 1);
        assert_eq!(result.end_date.year(), 2026);
    }

    #[test]
    fn test_multiple_dates_diff_month() {
        let result = parse_date("28, 30 novembre, 5, 7, 11, 13 dicembre 2025").unwrap();
        assert_eq!(result.start_date.day(), 28);
        assert_eq!(result.start_date.month(), 11);
        assert_eq!(result.start_date.year(), 2025);
        assert_eq!(result.end_date.day(), 13);
        assert_eq!(result.end_date.month(), 12);
        assert_eq!(result.end_date.year(), 2025);
    }

    #[test]
    fn test_date_range_contains() {
        let range = parse_date("28, 30 novembre, 5, 7, 11, 13 dicembre 2025").unwrap();
        let test_date = NaiveDate::from_ymd_opt(2025, 11, 28).unwrap();
        assert!(range.contains(test_date));

        let test_date2 = NaiveDate::from_ymd_opt(2025, 12, 25).unwrap();
        assert!(!range.contains(test_date2));
    }

    #[test]
    fn test_date_range_overlaps() {
        let range1 = parse_date("28, 30 novembre, 5, 7, 11, 13 dicembre 2025").unwrap();
        let range2 = parse_date("9, 10, 11, 13 gennaio 2026").unwrap();
        let range3 = parse_date("19, 20, 21, 26, 27, 28 giugno 2026").unwrap();

        assert!(!range1.overlaps(&range2)); // Not overlapping
        assert!(!range1.overlaps(&range3)); // Not overlapping
    }
}
