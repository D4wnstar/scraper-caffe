use serde::{Deserialize, Serialize};
use std::{collections::HashSet, hash::Hash};

use crate::dates::TimeFrame;

#[derive(Serialize, Deserialize)]
pub struct Category {
    pub name: String,
    pub events: Vec<Event>,
}

/// An event somewhere, at some time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub title: String,
    pub time_frame: Option<TimeFrame>,
    pub locations: HashSet<String>,
    pub category: String,
    pub description: Option<String>,
    pub summary: Option<String>,
    pub tags: HashSet<String>,
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
            summary: None,
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

    pub fn with_description(self: Self, description: Option<String>) -> Self {
        Self {
            description,
            ..self
        }
    }

    pub fn with_summary(self: Self, summary: Option<String>) -> Self {
        Self { summary, ..self }
    }

    pub fn with_tags(self: Self, tags: HashSet<String>) -> Self {
        Self { tags, ..self }
    }
}
