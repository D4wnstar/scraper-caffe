mod dates;
mod events;
mod summarize;
mod utils;
mod venues;

use std::collections::HashMap;

use anyhow::Result;
use chrono::Days;
use clap::Parser;
use headless_chrome::LaunchOptions;
use reqwest::{self, Client};

use crate::{
    dates::DateRange,
    events::Event,
    venues::{cinemas, custom, theaters},
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = 7)]
    days: u64,

    #[arg(short, long)]
    save_debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let today = chrono::Local::now().date_naive();
    let in_a_week = today.checked_add_days(Days::new(args.days - 1)).unwrap();
    let current_week = DateRange::new(today, in_a_week);

    let _ = std::fs::create_dir("./qsat");
    let filename = format!(
        "SettimanaTrieste_{}_{}",
        today.format("%d-%m"),
        in_a_week.format("%d-%m")
    );
    let maybe_filename = args.save_debug.then_some(filename.as_str()).or(None);

    let events = fetch_events(&current_week).await;
    let markdown = write_markdown(events, &current_week, maybe_filename);
    let html = write_html(&markdown, maybe_filename);
    print_to_pdf(&html, &filename);

    println!("Done!");
    Ok(())
}

async fn fetch_events(time_period: &DateRange) -> Vec<Event> {
    println!("Fetching events...");
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:146.0) Gecko/20100101 Firefox/146.0")
        .build()
        .unwrap();

    let movies = cinemas::fetch(&client, &time_period).await.unwrap();
    let shows = theaters::fetch(&client, &time_period).await.unwrap();
    let custom = custom::fetch("custom_events.toml", &time_period).unwrap();

    return [movies, shows, custom].concat();
}

fn write_markdown(events: Vec<Event>, time_period: &DateRange, filename: Option<&str>) -> String {
    println!("Writing markdown...");
    let mut events_by_category: HashMap<String, Vec<Event>> = HashMap::new();

    for event in events {
        events_by_category
            .entry(event.category.clone())
            .or_insert_with(Vec::new)
            .push(event);
    }

    let mut markdown = String::new();
    markdown += "# QUESTA SETTIMANA A TRIESTE\n";
    markdown += &format!(
        "Questa è una lista di buona parte dei film e spettacoli teatrali a Trieste dal {} al {}. Prendeteli come spunto per nuove uscite!\n\n",
        time_period.start_date.format("%d/%m"),
        time_period.end_date.format("%d/%m")
    );
    markdown += "(La lista è generata automaticamente e potrebbe contenere errori o duplicati.)\n";
    markdown += "\n---\n\n";

    let mut categories: Vec<&str> = events_by_category.keys().map(|c| c.as_str()).collect();
    categories.sort();
    for category in categories {
        markdown += &format!("\n## {}\n", category.to_uppercase());

        for event in events_by_category.get(category).unwrap() {
            markdown += &format!("- {event}\n");

            if let Some(desc) = &event.description {
                markdown += &format!("  - {desc}\n");
            }
        }
    }

    if let Some(filename) = filename {
        std::fs::write(format!("./qsat/{filename}.md"), &markdown).unwrap();
    }

    return markdown;
}

fn write_html(markdown: &str, filename: Option<&str>) -> String {
    println!("Converting to HTML...");
    // For some reason, the # character hard stops the print-to-PDF process at that location
    let html = comrak::markdown_to_html(&markdown, &comrak::Options::default()).replace("#", "");

    if let Some(filename) = filename {
        std::fs::write(format!("./qsat/{filename}.html"), &html).unwrap();
    }

    return html;
}

fn print_to_pdf(html: &str, filename: &str) {
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
}
