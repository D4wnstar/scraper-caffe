use chrono::NaiveDate;

use crate::dates::DateRange;

/// Parse a date string from Miela data and return a DateRange
///
/// This function handles the format: "20260109" (YYYYMMDD)
/// which is stored in the data-calendar-day attribute
pub fn parse_miela_date(date_str: &str) -> Option<DateRange> {
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
        let range = parse_miela_date("20260109").unwrap();
        assert_eq!(range.start_date.day(), 9);
        assert_eq!(range.end_date.day(), 9);
        assert_eq!(range.start_date.month(), 1);
        assert_eq!(range.start_date.year(), 2026);
    }

    #[test]
    fn test_parse_miela_date_with_leading_zero() {
        let range = parse_miela_date("20260101").unwrap();
        assert_eq!(range.start_date.day(), 1);
        assert_eq!(range.end_date.day(), 1);
        assert_eq!(range.start_date.month(), 1);
        assert_eq!(range.start_date.year(), 2026);
    }
}
