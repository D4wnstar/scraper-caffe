use std::fmt;

/// An event.
#[derive(Debug)]
pub struct Event {
    pub title: String,
    pub date: Option<String>,
    pub location: Option<String>,
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let title = format!("{}", self.title);
        let date = self
            .date
            .as_ref()
            .map_or("".to_string(), |d| format!(" @ {d}"));
        let location = self
            .location
            .as_ref()
            .map_or("".to_string(), |d| format!(" ({d})"));

        write!(f, "{title}{date}{location}")
    }
}
