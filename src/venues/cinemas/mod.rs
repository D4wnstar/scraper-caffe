mod the_space;
mod triestecinema;

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use fancy_regex::Regex;
use lazy_static::lazy_static;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{dates::DateRange, events::Event, venues::CacheManager};

lazy_static! {
    static ref UPPERCASE_MATCHER: Regex = Regex::new(r"^[^a-z]+").unwrap();
    static ref ORIGINAL_LANG: Regex = Regex::new(r"(?i)In [\w\d ]+ Con S\.+t\.+ Italiani").unwrap();
    static ref HYPHENS: Regex = Regex::new(r" +\- +").unwrap();
    static ref SPACE_NUKE: Regex = Regex::new(r"(\s){2,}").unwrap();
    static ref PUNCTUATION_NUKE: Regex = Regex::new(r"[.,;:]").unwrap();
    static ref SUBTITLE_STRIPPER: Regex = Regex::new(r":\s+.*$").unwrap();
}

/// A set of movie [Event]s to handle multiple variants of the same movie. For instance,
/// a movie could be screened normally, in original language, in 3D, etc. These are different
/// events, but all the same movie.
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct MovieGroup {
    title: String,
    description: Option<String>,
    movies: HashSet<Event>,
}

impl MovieGroup {
    fn add_movie(&mut self, movie: Event) {
        if let Some(mut ext_movie) = self.movies.take(&movie) {
            ext_movie.locations.extend(movie.locations);

            if let Some(old_tf) = movie.time_frame {
                if let Some(ext_tf) = ext_movie.time_frame {
                    let new_tf = ext_tf.merge(&old_tf);
                    ext_movie.time_frame = Some(new_tf);
                }
            }

            self.movies.insert(ext_movie);
        } else {
            self.movies.insert(movie);
        }
    }
}

pub async fn fetch(
    client: &Client,
    date_range: &DateRange,
    cache_manager: &mut CacheManager,
) -> Result<Vec<Event>> {
    cache_manager.set_category("cinema");
    let triestecinema = cache_manager
        .get_or_fetch("triestecinema", async || {
            triestecinema::fetch(client, date_range).await
        })
        .await?;
    let the_space = cache_manager
        .get_or_fetch("the_space", async || the_space::fetch(date_range).await)
        .await?;

    // Combine identical movies in a single list
    let mut movie_groups: HashMap<String, MovieGroup> = HashMap::new();

    for groups in [triestecinema, the_space] {
        for group in groups {
            movie_groups
                .entry(group.title.clone())
                .and_modify(|ext_group| {
                    for movie in group.movies.clone() {
                        ext_group.add_movie(movie);
                    }
                    // Last existing description wins
                    if group.description.is_some() {
                        ext_group.description = group.description.clone();
                    }
                })
                .or_insert(group);
        }
    }

    let mut movies_by_group: Vec<Vec<Event>> = Vec::new();
    for group in movie_groups.into_values() {
        // Put base variants before special variants (e.g., 3D)
        let mut variants: Vec<Event> = group.movies.into_iter().collect();
        variants.sort_by(|a, b| a.tags.len().cmp(&b.tags.len()));
        // The last variant inherits the group description
        // This is because graphically this'll be printed last
        // and we should end on a group on its description
        if let Some(var) = variants.last_mut() {
            var.description = group.description;
        }
        movies_by_group.push(variants);
    }

    // Order alphabetically
    movies_by_group.sort_by(|a, b| a[0].title.cmp(&b[0].title));

    let movies: Vec<Event> = movies_by_group.into_iter().flatten().collect();
    return Ok(movies);
}

pub(super) enum Cinema {
    TriesteCinema,
    TheSpace,
}

pub(super) fn clean_title(title: &str, cinema: Cinema) -> (String, String, HashSet<String>) {
    let mut new_title = title.to_string();

    // Annoyances
    match cinema {
        Cinema::TriesteCinema => {
            new_title = new_title.replace("/", "").replace("4K", "");
            new_title = UPPERCASE_MATCHER
                .find(&new_title)
                .ok()
                .flatten()
                .map(|m| m.as_str().to_string())
                .unwrap_or(new_title);
        }
        Cinema::TheSpace => {}
    }

    new_title = new_title
        .to_lowercase()
        .replace("a'", "à")
        .replace("e'", "è")
        .trim()
        .to_string();

    new_title = HYPHENS.replace_all(&new_title, ": ").to_string();
    new_title = SPACE_NUKE.replace_all(&new_title, "$1").to_string();

    // Possible tags
    let mut tags: HashSet<String> = HashSet::new();
    let mut extract = |text: &str, search: &str, tag: &str| {
        if text.contains(search) {
            tags.insert(tag.to_string());
        }
        return text.replace(search, "");
    };
    new_title = extract(&new_title, "in 3d", "3D");

    let mut extract_re = |text: &str, search: &Regex, tag: &str| {
        if search.is_match(text).unwrap() {
            tags.insert(tag.to_string());
        }
        return search.replace_all(text, "").to_string();
    };
    new_title = extract_re(&new_title, &ORIGINAL_LANG, "Originale Sottotitolato");

    // Base title without subtitle
    let base_title = SUBTITLE_STRIPPER.replace_all(&new_title, "");

    return (
        new_title.trim().to_string(),
        base_title.trim().to_string(),
        tags,
    );
}

/// Make an identifier that's inclusive of tags to differentiate the same movie
/// in different contexts (e.g., 2D vs. 3D vs. original language).
pub(super) fn make_id(base_title: &str, tags: &HashSet<String>) -> String {
    let mut id = base_title.to_string();
    if !tags.is_empty() {
        let mut tags_vec: Vec<String> = tags.iter().cloned().collect();
        tags_vec.sort();
        let tag_id = tags_vec
            .iter()
            .fold(String::new(), |acc, new| format!("{acc} {new}"))
            .trim()
            .to_string();
        id = format!("{id} {tag_id}");
    }

    id = PUNCTUATION_NUKE
        .replace_all(&id, "")
        .trim()
        .replace(" ", "_")
        .to_lowercase();

    return id;
}
