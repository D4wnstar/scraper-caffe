mod the_space;
mod triestecinema;

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use fancy_regex::Regex;
use lazy_static::lazy_static;
use reqwest::Client;

use crate::{dates::DateRange, events::Event};

lazy_static! {
    static ref ORIGINAL_LANG: Regex = Regex::new(r"(?i)In [\w\d ]+ Con S\.+t\.+ Italiani").unwrap();
    static ref LEONE: Regex = Regex::new(r"(?i)Leone d'oro.*").unwrap();
    static ref HYPHENS: Regex = Regex::new(r" +\- +").unwrap();
    static ref SPACE_NUKE: Regex = Regex::new(r"(\s){2,}").unwrap();
    static ref PUNCTUATION_NUKE: Regex = Regex::new(r"[.,;:]").unwrap();
    static ref SUBTITLE_STRIPPER: Regex = Regex::new(r":\s+.*$").unwrap();
}

/// A set of movie [Event]s to handle multiple variants of the same movie. For instance,
/// a movie could be screened normally, in original language, in 3D, etc. These are different
/// events, but all the same movie.
#[derive(Debug)]
pub(super) struct MovieGroup {
    description: Option<String>,
    movies: HashSet<Event>,
}

pub async fn fetch(client: &Client, current_week: &DateRange) -> Result<Vec<Event>> {
    let mut movie_groups: HashMap<String, MovieGroup> = HashMap::new();

    triestecinema::fetch(client, current_week, &mut movie_groups).await?;
    the_space::fetch(client, &mut movie_groups).await?;

    // Collapse groups into a flat list
    let mut movies_by_group: Vec<Vec<Event>> = Vec::new();
    for (_, group) in movie_groups.into_iter() {
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
    movies_by_group.sort_by_key(|movies| movies[0].title.clone());

    let movies: Vec<Event> = movies_by_group.into_iter().flatten().collect();
    return Ok(movies);
}

pub(super) fn clean_title(title: &str) -> (String, String, HashSet<String>) {
    // Full title to be displayed
    let mut new_title = title
        .to_lowercase()
        .replace("ultimi giorni", "")
        .replace(" / ultimo giorno", "")
        .replace("4k", "")
        .replace("a'", "à")
        .replace("e'", "è")
        .trim()
        .to_string();

    // Annoyances
    new_title = LEONE.replace_all(&new_title, "").to_string();
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

pub(super) fn add_or_merge_to_group(group: &mut MovieGroup, movie: &Event) {
    if group.movies.contains(movie) {
        // Merge location if the variant already exists
        let mut existing_variant = group.movies.take(movie).unwrap();
        existing_variant.locations.extend(movie.locations.clone());
        group.movies.insert(existing_variant);
    } else {
        group.movies.insert(movie.clone());
    };
}
