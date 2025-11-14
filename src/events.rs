use std::{collections::HashSet, fmt, hash::Hash};

/// An event somewhere, at some time.
#[derive(Debug, Clone)]
pub struct Event {
    pub title: String,
    pub date: Option<String>,
    pub locations: Locations,
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let title = format!("_{}_", self.title);
        let date = self
            .date
            .as_ref()
            .map_or("".to_string(), |d| format!(" @ {d}"));

        let location = if self.locations.is_empty() {
            "".to_string()
        } else {
            format!(" ({})", self.locations)
        };

        write!(f, "{title}{date}{location}")
    }
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title
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
        self.title.hash(state);
    }
}

impl Event {
    /// Add all locations of the argument Event into the caller Event.
    /// Consumes the argument.
    pub fn merge_by_location(mut self, other: Self) -> Self {
        self.locations.extend(other.locations);
        return self;
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
