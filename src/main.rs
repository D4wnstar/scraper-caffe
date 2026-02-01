mod dates;
mod events;
mod inference;
mod rendering;
mod utils;
mod venues;

use std::{collections::HashMap, env};

use anyhow::Result;
use chrono::Days;
use clap::Parser;
use lazy_static::lazy_static;
use reqwest::{self, Client};

use crate::{
    dates::DateRange,
    events::{Category, Event},
    inference::InferenceService,
    venues::{CacheManager, cinemas, custom, libraries, theaters},
};

lazy_static! {
    static ref INFERENCE_SERVICE: InferenceService = InferenceService::new(
        &env::var("INFERENCE_API_URL").unwrap_or_default(),
        &env::var("INFERENCE_API_KEY").unwrap_or_default(),
        &env::var("INFERENCE_MODEL").unwrap_or_default(),
        Client::new()
    );
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(
        short,
        long,
        default_value_t = 7,
        help = "The number of days to fetch events for, today included"
    )]
    days: u64,

    #[arg(
        short,
        long,
        help = "Reuse cached events instead of fetching. If cache doesn't exist yet, fetch normally and create it"
    )]
    cache: bool,

    #[arg(
        short,
        long,
        help = "Individual venues to skip, as a space-separate list of snake_case names"
    )]
    skip_venues: Option<String>,

    #[arg(
        short,
        long,
        help = "Like skip_venues, but to forcefully rebuild the cache for those venues. Does nothing without --cache"
    )]
    rebuild_venues: Option<String>,

    #[arg(
        short = 'R',
        long,
        help = "Forcefully rebuild the entire cache. Does nothing without --cache"
    )]
    rebuild_cache: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    dotenv::dotenv().ok();

    let today = chrono::Local::now().date_naive();
    let in_a_week = today + Days::new(args.days - 1);
    let current_week = DateRange::new(today, in_a_week);

    drop(std::fs::create_dir("qsat"));
    let filename = format!(
        "SettimanaTrieste_{}_{}",
        today.format("%d-%m"),
        in_a_week.format("%d-%m")
    );

    let categories = fetch_events(&current_week, args).await;
    let html = rendering::render_to_html(categories, &current_week)?;
    std::fs::write(format!("qsat/{filename}.html"), &html)?;

    println!("Done!");
    Ok(())
}

async fn fetch_events(date_range: &DateRange, args: Args) -> Vec<Category> {
    println!("Fetching events...");
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:146.0) Gecko/20100101 Firefox/146.0")
        .build()
        .unwrap();

    let mut cache_manager = CacheManager::new(
        "",
        args.cache,
        args.rebuild_cache,
        args.rebuild_venues.map_or_else(Vec::new, |list| {
            list.split_whitespace().map(|s| s.to_string()).collect()
        }),
        args.skip_venues.map_or_else(Vec::new, |list| {
            list.split_whitespace().map(|s| s.to_string()).collect()
        }),
    );

    let mut events_by_category: HashMap<String, Vec<Event>> = HashMap::new();

    let movies = cinemas::fetch(&client, &date_range, &mut cache_manager)
        .await
        .unwrap();
    events_by_category.insert("Film".to_string(), movies);

    let shows = theaters::fetch(&client, &date_range, &mut cache_manager)
        .await
        .unwrap();
    events_by_category.insert("Teatri".to_string(), shows);

    let libraries = libraries::fetch(&client, date_range, &mut cache_manager)
        .await
        .unwrap();
    events_by_category.insert("Librerie".to_string(), libraries);

    // Merge custom events with existing categories
    let custom = custom::fetch("custom_events.toml", &date_range).unwrap();
    for event in custom {
        events_by_category
            .entry(event.category.clone())
            .or_insert_with(Vec::new)
            .push(event);
    }

    let mut categories: Vec<Category> = events_by_category
        .into_iter()
        .map(|(name, events)| Category { name, events })
        .collect();
    categories.sort_by(|a, b| a.name.cmp(&b.name));

    return categories;
}
