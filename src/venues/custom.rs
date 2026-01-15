use anyhow::Result;
use chrono::NaiveDate;
use std::{collections::HashSet, fs, path::Path};
use toml::{Table, Value};

use crate::{
    dates::{DateRange, DateSet, TimeFrame},
    events::Event,
};

pub fn fetch(filename: &str, date_range: &DateRange) -> Result<Vec<Event>> {
    let custom_events = load_custom_events(filename)?;

    // Filter custom events for current week
    let mut filtered: Vec<Event> = custom_events
        .into_iter()
        .filter(|e| {
            e.time_frame
                .as_ref()
                .map(|d| d.as_range().overlaps(&date_range))
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
    let table: Table = content.parse()?;

    let mut events = Vec::new();

    if let Some(events_array) = table.get("events").and_then(Value::as_array) {
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

    let time_frame = table.get("date").and_then(|date| parse_date(date));

    if let Some(title) = title {
        let locs = HashSet::from_iter(locations);
        let event = Event::new(&title, locs, &category).with_time_frame(time_frame);
        return Ok(Some(event));
    }

    Ok(None)
}

/// Parse a date string into a DateRange
/// Supports formats:
/// - Single date: `"24-07-2026"`
/// - Multiple dates: `["11-02-2026", "13-02-2026", "14-02-2026"]`
/// - Date range: `"12-07-2026/29-12-2026"`
fn parse_date(value: &Value) -> Option<TimeFrame> {
    if let Some(vec) = value.as_array() {
        // This is a multiple date
        let mut dates: Vec<NaiveDate> = vec![];
        for val in vec {
            if let Some(date_str) = val.as_str() {
                if let Ok(date) = NaiveDate::parse_from_str(date_str, "%d-%m-%Y") {
                    dates.push(date);
                }
            }
        }
        let date_set = DateSet::new(dates)?;
        return Some(TimeFrame::Dates(date_set));
    }

    if let Some(date_str) = value.as_str() {
        if date_str.contains('/') {
            // This is a date range
            if let Some((start, end)) = date_str.split_once('/') {
                let start_date = NaiveDate::parse_from_str(start, "%Y-%m-%d").ok()?;
                let end_date = NaiveDate::parse_from_str(end, "%Y-%m-%d").ok()?;
                let date_range = DateRange::new(start_date, end_date);
                return Some(TimeFrame::Period(date_range));
            }
        } else {
            // This is a single date
            let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()?;
            let date_set = DateSet::new(vec![date]).unwrap();
            return Some(TimeFrame::Dates(date_set));
        }
    }

    return None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_parse_single_date() {
        let time_frame = parse_date(&Value::String("24-07-2026".to_string())).unwrap();
        assert!(matches!(time_frame, TimeFrame::Dates(_)));
        if let TimeFrame::Dates(date_set) = time_frame {
            assert_eq!(date_set.first().day(), 24);
            assert_eq!(date_set.first().month(), 7);
            assert_eq!(date_set.first().year(), 2026);
            assert_eq!(date_set.last().day(), 24);
        }
    }

    #[test]
    fn test_parse_multiple_date() {
        let arr = toml::value::Array::from_iter(
            ["11-02-2026", "13-02-2026", "14-02-2026"]
                .iter()
                .map(|s| Value::String(s.to_string())),
        );
        let time_frame = parse_date(&Value::Array(arr)).unwrap();
        assert!(matches!(time_frame, TimeFrame::Dates(_)));
        if let TimeFrame::Dates(date_set) = time_frame {
            assert_eq!(date_set.dates().len(), 3);
            assert_eq!(date_set.first().day(), 11);
            assert_eq!(date_set.first().month(), 2);
            assert_eq!(date_set.first().year(), 2026);
            assert_eq!(date_set.last().day(), 14);
            assert_eq!(date_set.first().month(), 2);
            assert_eq!(date_set.first().year(), 2026);
        }
    }

    #[test]
    fn test_parse_date_range() {
        let time_frame = parse_date(&Value::String("12-07-2026/29-12-2026".to_string())).unwrap();
        assert!(matches!(time_frame, TimeFrame::Period(_)));
        if let TimeFrame::Period(date_range) = time_frame {
            assert_eq!(date_range.start.day(), 12);
            assert_eq!(date_range.start.month(), 7);
            assert_eq!(date_range.start.year(), 2026);
            assert_eq!(date_range.end.day(), 29);
            assert_eq!(date_range.end.month(), 12);
            assert_eq!(date_range.end.year(), 2026);
        }
    }
}
