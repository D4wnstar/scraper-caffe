mod dates;
mod events;
mod venues;

use std::collections::HashMap;

use anyhow::Result;
use chrono::Days;
use headless_chrome::LaunchOptions;
use reqwest::{self, Client};

use crate::{
    dates::DateRange,
    events::Event,
    venues::{cinemas, custom, theaters},
};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder().user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36").build().unwrap();
    let today = chrono::Local::now().date_naive();
    let in_a_week = today.checked_add_days(Days::new(7)).unwrap();
    let current_week = DateRange::new(today, in_a_week);
    let filename = format!(
        "SettimanaTrieste_{}_{}",
        today.format("%d-%m"),
        in_a_week.format("%d-%m")
    );

    println!("Fetching events...");
    let movies = cinemas::fetch(&client).await?;
    let shows = theaters::fetch(&client, &current_week).await?;
    let custom = custom::fetch("custom_events.toml", &current_week)?;

    let events_by_category = group_by_category([movies, shows, custom].concat());
    let mut categories: Vec<&str> = events_by_category.keys().map(|s| s.as_str()).collect();
    categories.sort();

    println!("Writing markdown...");
    let mut output = String::new();
    output += "# QUESTA SETTIMANA A TRIESTE\n";
    output += &format!(
        "Questa è una lista di buona parte dei film e spettacoli teatrali a Trieste dal {} al {}. Prendeteli come spunto per nuove uscite!\n\n",
        today.format("%d/%m"),
        in_a_week.format("%d/%m")
    );
    output += "(La lista è generata automaticamente e potrebbe contenere errori o duplicati.)\n";
    output += "\n---\n\n";

    for category in categories {
        let events = events_by_category.get(category).unwrap();
        output += &format!("\n### {}\n", category.to_uppercase());
        for event in events {
            output += &format!("- {event}\n");
        }
    }

    let _ = std::fs::create_dir("./qsat");
    // std::fs::write(format!("./qsat/{filename}.md"), &output).unwrap();

    println!("Converting to HTML...");
    let mut html = comrak::markdown_to_html(&output, &comrak::Options::default());
    html = html.replace("#", ""); // For some reason, the # character hard stops the print-to-PDF process at that location
    // std::fs::write(format!("./qsat/{filename}.html"), &html).unwrap();

    println!("Printing to PDF...");
    // This will download Chrome binaries from the web
    let browser =
        headless_chrome::Browser::new(LaunchOptions::default_builder().path(None).build().unwrap())
            .unwrap();
    let tab = browser.new_tab().unwrap();
    let tab = tab
        .navigate_to(&format!("data:text/html;charset=utf-8,{}", html))
        .unwrap()
        .wait_until_navigated()
        .unwrap();
    let pdf_bytes = tab.print_to_pdf(None).unwrap();
    std::fs::write(format!("./qsat/{filename}.pdf"), pdf_bytes).unwrap();

    println!("Done!");
    Ok(())
}

fn group_by_category(events: Vec<Event>) -> HashMap<String, Vec<Event>> {
    let mut events_by_category: HashMap<String, Vec<Event>> = HashMap::new();

    for event in events {
        events_by_category
            .entry(event.category.clone())
            .or_insert_with(Vec::new)
            .push(event);
    }

    return events_by_category;
}
