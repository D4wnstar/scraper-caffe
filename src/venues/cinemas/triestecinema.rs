use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use anyhow::Result;
use convert_case::{Case, Casing};
use reqwest::Client;
use scraper::{Html, Selector};

use crate::{
    dates::DateRange,
    events::Event,
    venues::cinemas::{MovieGroup, SPACE_NUKE, clean_title},
};

pub async fn fetch(
    client: &Client,
    date_range: &DateRange,
    movie_groups: &mut HashMap<String, MovieGroup>,
) -> Result<()> {
    let movie_list_sel = Selector::parse("div.media-body").unwrap();
    let cinema_sel = Selector::parse("h3.media-heading").unwrap();
    let title_sel = Selector::parse("a.oggi").unwrap();
    let desc_sel = Selector::parse("div.col-md-5.wow.fadeIn").unwrap();

    // Fetch movies from TriesteCinema for each request day
    let days = (date_range.end_date - date_range.start_date).num_days();
    for delta in 0..=days {
        let cinema_url = format!("https://www.triestecinema.it/index.php?pag=orari&delta={delta}");
        let html_body = client.get(cinema_url).send().await?.text().await?;
        let document = Html::parse_document(&html_body);

        for movie_list in document.select(&movie_list_sel) {
            // All text here is in UPPERCASE
            let cinema = movie_list
                .select(&cinema_sel)
                .next()
                .and_then(|e| e.text().next())
                .map(|s| s.trim().from_case(Case::Upper).to_case(Case::Title))
                .expect("Missing cinema header");

            let links: Vec<(&str, &str)> = movie_list
                .select(&title_sel)
                .map(|a| (a.text().next().unwrap(), a.attr("href").unwrap()))
                .collect();

            for (title, href) in links {
                let (title, base_title, id, tags) = clean_title(title);

                let url = &format!("https://www.triestecinema.it/{href}");
                let movie_page = client.get(url).send().await?.text().await?;
                let desc_doc = Html::parse_document(&movie_page);
                let mut description = desc_doc
                    .select(&desc_sel)
                    .skip(1)
                    .next()
                    .unwrap()
                    .text()
                    .fold(String::new(), |acc, new| format!("{acc}\n{new}"));

                description = SPACE_NUKE.replace_all(&description, "$1").trim().into();

                let movie = Event::new(
                    &title.from_case(Case::Upper).to_case(Case::Title),
                    HashSet::from_iter([cinema.to_string()]),
                    "Film",
                )
                .id(id)
                .tags(tags);

                movie_groups
                    .entry(base_title)
                    .and_modify(|group| {
                        if group.movies.contains(&movie) {
                            // Merge location if the variant already exists
                            let mut existing_variant = group.movies.take(&movie).unwrap();
                            existing_variant.locations.extend(movie.locations.clone());
                            group.movies.insert(existing_variant);
                        } else {
                            group.movies.insert(movie.clone());
                        };
                    })
                    .or_insert_with(|| MovieGroup {
                        description: Some(description),
                        movies: HashSet::from([movie]),
                    });

                // Await to not send too many requests too fast
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }

    return Ok(());
}
