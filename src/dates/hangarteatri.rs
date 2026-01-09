use chrono::NaiveDate;

use crate::dates::{DateRange, italian_month_to_number};

/// Parse a date string from Hangar Teatri data and return a DateRange
///
/// This function handles these formats:
/// - Single dates with time: "9 Gennaio 2026 @ 20:30"
/// - Date ranges with time: "9 Gennaio 2026 @ 20:30 - 22:00"
/// - Single dates without time: "10 Gennaio 2026 @ 19:00"
pub fn parse_hangarteatri_date(date_str: &str) -> Option<DateRange> {
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
        let range = parse_hangarteatri_date("9 Gennaio 2026 @ 20:30").unwrap();
        assert_eq!(range.start_date.day(), 9);
        assert_eq!(range.end_date.day(), 9); // Single date = same start and end
        assert_eq!(range.start_date.month(), 1);
        assert_eq!(range.start_date.year(), 2026);
    }

    #[test]
    fn test_single_date_without_time() {
        let range = parse_hangarteatri_date("10 Gennaio 2026 @ 19:00").unwrap();
        assert_eq!(range.start_date.day(), 10);
        assert_eq!(range.end_date.day(), 10);
        assert_eq!(range.start_date.month(), 1);
        assert_eq!(range.start_date.year(), 2026);
    }

    #[test]
    fn test_date_range() {
        let result = parse_hangarteatri_date("9 Gennaio 2026 @ 20:30 - 22:00").unwrap();
        assert_eq!(result.start_date.day(), 9);
        assert_eq!(result.start_date.month(), 1);
        assert_eq!(result.start_date.year(), 2026);
        assert_eq!(result.end_date.day(), 9);
        assert_eq!(result.end_date.month(), 1);
        assert_eq!(result.end_date.year(), 2026);
    }

    #[test]
    fn test_date_range_contains() {
        let range = parse_hangarteatri_date("9 Gennaio 2026 @ 20:30 - 22:00").unwrap();
        let test_date = NaiveDate::from_ymd_opt(2026, 1, 9).unwrap();
        assert!(range.contains(test_date));

        let test_date2 = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        assert!(!range.contains(test_date2));
    }
}
