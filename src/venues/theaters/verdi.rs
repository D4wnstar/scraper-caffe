use std::{collections::HashSet, time::Duration};

use anyhow::Result;
use chrono::NaiveDate;
use indicatif::{ProgressBar, ProgressFinish, ProgressIterator, ProgressStyle};
use reqwest::Client;
use scraper::{Html, Selector};

use crate::{
    INFERENCE_SERVICE,
    dates::{DateRange, DateSet, TimeFrame, italian_month_to_number},
    events::{Event, Location},
    inference::SUMMARY_PROMPT,
    utils::PROGRESS_BAR_TEMPLATE,
    venues::CATEGORY_THEATRES,
};

pub async fn fetch(client: &Client, date_range: &DateRange) -> Result<Vec<Event>> {
    let mut events: HashSet<Event> = HashSet::new();

    let url = "https://www.teatroverdi-trieste.com/it/calendario-spettacoli/";
    let html_body = client
        .get(url)
        .send()
        .await
        .inspect_err(|e| println!("GET request failed: {e}"))?
        .text()
        .await?;

    let document = Html::parse_document(&html_body);
    let shows_sel = Selector::parse("ul.spettacolo-list div.list-text").unwrap();
    let link_sel = Selector::parse("h2.spettacolo-list-title > a").unwrap();
    let date_sel = Selector::parse("span.spettacolo-list-date > strong").unwrap();

    let show_count = document.select(&shows_sel).count();
    let progress = ProgressBar::new(show_count as u64)
        .with_style(ProgressStyle::with_template(PROGRESS_BAR_TEMPLATE).unwrap())
        .with_message("Fetching Verdi")
        .with_finish(ProgressFinish::AndLeave);

    for show in document.select(&shows_sel).progress_with(progress) {
        let link_el = show.select(&link_sel).next();
        let date_el = show.select(&date_sel).next();
        if link_el.is_none() || date_el.is_none() {
            continue;
        }

        let title = link_el
            .and_then(|el| el.text().next())
            .map(|t| t.to_string())
            .expect("Each link element should have text");

        let event_url = link_el.unwrap().attr("href").unwrap();
        let location = Location::new("Verdi", Some(event_url.to_string()));
        let locations = HashSet::from_iter([location]);

        let (description, summary, dates) = get_description_and_dates(client, event_url)
            .await
            .unwrap_or((None, None, DateSet::today()));

        // Events are chronological: stop as soon as one is beyond the given range
        if !dates.as_range().overlaps(&date_range) {
            break;
        }

        let time_frame = TimeFrame::Dates(dates);

        let event = Event::new(&title, locations, CATEGORY_THEATRES)
            .with_time_frame(Some(time_frame))
            .with_description(description)
            .with_summary(summary);

        events.insert(event);

        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    return Ok(events.into_iter().collect());
}

fn parse_date(date_str: &str) -> Option<NaiveDate> {
    let split: Vec<&str> = date_str.split_whitespace().collect();
    let day = split[0].parse::<u32>().ok()?;
    let month = italian_month_to_number(split[1])?;
    let year = split[2].parse::<i32>().ok()?;

    return NaiveDate::from_ymd_opt(year, month, day);
}

async fn get_description_and_dates(
    client: &Client,
    url: &str,
) -> Result<(Option<String>, Option<String>, DateSet)> {
    let desc_sel = Selector::parse("section.mnk-block.spettacolo-block:not([id]) div").unwrap();
    let date_sel = Selector::parse("span.spettacolo-ticket-date").unwrap();

    let html_body = client.get(url).send().await?.text().await?;
    let document = Html::parse_document(&html_body);
    let desc_els = document.select(&desc_sel);
    let date_els = document.select(&date_sel);

    let mut dates: Vec<NaiveDate> = date_els
        .filter_map(|el| el.text().next().and_then(|t| parse_date(t)))
        .collect();
    dates.dedup();
    let dateset = DateSet::new(dates).unwrap();

    if desc_els.clone().count() == 0 {
        println!("No desc_els");
        return Ok((None, None, dateset));
    }

    let description = desc_els.fold(String::new(), |acc, el| {
        let text = el
            .text()
            .filter(|t| !t.trim().is_empty())
            .fold(String::new(), |acc, t| format!("{acc}. {t}"))
            .trim()
            .replace("\n", "");
        format!("{acc}. {text}",)
    });

    let prompt = format!("{SUMMARY_PROMPT}\n\n{description}");
    let summary = INFERENCE_SERVICE
        .infer(&prompt)
        .await
        .inspect_err(|err| eprintln!("Failed to generate summary: {err}"))
        .ok();

    return Ok((Some(description), summary, dateset));
}
