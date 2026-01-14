use std::{collections::HashSet, fmt, hash::Hash};

use crate::dates::DateRange;

/// An event somewhere, at some time.
#[derive(Debug, Clone)]
pub struct Event {
    pub id: String,
    pub title: String,
    pub date: Option<DateRange>,
    pub locations: HashSet<String>,
    pub category: String,
    pub description: Option<String>,
    pub summary: Option<String>,
    pub tags: HashSet<String>,
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let title = format!("**{}**", self.title);
        let tags = self
            .tags
            .iter()
            .fold(String::new(), |acc, tag| format!("{acc} **[{tag}]**"));

        let date = self
            .date
            .as_ref()
            .map_or("".to_string(), |d| format!(", {d}"));

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
            date: None,
            locations,
            category: category.to_string(),
            description: None,
            summary: None,
            tags: HashSet::new(),
        }
    }

    pub fn id(self: Self, id: String) -> Self {
        Self { id, ..self }
    }

    pub fn date(self: Self, date: Option<DateRange>) -> Self {
        Self { date, ..self }
    }

    pub fn description(self: Self, description: Option<String>) -> Self {
        Self {
            description,
            ..self
        }
    }

    pub fn tags(self: Self, tags: HashSet<String>) -> Self {
        Self { tags, ..self }
    }
}
