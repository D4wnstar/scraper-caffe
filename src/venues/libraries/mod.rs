mod lovat;
use anyhow::Result;
use reqwest::Client;

use crate::{
    dates::DateRange,
    events::Event,
    venues::{CATEGORY_BOOKSTORES, CacheManager},
};

pub async fn fetch(
    client: &Client,
    date_range: &DateRange,
    cache_manager: &mut CacheManager,
) -> Result<Vec<Event>> {
    cache_manager.set_category(&CATEGORY_BOOKSTORES.to_lowercase());
    let lovat = cache_manager
        .get_or_fetch("lovat", async || lovat::fetch(client, date_range).await)
        .await?
        .unwrap_or_else(Vec::new);

    let mut events: Vec<Event> = [lovat].concat();
    events.sort();

    Ok(events)
}
