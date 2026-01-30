pub mod cinemas;
pub mod custom;
pub mod libraries;
pub mod theaters;

use anyhow::Result;
use convert_case::{Case, Casing};
use fancy_regex::{Captures, Regex};
use lazy_static::lazy_static;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fs;
use std::path::PathBuf;

/// Generic cache manager for venue data
pub struct CacheManager {
    cache_dir: PathBuf,
    cache: bool,
    rebuild: bool,
    venues_to_rebuild: Vec<String>,
    venues_to_skip: Vec<String>,
}

impl CacheManager {
    /// Create a new CacheManager for a category (e.g., "cinema", "theater")
    pub fn new(
        category: &str,
        cache: bool,
        rebuild: bool,
        venues_to_rebuild: Vec<String>,
        venues_to_skip: Vec<String>,
    ) -> Self {
        Self {
            cache_dir: PathBuf::from(format!("cache/{category}")),
            cache,
            rebuild,
            venues_to_rebuild,
            venues_to_skip,
        }
    }

    pub fn set_category(&mut self, category: &str) {
        self.cache_dir = PathBuf::from(format!("cache/{category}"));
    }

    /// Load from cache if exists and valid, otherwise fetch and cache.
    ///
    /// Returns the data whether from cache or freshly fetched.
    pub async fn get_or_fetch<V, F>(&self, venue_name: &str, fetcher: F) -> Result<Option<V>>
    where
        V: Serialize + DeserializeOwned,
        F: AsyncFnOnce() -> Result<V>,
    {
        if self.venues_to_skip.contains(&venue_name.to_string()) {
            println!("Skipping {venue_name}");
            return Ok(None);
        }

        let cache_path = self.cache_dir.join(format!("{venue_name}.json"));

        // Try to load from cache
        if self.cache && !self.rebuild && !self.venues_to_rebuild.contains(&venue_name.to_string())
        {
            if let Ok(exists) = fs::exists(&cache_path) {
                if exists {
                    println!("Loading {venue_name}.json from cache");
                    let content = fs::read_to_string(&cache_path)?;
                    return Ok(Some(serde_json::from_str(&content)?));
                }
            }
        }

        // Fetch from API
        let result = fetcher().await?;

        // Write to cache if caching is enabled
        if self.cache {
            fs::create_dir_all(&self.cache_dir)?;
            let serialized = serde_json::to_string(&result)?;
            fs::write(&cache_path, serialized)?;
        }

        Ok(Some(result))
    }
}

pub trait StandardCasing {
    /// Casing conversion with extra grammatical rules.
    /// Provide the current casing of the string, if known, through `starting_case`
    /// help the conversion engine reduce errors.
    fn standardize_case(&self, starting_case: Option<Case>) -> String;
}

impl StandardCasing for String {
    fn standardize_case(&self, starting_case: Option<Case>) -> String {
        let mut text = self
            .from_case(starting_case.unwrap_or(Case::Sentence))
            .to_case(Case::Title);

        // Make grammatical particles lowercase
        text = PARTICLES
            .replace_all(&text, |caps: &Captures| {
                caps.get(0).unwrap().as_str().to_lowercase()
            })
            .to_string();

        // Make the letter after some appropriate apostrophes uppercase
        text = APOSTROPHES
            .replace_all(&text, |caps: &Captures| {
                let particle = caps.get(1).unwrap().as_str();
                let letter = caps.get(2).unwrap().as_str().to_uppercase();
                format!("{particle}'{letter}")
            })
            .to_string();

        // Make the letter immediately after quotes uppercase
        text = QUOTES
            .replace_all(&text, |caps: &Captures| {
                caps.get(0).unwrap().as_str().to_uppercase()
            })
            .to_string();

        // Replace regular quotes with fancy quotes for display typography
        text = text.replace("'", "’");
        text = QUOTES_FANCY.replace_all(&text, "“$1”").to_string();

        return text;
    }
}

impl StandardCasing for str {
    fn standardize_case(&self, starting_case: Option<Case>) -> String {
        self.to_string().standardize_case(starting_case)
    }
}

lazy_static! {
    static ref PARTICLES: Regex = Regex::new(
        r"(?i)(?<=.)(?<![.:;] )\b(il|la?|le|gli|una?|ad?|ed?|i|o|di?|in|con|per|tra|fra|si|(?:a|da|de|su|ne)(?:i|l|ll|lla|lle|gli)?)\b"
    )
    .unwrap();
    static ref APOSTROPHES: Regex = Regex::new(r"(?i)\b(l|d|s|un|(?:a|da|de|su|ne)ll)(?:'|’)(\w)").unwrap();
    static ref QUOTES: Regex = Regex::new(r#"("|“|”)\w"#).unwrap();
    static ref QUOTES_FANCY: Regex = Regex::new(r#""(.*?)""#).unwrap();
}
