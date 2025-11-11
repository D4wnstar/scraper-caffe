mod events;

use anyhow::Result;
use reqwest::{self, Client};
use scraper::{Html, Selector};

use crate::events::Event;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new();

    let movies = fetch_movies(&client).await?;

    // Print results
    println!("\n--- Compiled List of Events ---");
    for event in movies {
        println!("{}", event);
    }

    Ok(())
}

async fn fetch_movies(client: &Client) -> Result<Vec<Event>> {
    let mut movies: Vec<Event> = Vec::new();

    let mymovies_url = "https://www.mymovies.it/cinema/trieste/";
    let html_body = client.get(mymovies_url).send().await?.text().await?;
    let document = Html::parse_document(&html_body);

    let movie_box_sel = Selector::parse("div.mm-white.mm-padding-8").unwrap();
    let title_sel = Selector::parse("div.schedine-titolo > a[title]").unwrap();
    let cinemas_sel = Selector::parse("div.mm-light-grey a[title] > div > div").unwrap();
    for movie_box in document.select(&movie_box_sel) {
        let maybe_title_el = movie_box.select(&title_sel).next();
        if let None = maybe_title_el {
            continue;
        }
        let title = maybe_title_el
            .unwrap()
            .attr("title")
            .unwrap_or("NO TITLE")
            .trim()
            .to_string();

        let cinemas_els = movie_box.select(&cinemas_sel);
        let mut cinemas = Vec::new();
        for cinema_el in cinemas_els {
            // There should be only one text node in the cinema element
            let cinema = cinema_el
                .text()
                .next()
                .unwrap()
                .replace("Trieste", "")
                .replace("CINEMA", "");
            cinemas.push(cinema.trim().to_string());
        }
        let location = cinemas
            .into_iter()
            .reduce(|acc, new| format!("{acc}, {new}"));

        let movie = Event {
            title,
            date: None,
            location,
        };
        movies.push(movie);
    }

    return Ok(movies);
}
