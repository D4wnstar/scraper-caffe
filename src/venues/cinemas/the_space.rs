use std::collections::{HashMap, HashSet};

use anyhow::Result;
use convert_case::{Case, Casing};
use reqwest::Client;
use scraper::{Html, Selector};

use crate::{
    events::Event,
    venues::cinemas::{MovieGroup, clean_title},
};

pub async fn fetch(client: &Client, movie_groups: &mut HashMap<String, MovieGroup>) -> Result<()> {
    // Also fetch The Space movies from MyMovies because the actual website needs JavaScript
    // jank to load the movie list at runtime after the page loads, meaning it does not work
    // outside of a persistent browser
    // This only fetches movies for today
    let the_space_url = "https://www.mymovies.it/cinema/trieste/5894/";
    let html_body = client.get(the_space_url).send().await?.text().await?;
    let document = Html::parse_document(&html_body);

    let title_sel =
        Selector::parse("div.mm-white.mm-padding-8 div.schedine-titolo > a[title]").unwrap();
    for title_el in document.select(&title_sel) {
        let (title, base_title, id, tags) = clean_title(title_el.attr("title").unwrap());

        let movie = Event::new(
            &title.from_case(Case::Sentence).to_case(Case::Title),
            HashSet::from_iter(["The Space".to_string()]),
            "Film",
        )
        .id(id)
        .tags(tags);

        movie_groups
            .entry(base_title)
            .and_modify(|group| {
                if group.movies.contains(&movie) {
                    let mut existing_variant = group.movies.take(&movie).unwrap();
                    existing_variant.locations.extend(movie.locations.clone());
                    group.movies.insert(existing_variant);
                } else {
                    group.movies.insert(movie.clone());
                };
            })
            .or_insert_with(|| MovieGroup {
                description: None,
                movies: HashSet::from([movie]),
            });
    }

    return Ok(());
}
