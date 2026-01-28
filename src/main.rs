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
    events::Event,
    inference::InferenceService,
    venues::{CacheManager, cinemas, custom, theaters},
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
        short = 'R',
        long,
        help = "Forcefully recreate the cache. Does nothing without --cache"
    )]
    rebuild_cache: bool,

    #[arg(
        short,
        long,
        help = "Individual venues to rebuild, as a space-separate list of snake_case names. Does nothing without --cache"
    )]
    rebuild_venues: Option<String>,

    #[arg(short, long, help = "Save markdown and HTML working files")]
    save_debug: bool,
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
    let maybe_filename = args.save_debug.then_some(filename.as_str()).or(None);

    let events = fetch_events(&current_week, &args).await;
    let mut events_by_category: HashMap<String, Vec<Event>> = HashMap::new();

    for event in events {
        events_by_category
            .entry(event.category.clone())
            .or_insert_with(Vec::new)
            .push(event);
    }

    // TODO: Move file saving out of write_html
    rendering::write_html(events_by_category, &current_week, maybe_filename)?;

    println!("Done!");
    Ok(())
}

async fn fetch_events(date_range: &DateRange, args: &Args) -> Vec<Event> {
    println!("Fetching events...");
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:146.0) Gecko/20100101 Firefox/146.0")
        .build()
        .unwrap();

    let mut cache_manager = CacheManager::new(
        "",
        args.cache,
        args.rebuild_cache,
        args.rebuild_venues.clone().map_or(vec![], |list| {
            list.split_whitespace().map(|s| s.to_string()).collect()
        }),
    );

    let movies = cinemas::fetch(&client, &date_range, &mut cache_manager)
        .await
        .unwrap();
    let shows = theaters::fetch(&client, &date_range, &mut cache_manager)
        .await
        .unwrap();
    let custom = custom::fetch("custom_events.toml", &date_range).unwrap();

    return [movies, shows, custom].concat();
}
