use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt, hash::Hash};

use crate::dates::{DateRange, DateSet, TimeFrame};

/// An event somewhere, at some time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub title: String,
    pub time_frame: Option<TimeFrame>,
    pub locations: HashSet<String>,
    pub category: String,
    pub description: Option<String>,
    // pub summary: Option<String>,
    pub tags: HashSet<String>,
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let title = format!("**{}**", self.title);
        let tags = self
            .tags
            .iter()
            .fold(String::new(), |acc, tag| format!("{acc} **[{tag}]**"));

        let date = match &self.time_frame {
            None => String::new(),
            Some(time_frame) => {
                let tf_text = match time_frame {
                    TimeFrame::Dates(set) => fmt_date_set(set),
                    TimeFrame::Period(range) => fmt_date_range(range),
                };
                format!(", {tf_text}")
            }
        };

        let mut locs: Vec<String> = self.locations.iter().cloned().collect();
        locs.sort();
        let loc_text = locs
            .iter()
            .enumerate()
            .fold(String::new(), |acc, (i, new)| {
                if i == 0 {
                    new.to_string()
                } else {
                    format!("{acc}, {new}")
                }
            });
        let locations = format!(" ({})", loc_text);

        write!(f, "{title}{tags}{date}{locations}")
    }
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Event {}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.title.cmp(&other.title)
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for Event {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Event {
    pub fn new(title: &str, locations: HashSet<String>, category: &str) -> Self {
        Self {
            id: title.to_string(),
            title: title.to_string(),
            time_frame: None,
            locations,
            category: category.to_string(),
            description: None,
            // summary: None,
            tags: HashSet::new(),
        }
    }

    pub fn with_id(self: Self, id: String) -> Self {
        Self { id, ..self }
    }

    pub fn with_time_frame(self: Self, date: Option<TimeFrame>) -> Self {
        Self {
            time_frame: date,
            ..self
        }
    }

    #[allow(unused)]
    pub fn with_description(self: Self, description: Option<String>) -> Self {
        Self {
            description,
            ..self
        }
    }

    pub fn with_tags(self: Self, tags: HashSet<String>) -> Self {
        Self { tags, ..self }
    }
}

fn fmt_date_set(set: &DateSet) -> String {
    if set.dates().len() == 1 {
        format!("il {}", set.first().format("%d/%m"))
    } else {
        let str = set
            .dates()
            .iter()
            .enumerate()
            .fold("il ".to_string(), |acc, (i, date)| {
                let str = date.format("%d/%m");
                if i == set.dates().len() - 1 {
                    format!("{acc} e {str}")
                } else if i == set.dates().len() - 2 {
                    format!("{acc} {str}")
                } else {
                    format!("{acc} {str}, ")
                }
            });

        format!("{str}")
    }
}

fn fmt_date_range(range: &DateRange) -> String {
    format!(
        "dal {} al {}",
        range.start.format("%d/%m/%Y"),
        range.end.format("%d/%m/%Y")
    )
}
