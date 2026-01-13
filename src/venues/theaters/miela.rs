use std::collections::HashSet;

use anyhow::Result;
use chrono::NaiveDate;
use convert_case::{Case, Casing};
use reqwest::Client;
use scraper::{Html, Selector};

use crate::{
    dates::DateRange,
    events::{Event, Locations},
};

pub async fn fetch(client: &Client, current_week: &DateRange) -> Result<Vec<Event>> {
    let mut events: HashSet<Event> = HashSet::new();

    let url = "https://www.miela.it/calendario/";
    let html_body = client.get(url).send().await?.text().await?;
    let document = Html::parse_document(&html_body);

    let shows_sel = Selector::parse("div.calendar-day").unwrap();
    let title_sel = Selector::parse("a.calendar-show > p").unwrap();

    for show in document.select(&shows_sel) {
        let title_el = show.select(&title_sel).next();
        if let None = title_el {
            // Since Miela uses a full calendar, many calendar boxes are empty
            continue;
        }
        let title = title_el
            .unwrap()
            .text()
            .next()
            .expect("Each event card should have text")
            .trim()
            .from_case(Case::Upper)
            .to_case(Case::Title);

        let date_str = show
            .attr("data-calendar-day")
            .expect("Each calendar day should have a date");
        let date_range = parse_date(&date_str).expect("Date should be in a standardized format");

        // Skip events not in the current week
        if !date_range.overlaps(&current_week) {
            continue;
        }

        let event = Event::new(&title, Locations::from_loc("Miela".to_string()), "Teatri")
            .date(Some(date_range));
        events.insert(event);
    }

    Ok(events.into_iter().collect())
}

/// Parse a date string from Miela data and return a DateRange
///
/// This function handles the format: "20260109" (YYYYMMDD)
/// which is stored in the data-calendar-day attribute
fn parse_date(date_str: &str) -> Option<DateRange> {
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
    return Some(DateRange::new(date, date));
}

#[cfg(test)]
mod tests {
    use chrono::Datelike;

    use super::*;

    #[test]
    fn test_parse_miela_date() {
        let range = parse_date("20260109").unwrap();
        assert_eq!(range.start_date.day(), 9);
        assert_eq!(range.end_date.day(), 9);
        assert_eq!(range.start_date.month(), 1);
        assert_eq!(range.start_date.year(), 2026);
    }

    #[test]
    fn test_parse_miela_date_with_leading_zero() {
        let range = parse_date("20260101").unwrap();
        assert_eq!(range.start_date.day(), 1);
        assert_eq!(range.end_date.day(), 1);
        assert_eq!(range.start_date.month(), 1);
        assert_eq!(range.start_date.year(), 2026);
    }
}
