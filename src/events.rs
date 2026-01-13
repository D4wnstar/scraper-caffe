use std::{collections::HashSet, fmt, hash::Hash};

use fancy_regex::Regex;

use crate::dates::DateRange;

/// An event somewhere, at some time.
#[derive(Debug, Clone)]
pub struct Event {
    pub id: String,
    pub title: String,
    pub date: Option<DateRange>,
    pub locations: Locations,
    pub category: String,
    pub description: Option<String>,
    pub summary: Option<String>,
    pub tags: HashSet<String>,
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let title = format!("_{}_", self.title);
        let tags = self
            .tags
            .iter()
            .fold(String::new(), |acc, tag| format!("{acc} [{tag}]"));

        let date = self
            .date
            .as_ref()
            .map_or("".to_string(), |d| format!(", {d}"));

        let location = if self.locations.is_empty() {
            "".to_string()
        } else {
            format!(" ({})", self.locations)
        };

        write!(f, "{title}{tags}{date}{location}")
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
    pub fn new(title: &str, locations: Locations, category: &str) -> Self {
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

    /// Add all locations of the argument [Event] into the caller [Event] in place.
    pub fn extend_locations(&mut self, other: &Self) {
        self.locations.extend(other.locations.clone());
    }
}

#[derive(Debug, Clone)]
pub struct Locations {
    locs: HashSet<String>,
}

impl Locations {
    pub fn from_loc(loc: String) -> Self {
        Self {
            locs: HashSet::from([loc]),
        }
    }

    pub fn from_locs(locs: Vec<String>) -> Self {
        Self {
            locs: HashSet::from_iter(locs),
        }
    }
}

impl fmt::Display for Locations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Print in a comma-separated list
        let text = self
            .locs
            .iter()
            .enumerate()
            .fold(String::new(), |acc, (i, new)| {
                if i == 0 {
                    new.to_string()
                } else {
                    format!("{acc}, {new}")
                }
            });

        write!(f, "{text}")
    }
}

impl Locations {
    pub fn extend(&mut self, locs: Locations) {
        self.locs.extend(locs.dump());
    }

    pub fn is_empty(&self) -> bool {
        self.locs.is_empty()
    }

    pub fn dump(self) -> HashSet<String> {
        self.locs
    }
}
