pub mod cinemas;
pub mod custom;
pub mod theaters;

use anyhow::Result;
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
}

impl CacheManager {
    /// Create a new CacheManager for a category (e.g., "cinema", "theater")
    pub fn new(category: &str, cache: bool, rebuild: bool, venues_to_rebuild: Vec<String>) -> Self {
        Self {
            cache_dir: PathBuf::from(format!("cache/{category}")),
            cache,
            rebuild,
            venues_to_rebuild,
        }
    }

    pub fn set_category(&mut self, category: &str) {
        self.cache_dir = PathBuf::from(format!("cache/{category}"));
    }

    /// Load from cache if exists and valid, otherwise fetch and cache.
    ///
    /// Returns the data whether from cache or freshly fetched.
    pub async fn get_or_fetch<V, F>(&self, venue_name: &str, fetcher: F) -> Result<V>
    where
        V: Serialize + DeserializeOwned,
        F: AsyncFnOnce() -> Result<V>,
    {
        let cache_path = self.cache_dir.join(format!("{venue_name}.json"));

        // Try to load from cache
        if self.cache && !self.rebuild && !self.venues_to_rebuild.contains(&venue_name.to_string())
        {
            if let Ok(exists) = fs::exists(&cache_path) {
                if exists {
                    println!("Loading {venue_name}.json from cache");
                    let content = fs::read_to_string(&cache_path)?;
                    return Ok(serde_json::from_str(&content)?);
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

        Ok(result)
    }
}
