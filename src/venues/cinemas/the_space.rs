use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use anyhow::Result;
use convert_case::{Case, Casing};
use headless_chrome::LaunchOptions;
use indicatif::{ProgressBar, ProgressFinish, ProgressIterator, ProgressStyle};
use scraper::{Html, Selector};
use serde_json::Value;

use crate::{events::Event, utils::PROGRESS_BAR_TEMPLATE, venues::cinemas::MovieGroup};

pub async fn fetch() -> Result<Vec<MovieGroup>> {
    // The Space's website is a Next.js app and contains absolutely zero functional
    // HTML without JavaScript. It does contain a JSON object that contains a bunch of content,
    // but only a few movies Thankfully, the movies are taken from an server API route that
    // returns a nice and convenient list of movies and all their metadata (except showtimes).
    // It's not really public, so sometimes it throws 401 Unauthorized errors. It's weirdly
    // inconsistent though, same API and same user agent can cause different statuses even
    // across a few seconds.
    // Also see for date-specific movie lists:
    // https://www.thespacecinema.it/api/microservice/showings/cinemas/1011/films?showingDate=2026-01-14T00:00:00&minEmbargoLevel=3&includesSession=true&includeSessionAttributes=true
    let url = "https://www.thespacecinema.it/api/microservice/showings/cinemas/1011/films?minEmbargoLevel=2&includesSession=false&includeSessionAttributes=true";

    let mut listings: Vec<Value> = Vec::new();
    let mut attempt = 1;
    while attempt <= 3 {
        match call_api(url).await {
            Ok(json) => {
                listings = json["result"].as_array().unwrap().to_vec();
                break;
            }
            Err(e) => {
                eprintln!("Error: {e}. Attempt: {attempt} of 3. Retrying in 5 seconds...");
                attempt += 1;
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }

    let progress = ProgressBar::new(listings.len() as u64)
        .with_style(ProgressStyle::with_template(PROGRESS_BAR_TEMPLATE).unwrap())
        .with_message("Fetching The Space")
        .with_finish(ProgressFinish::AndLeave);

    let mut movie_groups: HashMap<String, MovieGroup> = HashMap::new();

    for listing in listings.iter().progress_with(progress) {
        let (title, base_title, _) = super::clean_title(listing["filmTitle"].as_str().unwrap());
        let description = listing["synopsisShort"].as_str().unwrap();

        // These contain properties for the showings that we need for tags. However
        // each listing isn't a single showing, but rather a collection of all showings that are
        // done in the cinema. As such, each attribute is counted as a separate [Event]
        // For instance, a movie with 3D and LINGUA ORIGINALE is actually three
        // events, one in 2D, one in 3D and one in original language
        let attributes = listing["sessionAttributes"].as_array().unwrap();
        for attr in attributes {
            let tags = match attr["name"].as_str().unwrap() {
                "2D" => HashSet::new(),
                "3D" => HashSet::from(["3D".to_string()]),
                "LINGUA ORIGINALE" => HashSet::from(["Originale Sottotitolato".to_string()]),
                _ => continue,
            };
            let id = super::make_id(&base_title, &tags);

            let movie = Event::new(
                &title.from_case(Case::Sentence).to_case(Case::Title),
                HashSet::from_iter(["The Space".to_string()]),
                "Film",
            )
            .with_id(id)
            .with_tags(tags);

            movie_groups
                .entry(base_title.clone())
                .and_modify(|group| {
                    super::add_or_merge_to_group(group, &movie);
                    // Prioritize The Space descriptions
                    group.description = Some(description.to_string());
                })
                .or_insert_with(|| MovieGroup {
                    title: base_title.clone(),
                    description: Some(description.to_string()),
                    movies: HashSet::from([movie]),
                });
        }
    }

    return Ok(movie_groups.into_values().collect());
}

async fn call_api(url: &str) -> Result<Value> {
    // We need a proper browser here because the API function isn't really meant to be
    // accessed from code, so it seems to check for fresh session cookies
    let browser =
        headless_chrome::Browser::new(LaunchOptions::default_builder().path(None).build().unwrap())
            .unwrap();

    // Navigate to the proper page to create session cookies
    let tab = browser.new_tab().unwrap();
    tab.navigate_to("https://www.thespacecinema.it/cinema/trieste/al-cinema")
        .unwrap()
        .wait_until_navigated()
        .unwrap();

    // Call the API URL
    tab.navigate_to(url).unwrap();
    tab.wait_until_navigated().unwrap();
    let content = tab.get_content().unwrap();

    // Extract the JSON from the response
    let doc = Html::parse_document(&content);
    let json = doc
        .select(&Selector::parse("pre").unwrap())
        .next()
        .and_then(|el| el.text().next())
        .unwrap();

    let value = serde_json::from_str(json)?;

    return Ok(value);
}
