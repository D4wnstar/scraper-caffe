use std::{collections::HashSet, iter::Take};

use chrono::{NaiveDate, naive::NaiveDateDaysIterator};
use serde::{Deserialize, Serialize};

/// A set of dates, such as the days on which as event occurs.
/// Also usable to represent a span of time by adding the first and
/// last dates of the span. There must be at least one date.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateSet {
    dates: Vec<NaiveDate>,
}

impl DateSet {
    /// Creates a new [DateSet].
    /// # Errors
    /// Returns `None` if the input vector is empty.
    pub fn new(dates: Vec<NaiveDate>) -> Option<Self> {
        if dates.is_empty() {
            None
        } else {
            Some(Self { dates })
        }
    }

    /// Creates a new [DateSet] including only today's date.
    pub fn today() -> Self {
        Self {
            dates: vec![chrono::Local::now().date_naive()],
        }
    }

    /// Returns a reference to the dates in this [DateSet]. Not guaranteed
    /// to be sorted.
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

    pub fn merge(self, other: Self) -> Self {
        // Use a temporary hashset to deduplicate
        let mut new_set = HashSet::new();
        new_set.extend(self.dates);
        new_set.extend(other.dates);
        DateSet::new(Vec::from_iter(new_set)).unwrap()
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
        (self.end - self.start).num_days() + 1
    }

    /// Checks if this [DateRange] overlaps with another.
    pub fn overlaps(&self, other: &DateRange) -> bool {
        self.start <= other.end && self.end >= other.start
    }

    pub fn merge(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    pub fn iter_days(&self) -> Take<NaiveDateDaysIterator> {
        self.start.iter_days().take(self.days_spanned() as usize)
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
            Self::Dates(set) => set.as_range(),
            Self::Period(range) => range.clone(),
        }
    }

    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Dates(set1), Self::Dates(set2)) => Self::Dates(set1.merge(set2)),
            (Self::Period(range1), Self::Period(range2)) => Self::Period(range1.merge(range2)),
            _ => todo!(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_days_spanned_is_end_inclusive() {
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        let range = DateRange::new(start, end);

        assert_eq!(range.days_spanned(), 5);
    }

    #[test]
    fn test_iter_days_includes_both_start_and_end() {
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        let range = DateRange::new(start, end);

        let days: Vec<NaiveDate> = range.iter_days().collect();

        assert_eq!(days.len(), 5);
        assert_eq!(days.first(), Some(&start));
        assert_eq!(days.last(), Some(&end));
    }
}
