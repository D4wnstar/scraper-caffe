use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// A set of dates, such as the days on which as event occurs.
/// Also usable to represent a span of time by adding the first and
/// last dates of the span. There must be at least one date.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateSet {
    dates: Vec<NaiveDate>,
}

impl DateSet {
    /// Create a new [DateSet].
    /// # Errors
    /// Returns `None` if the input vector is empty.
    pub fn new(dates: Vec<NaiveDate>) -> Option<Self> {
        if dates.is_empty() {
            None
        } else {
            Some(Self { dates })
        }
    }

    #[allow(unused)]
    pub fn dates(&self) -> &Vec<NaiveDate> {
        &self.dates
    }

    /// Returns the first date in chronological order.
    pub fn first(&self) -> NaiveDate {
        self.dates
            .iter()
            .reduce(|min, curr| min.min(curr))
            .unwrap()
            .clone()
    }

    /// Returns the last date in chronological order.
    pub fn last(&self) -> NaiveDate {
        self.dates
            .iter()
            .reduce(|max, curr| max.max(curr))
            .unwrap()
            .clone()
    }

    /// Returns a [DateRange] with the first and last dates of this set.
    pub fn as_range(&self) -> DateRange {
        DateRange {
            start: self.first(),
            end: self.last(),
        }
    }
}

/// A period of time represented by a start and an end date.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

impl DateRange {
    pub fn new(start: NaiveDate, end: NaiveDate) -> Self {
        Self { start, end }
    }

    /// Returns the distance in days between the first and last dates of the set.
    pub fn days_spanned(&self) -> i64 {
        (self.end - self.start).num_days()
    }

    /// Checks if this [DateRange] overlaps with another.
    pub fn overlaps(&self, other: &DateRange) -> bool {
        self.start <= other.end && self.end >= other.start
    }
}

/// A representation of a time frame, either as a discrete set of dates
/// (a [DateSet]) or a continuous period of time (a [DateRange]).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TimeFrame {
    Dates(DateSet),
    Period(DateRange),
}

impl TimeFrame {
    pub fn as_range(&self) -> DateRange {
        match self {
            TimeFrame::Dates(set) => set.as_range(),
            TimeFrame::Period(range) => range.clone(),
        }
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
