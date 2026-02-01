pub mod hangarteatri;
pub mod miela;
pub mod rossetti;
pub mod verdi;

use anyhow::Result;
use reqwest::Client;

use crate::{
    dates::DateRange,
    events::Event,
    venues::{CATEGORY_THEATRES, CacheManager},
};

pub async fn fetch(
    client: &Client,
    date_range: &DateRange,
    cache_manager: &mut CacheManager,
) -> Result<Vec<Event>> {
    cache_manager.set_category(&CATEGORY_THEATRES.to_lowercase());
    let hangarteatri = cache_manager
        .get_or_fetch("hangarteatri", async || {
            hangarteatri::fetch(client, date_range).await
        })
        .await?
        .unwrap_or_else(Vec::new);

    let miela = cache_manager
        .get_or_fetch("miela", async || miela::fetch(client, date_range).await)
        .await?
        .unwrap_or_else(Vec::new);

    let rossetti = cache_manager
        .get_or_fetch("rossetti", async || {
            rossetti::fetch(client, date_range).await
        })
        .await?
        .unwrap_or_else(Vec::new);

    let verdi = cache_manager
        .get_or_fetch("verdi", async || verdi::fetch(client, date_range).await)
        .await?
        .unwrap_or_else(Vec::new);

    let mut events: Vec<Event> = [hangarteatri, miela, rossetti, verdi].concat();
    events.sort();

    Ok(events)
}
