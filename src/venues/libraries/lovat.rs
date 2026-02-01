use std::collections::HashSet;

use anyhow::Result;
use chrono::NaiveDate;
use convert_case::Case;
use indicatif::{ProgressBar, ProgressFinish, ProgressIterator, ProgressStyle};
use reqwest::Client;
use scraper::{Html, Selector};

use crate::{
    INFERENCE_SERVICE,
    dates::{DateRange, DateSet, TimeFrame},
    events::Event,
    inference::SUMMARY_PROMPT,
    utils::PROGRESS_BAR_TEMPLATE,
    venues::{CATEGORY_BOOKSTORES, StandardCasing},
};

pub async fn fetch(client: &Client, date_range: &DateRange) -> Result<Vec<Event>> {
    let mut events: HashSet<Event> = HashSet::new();

    let url = "https://www.librerielovat.com/eventi/";
    let html_body = client
        .get(url)
        .send()
        .await
        .inspect_err(|e| println!("GET request failed: {e}"))?
        .text()
        .await?;

    let document = Html::parse_document(&html_body);
    let next_events_sel = Selector::parse("div#c233 > div.calendarize").unwrap();
    let event_sel = Selector::parse("div.media.calendarize-item").unwrap();
    let link_sel = Selector::parse("a.stretched-link").unwrap();
    let category_sel = Selector::parse("span.category span.label").unwrap();
    let date_sel = Selector::parse("h4").unwrap();

    let next_events_el = document.select(&next_events_sel).next().unwrap();

    let event_count = next_events_el.select(&event_sel).count();
    let progress = ProgressBar::new(event_count as u64)
        .with_style(ProgressStyle::with_template(PROGRESS_BAR_TEMPLATE).unwrap())
        .with_message("Fetching Lovat")
        .with_finish(ProgressFinish::AndLeave);

    for event_el in next_events_el.select(&event_sel).progress_with(progress) {
        // Lovat has a location outside of Trieste too
        // Make sure to filter only for local events
        let is_local = event_el
            .select(&category_sel)
            .filter_map(|el| el.text().next())
            .find(|t| t.to_lowercase() == "trieste")
            .is_some();
        if !is_local {
            continue;
        }

        let link_el = event_el
            .select(&link_sel)
            .next()
            .expect("Each event card should have a link");
        let title = link_el
            .text()
            .next()
            .map(|t| t.trim().standardize_case(Some(Case::Title)))
            .expect("Each event link should have a title");
        let location = HashSet::from_iter(["Lovat".to_string()]);
        let date = event_el
            .select(&date_sel)
            .next()
            .and_then(|el| el.text().next())
            .and_then(|t| parse_date(t))
            .unwrap();
        if !date.as_range().overlaps(date_range) {
            continue;
        }
        let time_frame = TimeFrame::Dates(date);
        let href = link_el.attr("href").unwrap();
        let event_url = format!("https://www.librerielovat.com{href}");
        let (description, summary) = get_description(client, &event_url, &title)
            .await
            .unwrap_or((None, None));

        let event = Event::new(&title, location, CATEGORY_BOOKSTORES)
            .with_time_frame(Some(time_frame))
            .with_description(description)
            .with_summary(summary);

        events.insert(event);
    }

    Ok(events.into_iter().collect())
}

/// Parses a date string from Lovat data and return a DateSet.
/// This function handles the format: "Ven 30/01/2026" (weekday DD/MM/YY).
fn parse_date(date_str: &str) -> Option<DateSet> {
    let parts: Vec<&str> = date_str.split_whitespace().collect();
    if parts.len() != 2 {
        return None;
    }
    // parts[0] is the weekday, which we don't care about
    // parts[1] is the actual date
    let date_parts: Vec<u32> = parts[1]
        .split("/")
        .filter_map(|p| p.parse::<u32>().ok())
        .collect();
    if date_parts.len() != 3 {
        return None;
    }
    // date is in DD/MM/YY so 0 is day, 1 is month, 2 is year
    let date =
        NaiveDate::from_ymd_opt((2000 + date_parts[2]) as i32, date_parts[1], date_parts[0])?;

    return Some(DateSet::new(vec![date]).unwrap());
}

async fn get_description(
    client: &Client,
    url: &str,
    title: &str,
) -> Result<(Option<String>, Option<String>)> {
    let html_body = client
        .get(url)
        .send()
        .await
        .inspect_err(|e| println!("GET request failed: {e}"))?
        .text()
        .await?;

    let document = Html::parse_document(&html_body);
    let desc_sel = Selector::parse("div.text").unwrap();
    let description = document.select(&desc_sel).next().map(|el| {
        // The title is the author, which is important for the description to make sense
        el.text()
            .fold(title.to_string(), |acc, new| format!("{acc}\n{new}"))
            .trim()
            .to_string()
    });

    if description.is_none() {
        return Ok((None, None));
    }

    let description = description.unwrap();
    let prompt = format!("{SUMMARY_PROMPT}\n\n{description}");
    let summary = INFERENCE_SERVICE
        .infer(&prompt)
        .await
        .inspect_err(|err| eprintln!("Failed to generate summary: {err}"))
        .ok();

    return Ok((Some(description), summary));
}
