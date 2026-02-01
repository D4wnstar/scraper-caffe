use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use anyhow::Result;
use convert_case::Case;
use headless_chrome::{Browser, LaunchOptions};
use indicatif::{ProgressBar, ProgressStyle};
use scraper::{Html, Selector};
use serde_json::Value;

use crate::{
    dates::{DateRange, DateSet, TimeFrame},
    events::Event,
    utils::PROGRESS_BAR_TEMPLATE,
    venues::{
        CATEGORY_MOVIES, StandardCasing,
        cinemas::{Cinema, MovieGroup},
    },
};

pub async fn fetch(date_range: &DateRange) -> Result<Vec<MovieGroup>> {
    // The Space's website is a Next.js app and contains absolutely zero functional
    // HTML without JavaScript. It does contain a JSON object that contains a bunch of content,
    // but only a few movies. Thankfully, the movies are taken from an server API route that
    // returns a nice and convenient list of movies and all their metadata.

    let progress = ProgressBar::new(0)
        .with_style(ProgressStyle::with_template(PROGRESS_BAR_TEMPLATE).unwrap())
        .with_message("Fetching The Space");

    // We need a proper browser here because the API function isn't really meant to be
    // accessed from code, so it seems to check for fresh session cookies
    let browser =
        headless_chrome::Browser::new(LaunchOptions::default_builder().path(None).build().unwrap())
            .unwrap();

    let mut movie_groups: HashMap<String, MovieGroup> = HashMap::new();
    for day in date_range.iter_days() {
        let url = format!(
            "https://www.thespacecinema.it/api/microservice/showings/cinemas/1011/films?showingDate={}&minEmbargoLevel=3&includesSession=true&includeSessionAttributes=true",
            day.format("%Y-%m-%dT00:00:00")
        );

        let mut listings: Vec<Value> = Vec::new();
        let mut attempt = 1;
        while attempt <= 3 {
            match call_api(&browser, &url).await {
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

        progress.inc_length(listings.len() as u64);

        for listing in listings.iter() {
            let title = listing["filmTitle"].as_str().unwrap();
            let (title, base_title, _) = super::clean_title(title, Cinema::TheSpace);
            let description = listing["synopsisShort"].as_str().unwrap();

            // To determine the tags, we need to look at the individual movie showings for the day
            // The showings are put in showingGroups. Each group is a day's worth of movies. Since
            // we're using the per-day API there's always one and only one. Each group contains the
            // showings in the sessions array. Each session has attributes that explain what variant
            // it is. We create a separate event for each distinct variant.
            let sessions = listing["showingGroups"].as_array().unwrap()[0]["sessions"]
                .as_array()
                .unwrap();
            for session in sessions {
                let mut tags = HashSet::new();
                for attr in session["attributes"].as_array().unwrap() {
                    match attr["name"].as_str().unwrap() {
                        "3D" => drop(tags.insert("3D".to_string())),
                        "LINGUA ORIGINALE" => drop(tags.insert("Originale".to_string())),
                        _ => {}
                    }
                }

                let id = super::make_id(&base_title, &tags);
                let dates = DateSet::new(vec![day]).unwrap();
                let movie = Event::new(
                    &title.standardize_case(Some(Case::Sentence)),
                    HashSet::from_iter(["The Space".to_string()]),
                    CATEGORY_MOVIES,
                )
                .with_id(id)
                .with_tags(tags)
                .with_time_frame(Some(TimeFrame::Dates(dates)));

                movie_groups
                    .entry(base_title.clone())
                    .and_modify(|group| {
                        group.add_movie(movie.clone());
                        // Prioritize The Space descriptions
                        group.description = Some(description.to_string());
                    })
                    .or_insert_with(|| MovieGroup {
                        title: base_title.clone(),
                        description: Some(description.to_string()),
                        movies: HashSet::from([movie]),
                    });
            }

            progress.inc(1);
        }

        // Await to not send too many requests too fast
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    progress.finish();

    return Ok(movie_groups.into_values().collect());
}

async fn call_api(browser: &Browser, url: &str) -> Result<Value> {
    // Navigate to the proper page to create session cookies
    let main_page = "https://www.thespacecinema.it/cinema/trieste/al-cinema";
    let tab = browser.new_tab().unwrap();
    tab.navigate_to(main_page).unwrap();
    tab.wait_until_navigated().unwrap();

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
