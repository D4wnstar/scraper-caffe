mod dates;
mod events;

use std::{collections::HashSet, time::Duration};

use anyhow::Result;
use chrono::Days;
use convert_case::{Case, Casing};
use reqwest::{self, Client};
use scraper::{Html, Selector};

use crate::{
    dates::{DateRange, rossetti::parse_rossetti_date, verdi::parse_verdi_date},
    events::{Event, Locations},
};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new();
    let today = chrono::Local::now().date_naive();
    let in_a_week = today.checked_add_days(Days::new(7)).unwrap();
    let current_week = DateRange::new(today, in_a_week);

    let movies = fetch_movies(&client).await?;
    let shows = fetch_theaters(&client, &current_week).await?;

    println!("--- QUESTA SETTIMANA A TRIESTE ---");
    println!("(Questa lista è generata automaticamente e potrebbe contenere errori o duplicati)");
    println!("\n-- FILM --");
    for event in movies {
        println!("- {event}");
    }

    println!("\n-- TEATRI --");
    for event in shows {
        println!("- {event}")
    }

    Ok(())
}

async fn fetch_movies(client: &Client) -> Result<Vec<Event>> {
    let mut movies: HashSet<Event> = HashSet::new();

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
                let title = title
                    .replace("ultimi giorni", "")
                    .trim()
                    .from_case(Case::Upper)
                    .to_case(Case::Title);
                let movie = Event {
                    title,
                    date: None,
                    locations: Locations::from_loc(cinema.to_string()),
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
                .from_case(Case::Sentence)
                .to_case(Case::Title);
            let movie = Event {
                title,
                date: None,
                locations: Locations::from_loc("The Space".to_string()),
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

async fn fetch_theaters(client: &Client, current_week: &DateRange) -> Result<Vec<Event>> {
    let mut events = Vec::new();
    events.extend(fetch_rossetti(client, current_week).await?);
    events.extend(fetch_teatroverdi(client, current_week).await?);

    Ok(events)
}

async fn fetch_rossetti(client: &Client, current_week: &DateRange) -> Result<Vec<Event>> {
    let mut events: HashSet<Event> = HashSet::new();

    let url = "https://www.ilrossetti.it/it/stagione/cartellone";
    let html_body = client.get(url).send().await?.text().await?;
    let document = Html::parse_document(&html_body);

    let shows_sel = Selector::parse("div.single-show:not(.single-show--disabled)").unwrap();
    let title_sel = Selector::parse("div.single-show__title > a").unwrap();
    let date_sel = Selector::parse("div.single-show__date").unwrap();
    for show in document.select(&shows_sel) {
        let title_el = show.select(&title_sel).next();
        if let None = title_el {
            continue;
        }
        let title = title_el
            .unwrap()
            .text()
            .next()
            .expect("Each event card should have text")
            .from_case(Case::Upper)
            .to_case(Case::Title);

        let date_el = show.select(&date_sel).next();
        if let None = date_el {
            continue;
        }
        let date_str = date_el
            .unwrap()
            .text()
            .skip(1) // First text elem is an empty string (due to the icon probably)
            .next()
            .expect("Second text element should always be the date")
            .trim()
            .to_string();
        let date_range =
            parse_rossetti_date(&date_str).expect("Date should be in a standardized format");

        // Skip events not in the current week
        if !date_range.overlaps(&current_week) {
            continue;
        }

        let event = Event {
            title,
            date: Some(date_range),
            locations: Locations::from_loc("Rossetti".to_string()),
        };
        events.insert(event);
    }

    // Order alphabetically
    let mut ordered_events: Vec<Event> = events.into_iter().collect();
    ordered_events.sort();

    Ok(ordered_events)
}

async fn fetch_teatroverdi(client: &Client, current_week: &DateRange) -> Result<Vec<Event>> {
    let mut events: HashSet<Event> = HashSet::new();

    let url = "https://www.teatroverdi-trieste.com/it/calendario-spettacoli/";
    let html_body = client.get(url).send().await?.text().await?;
    let document = Html::parse_document(&html_body);

    let shows_sel = Selector::parse("ul.spettacolo-list div.list-text").unwrap();
    let title_sel = Selector::parse("h2.spettacolo-list-title > a").unwrap();
    let date_sel = Selector::parse("span.spettacolo-list-date > strong").unwrap();
    for show in document.select(&shows_sel) {
        let title_el = show.select(&title_sel).next();
        if let None = title_el {
            continue;
        }
        let title = title_el.unwrap().text().next().unwrap().to_string();

        let date_el = show.select(&date_sel).next();
        if let None = date_el {
            continue;
        }
        let date_str = date_el.unwrap().text().next().unwrap();
        let date_range =
            parse_verdi_date(date_str).expect("Date should be in a standardized format");

        // Skip events not in the current week
        if !date_range.overlaps(&current_week) {
            continue;
        }

        let event = Event {
            title,
            date: Some(date_range),
            locations: Locations::from_loc("Verdi".to_string()),
        };
        events.insert(event);
    }

    // Order alphabetically
    let mut ordered_events: Vec<Event> = events.into_iter().collect();
    ordered_events.sort();

    return Ok(ordered_events);
}
