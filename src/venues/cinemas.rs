use std::{collections::HashSet, time::Duration};

use anyhow::Result;
use convert_case::{Case, Casing};
use fancy_regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};

use crate::events::{Event, Locations};

pub async fn fetch(client: &Client) -> Result<Vec<Event>> {
    let mut movies: HashSet<Event> = HashSet::new();

    let subtitle_regex = Regex::new(r"(?i)In [\w\d ]+ Con S\.+t\.+ Italiani").unwrap();
    let leone_regex = Regex::new(r"(?i)Leone d'oro.*").unwrap();
    let hyphen_regex = Regex::new(r" +\- +").unwrap();

    // Fetch movies from TriesteCinema for each day of the week
    for delta in 0..=6 {
        let cinema_url = format!("https://www.triestecinema.it/index.php?pag=orari&delta={delta}");
        let html_body = client.get(cinema_url).send().await?.text().await?;
        let document = Html::parse_document(&html_body);

        let movie_list_sel = Selector::parse("div.media-body").unwrap();
        let cinema_sel = Selector::parse("h3.media-heading").unwrap();
        let title_sel = Selector::parse("a.oggi").unwrap();
        for movie_list in document.select(&movie_list_sel) {
            // All text here is in UPPERCASE
            let cinema = movie_list
                .select(&cinema_sel)
                .next()
                .expect("Each list should have one cinema heading")
                .text()
                .next()
                .expect("Each heading should have text")
                .trim()
                .from_case(Case::Upper)
                .to_case(Case::Title);

            let titles: Vec<&str> = movie_list
                .select(&title_sel)
                .map(|title| title.text().next().expect("Each title should have text"))
                .collect();

            for title in titles {
                let mut title = title
                    .replace("ultimi giorni", "")
                    .replace("4K", "")
                    .trim()
                    .from_case(Case::Upper)
                    .to_case(Case::Title)
                    .replace("In 3d", "[3D]");

                title = subtitle_regex
                    .replace_all(&title, "[Originale Sottotitolato]")
                    .to_string();
                title = leone_regex.replace_all(&title, "").to_string();
                title = hyphen_regex.replace_all(&title, ": ").to_string();
                title = title.trim().to_string();

                let movie = Event {
                    title,
                    date: None,
                    locations: Locations::from_loc(cinema.to_string()),
                    category: "Film".to_string(),
                };

                if movies.contains(&movie) {
                    let existing_movie = movies.get(&movie).unwrap();
                    let merged = movie.merge_by_location(existing_movie.clone());
                    movies.replace(merged);
                } else {
                    movies.insert(movie);
                };
            }
        }

        // Await to not send too many requests too fast
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Also fetch The Space movies from MyMovies becauase the actual website needs JavaScript
    // jank to load the movie list at runtime after the page loads, meaning it does not work
    // outside of a persistent browser
    // This only fetches movies for today
    {
        let the_space_url = "https://www.mymovies.it/cinema/trieste/5894/";
        let html_body = client.get(the_space_url).send().await?.text().await?;
        let document = Html::parse_document(&html_body);

        let title_sel =
            Selector::parse("div.mm-white.mm-padding-8 div.schedine-titolo > a[title]").unwrap();
        for title_el in document.select(&title_sel) {
            let title = title_el
                .attr("title")
                .unwrap()
                .replace(" - ", ": ")
                .from_case(Case::Sentence)
                .to_case(Case::Title);
            let movie = Event {
                title,
                date: None,
                locations: Locations::from_loc("The Space".to_string()),
                category: "Film".to_string(),
            };

            if movies.contains(&movie) {
                let existing_movie = movies.get(&movie).unwrap();
                let merged = movie.merge_by_location(existing_movie.clone());
                movies.replace(merged);
            } else {
                movies.insert(movie);
            };
        }
    }

    // Order alphabetically
    let mut ordered_movies: Vec<Event> = movies.into_iter().collect();
    ordered_movies.sort();

    return Ok(ordered_movies);
}
