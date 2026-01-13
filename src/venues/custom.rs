use anyhow::Result;
use chrono::NaiveDate;
use std::{collections::HashSet, fs, path::Path};
use toml::Value;

use crate::{
    dates::DateRange,
    events::{Event, Locations},
};

pub fn fetch(filename: &str, current_week: &DateRange) -> Result<Vec<Event>> {
    let custom_events = load_custom_events(filename)?;

    // Filter custom events for current week
    let mut filtered: Vec<Event> = custom_events
        .into_iter()
        .filter(|e| {
            e.date
                .as_ref()
                .map(|d| d.overlaps(&current_week))
                .unwrap_or(false)
        })
        .collect();

    filtered.sort();

    return Ok(filtered);
}

/// Load custom events from a TOML file
fn load_custom_events(file_path: &str) -> Result<Vec<Event>> {
    // Check if file exists, if not return empty vec
    if !Path::new(file_path).exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(file_path)?;
    let value: Value = content.parse()?;

    let mut events = Vec::new();

    if let Some(events_array) = value.get("events").and_then(Value::as_array) {
        for event_table in events_array {
            if let Some(event) = parse_event_table(event_table)? {
                events.push(event);
            }
        }
    }

    Ok(events)
}

/// Parse a single event from a TOML table
fn parse_event_table(table: &Value) -> Result<Option<Event>> {
    let title = table
        .get("title")
        .and_then(Value::as_str)
        .map(|s| s.to_string());

    let category = table
        .get("category")
        .and_then(Value::as_str)
        .unwrap_or("Altro")
        .to_string();

    let locations = table
        .get("locations")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let date = table
        .get("date")
        .and_then(Value::as_str)
        .and_then(|date_str| parse_date(date_str));

    if let Some(title) = title {
        let event = Event::new(&title, Locations::from_locs(locations), &category).date(date);
        return Ok(Some(event));
    }

    Ok(None)
}

/// Parse a date string into a DateRange
/// Supports formats:
/// - Single date: "2026-12-24"
/// - Date range: "2026-12-24/2026-12-25"
fn parse_date(date_str: &str) -> Option<DateRange> {
    if let Some((start, end)) = date_str.split_once('/') {
        let start_date = NaiveDate::parse_from_str(start, "%Y-%m-%d").ok()?;
        let end_date = NaiveDate::parse_from_str(end, "%Y-%m-%d").ok()?;
        Some(DateRange::new(start_date, end_date))
    } else {
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()?;
        Some(DateRange::new(date, date))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_parse_single_date() {
        let date_range = parse_date("2026-12-24").unwrap();
        assert_eq!(date_range.start_date.day(), 24);
        assert_eq!(date_range.start_date.month(), 12);
        assert_eq!(date_range.start_date.year(), 2026);
        assert_eq!(date_range.end_date.day(), 24);
    }

    #[test]
    fn test_parse_date_range() {
        let date_range = parse_date("2026-12-24/2026-12-25").unwrap();
        assert_eq!(date_range.start_date.day(), 24);
        assert_eq!(date_range.end_date.day(), 25);
    }
}
