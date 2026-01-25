use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use anyhow::Result;
use chrono::Days;
use convert_case::{Case, Casing};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use scraper::{Html, Selector};

use crate::{
    dates::{DateRange, DateSet, TimeFrame},
    events::Event,
    utils::PROGRESS_BAR_TEMPLATE,
    venues::cinemas::{Cinema, MovieGroup, SPACE_NUKE},
};

pub async fn fetch(client: &Client, date_range: &DateRange) -> Result<Vec<MovieGroup>> {
    let progress = ProgressBar::new(0)
        .with_style(ProgressStyle::with_template(PROGRESS_BAR_TEMPLATE).unwrap())
        .with_message("Fetching TriesteCinema");

    let mut movie_groups: HashMap<String, MovieGroup> = HashMap::new();

    let movie_list_sel = Selector::parse("div.media-body").unwrap();
    let cinema_sel = Selector::parse("h3.media-heading").unwrap();
    let title_sel = Selector::parse("a.oggi").unwrap();

    // Fetch movies from TriesteCinema for each request day
    for delta in 0..=date_range.days_spanned() {
        let curr_date = date_range.start.clone() + Days::new(delta as u64);

        let cinema_url = format!("https://www.triestecinema.it/index.php?pag=orari&delta={delta}");
        let html_body = client.get(cinema_url).send().await?.text().await?;
        let document = Html::parse_document(&html_body);

        let movie_count = document
            .select(&movie_list_sel)
            .fold(0, |acc, list| acc + list.select(&title_sel).count());
        progress.inc_length(movie_count as u64);

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
                let (title, base_title, tags) = super::clean_title(title, Cinema::TriesteCinema);
                let id = super::make_id(&base_title, &tags);

                // If the same variant already exists, skip
                if movie_groups
                    .get(&base_title)
                    .and_then(|e| e.movies.iter().find(|m| m.id == id))
                    .is_some()
                {
                    continue;
                }

                let description = get_description(client, href).await?;

                let dates = DateSet::new(vec![curr_date]).unwrap();

                let movie = Event::new(
                    &title.from_case(Case::Upper).to_case(Case::Title),
                    HashSet::from_iter([cinema.to_string()]),
                    "Film",
                )
                .with_id(id)
                .with_tags(tags.clone())
                .with_time_frame(Some(TimeFrame::Dates(dates)));

                movie_groups
                    .entry(base_title.clone())
                    .and_modify(|group| {
                        super::add_or_merge_to_group(group, movie.clone());
                        // triestecinema.it often doesn't have descriptions for
                        // tagged variants, so make sure to give that priority
                        if group.description.is_none() || tags.len() == 0 {
                            group.description = description.clone();
                        }
                    })
                    .or_insert_with(|| MovieGroup {
                        title: base_title,
                        description,
                        movies: HashSet::from([movie]),
                    });

                progress.inc(1);

                // Await to not send too many requests too fast
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }
    }

    progress.finish();

    return Ok(movie_groups.into_values().collect());
}

async fn get_description(client: &Client, href: &str) -> Result<Option<String>> {
    let desc_sel = Selector::parse("div.col-md-5.wow.fadeIn").unwrap();

    let url = &format!("https://www.triestecinema.it/{href}");
    let movie_page = client.get(url).send().await?.text().await?;
    let desc_doc = Html::parse_document(&movie_page);
    let description_el = desc_doc.select(&desc_sel).skip(1).next().unwrap();

    // The description page layout is incredibly inconsistent and sometimes does not have
    // a description. As a heuristic, the page has a description if it has at least 6 HTML
    // elements in the selector, in which case the description is inside the element with the
    // longest text content
    if description_el.child_elements().count() < 6 {
        return Ok(None);
    }

    let description = description_el
        .child_elements()
        .skip(5) // Skip the first 5
        .max_by(|el1, el2| {
            // Find the element with the most text
            let size1 = el1.text().fold(0, |acc, t| acc + t.len());
            let size2 = el2.text().fold(0, |acc, t| acc + t.len());
            size1.cmp(&size2)
        })
        .and_then(|el| {
            // Fold it in a string
            let desc = el
                .text()
                .fold(String::new(), |acc, t| format!("{acc}\n{t}"));
            Some(desc)
        })
        .unwrap_or_default();

    // Drop really short strings as they are probably not the description
    if description.len() < 50 {
        return Ok(None);
    }

    return Ok(Some(
        SPACE_NUKE.replace_all(&description, "$1").trim().into(),
    ));
}
