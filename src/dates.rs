use chrono::NaiveDate;
use std::fmt;

/// Represents a date range that can be used for filtering and comparisons
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateRange {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

impl fmt::Display for DateRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start_date != self.end_date {
            let start = self.start_date.format("%d %b %Y");
            let end = self.end_date.format("%d %b %Y");
            write!(f, "da {start} a {end}",)
        } else {
            let start = self.start_date.format("%d %b %Y");
            write!(f, "il {start}")
        }
    }
}

impl DateRange {
    /// Create a new DateRange
    pub fn new(start_date: NaiveDate, end_date: NaiveDate) -> Self {
        Self {
            start_date,
            end_date,
        }
    }

    /// Check if this date range overlaps with another date range
    pub fn overlaps(&self, other: &DateRange) -> bool {
        self.start_date <= other.end_date && self.end_date >= other.start_date
    }

    /// Check if a specific date is within this date range
    pub fn contains(&self, date: NaiveDate) -> bool {
        date >= self.start_date && date <= self.end_date
    }
}

/// Parse Italian month names to numbers
fn italian_month_to_number(month_name: &str) -> Option<u32> {
    match month_name {
        "Gen" => Some(1),
        "Feb" => Some(2),
        "Mar" => Some(3),
        "Apr" => Some(4),
        "Mag" => Some(5),
        "Giu" => Some(6),
        "Lug" => Some(7),
        "Ago" => Some(8),
        "Set" => Some(9),
        "Ott" => Some(10),
        "Nov" => Some(11),
        "Dic" => Some(12),
        _ => None,
    }
}

/// Parse a date string from Rossetti data and return a DateRange
///
/// This function handles various date formats found in the Rossetti data:
/// - Single dates: "22 Set 2025"
/// - Date ranges with same month: "23 - 24 Set 2025"
/// - Date ranges spanning months: "8 - 19 Ott 2025", "27/2 - 1/3 2026"
/// - Date ranges with different year formats: "30/12/2025 - 1/1/2026"
pub fn parse_rossetti_date(date_str: &str) -> Option<DateRange> {
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
    Some(DateRange::new(date, date))
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

    None
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

    Some(DateRange::new(start_date, end_date))
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

    Some(DateRange::new(start_date, end_date))
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

    Some(DateRange::new(start_date, end_date))
}

#[cfg(test)]
mod tests {
    use chrono::Datelike;

    use super::*;

    #[test]
    fn test_single_date() {
        let range = parse_rossetti_date("22 Set 2025").unwrap();
        assert_eq!(range.start_date.day(), 22);
        assert_eq!(range.end_date.day(), 22); // Single date = same start and end
        assert_eq!(range.start_date.month(), 9);
        assert_eq!(range.start_date.year(), 2025);
    }

    #[test]
    fn test_same_month_range() {
        let result = parse_rossetti_date("23 - 24 Set 2025").unwrap();
        assert_eq!(result.start_date.day(), 23);
        assert_eq!(result.start_date.month(), 9);
        assert_eq!(result.start_date.year(), 2025);
        assert_eq!(result.end_date.day(), 24);
        assert_eq!(result.end_date.month(), 9);
        assert_eq!(result.end_date.year(), 2025);
    }

    #[test]
    fn test_slash_date_range() {
        let result = parse_rossetti_date("27/2 - 1/3 2026").unwrap();
        assert_eq!(result.start_date.day(), 27);
        assert_eq!(result.start_date.month(), 2);
        assert_eq!(result.start_date.year(), 2026);
        assert_eq!(result.end_date.day(), 1);
        assert_eq!(result.end_date.month(), 3);
        assert_eq!(result.end_date.year(), 2026);
    }

    #[test]
    fn test_full_date_range() {
        let result = parse_rossetti_date("30/12/2025 - 1/1/2026").unwrap();
        assert_eq!(result.start_date.day(), 30);
        assert_eq!(result.start_date.month(), 12);
        assert_eq!(result.start_date.year(), 2025);
        assert_eq!(result.end_date.day(), 1);
        assert_eq!(result.end_date.month(), 1);
        assert_eq!(result.end_date.year(), 2026);
    }

    #[test]
    fn test_date_range_contains() {
        let range = parse_rossetti_date("23 - 24 Set 2025").unwrap();
        let test_date = NaiveDate::from_ymd_opt(2025, 9, 23).unwrap();
        assert!(range.contains(test_date));

        let test_date2 = NaiveDate::from_ymd_opt(2025, 9, 30).unwrap();
        assert!(!range.contains(test_date2));
    }

    #[test]
    fn test_date_range_overlaps() {
        let range1 = parse_rossetti_date("23 - 24 Set 2025").unwrap();
        let range2 = parse_rossetti_date("24 - 25 Set 2025").unwrap();
        let range3 = parse_rossetti_date("26 - 27 Set 2025").unwrap();

        assert!(range1.overlaps(&range2)); // Overlapping
        assert!(!range1.overlaps(&range3)); // Not overlapping
    }
}
