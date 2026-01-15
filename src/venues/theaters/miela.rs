use std::collections::HashSet;

use anyhow::Result;
use chrono::NaiveDate;
use convert_case::{Case, Casing};
use indicatif::{ProgressBar, ProgressFinish, ProgressIterator, ProgressStyle};
use reqwest::Client;
use scraper::{Html, Selector};

use crate::{
    dates::{DateRange, DateSet, TimeFrame},
    events::Event,
    utils::PROGRESS_BAR_TEMPLATE,
};

pub async fn fetch(client: &Client, date_range: &DateRange) -> Result<Vec<Event>> {
    let mut events: HashSet<Event> = HashSet::new();

    let url = "https://www.miela.it/calendario/";
    let html_body = client.get(url).send().await?.text().await?;
    let document = Html::parse_document(&html_body);

    let shows_sel = Selector::parse("div.calendar-day").unwrap();
    let title_sel = Selector::parse("a.calendar-show > p").unwrap();

    let show_count = document.select(&shows_sel).count();
    let progress = ProgressBar::new(show_count as u64)
        .with_style(ProgressStyle::with_template(PROGRESS_BAR_TEMPLATE).unwrap())
        .with_message("Fetching Miela")
        .with_finish(ProgressFinish::AndLeave);

    for show in document.select(&shows_sel).progress_with(progress) {
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
        let dates = parse_date(&date_str).expect("Date should be in a standardized format");

        // Skip events not in the current week
        if !dates.as_range().overlaps(&date_range) {
            continue;
        }

        let location = HashSet::from_iter(["Miela".to_string()]);
        let time_frame = TimeFrame::Dates(dates);
        let event = Event::new(&title, location, "Teatri").with_time_frame(Some(time_frame));
        events.insert(event);
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
