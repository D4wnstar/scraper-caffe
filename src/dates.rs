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

    #[allow(unused)]
    /// Check if a specific date is within this date range
    pub fn contains(&self, date: NaiveDate) -> bool {
        date >= self.start_date && date <= self.end_date
    }
}

/// Parse Italian month names to numbers
pub fn italian_month_to_number(month_name: &str) -> Option<u32> {
    match month_name.to_lowercase().as_str() {
        "gen" => Some(1),
        "gennaio" => Some(1),
        "feb" => Some(2),
        "febbraio" => Some(2),
        "mar" => Some(3),
        "marzo" => Some(3),
        "apr" => Some(4),
        "aprile" => Some(4),
        "mag" => Some(5),
        "maggio" => Some(5),
        "giu" => Some(6),
        "giugno" => Some(6),
        "lug" => Some(7),
        "luglio" => Some(7),
        "ago" => Some(8),
        "agosto" => Some(8),
        "set" => Some(9),
        "settembre" => Some(9),
        "ott" => Some(10),
        "ottobre" => Some(10),
        "nov" => Some(11),
        "novembre" => Some(11),
        "dic" => Some(12),
        "dicembre" => Some(12),
        _ => None,
    }
}
