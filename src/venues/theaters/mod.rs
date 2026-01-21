pub mod hangarteatri;
pub mod miela;
pub mod rossetti;
pub mod verdi;

use anyhow::Result;
use reqwest::Client;

use crate::{
    dates::DateRange,
    events::{Event, EventVariants},
    venues::CacheManager,
};

pub(super) const SUMMARY_PROMPT: &str = "Accorcia la seguente descrizione di uno spettacolo o evento teatrale a non più di un paragrafo. Se la descrizione è già un paragrafo o meno, ripetila verbatim. Non andare a capo. Rispondi esclusivamente in testo semplice. Non usare markdown.";

pub async fn fetch(
    client: &Client,
    date_range: &DateRange,
    cache_manager: &mut CacheManager,
) -> Result<Vec<EventVariants>> {
    cache_manager.set_category("theater");
    let hangarteatri = cache_manager
        .get_or_fetch("hangarteatri", async || {
            hangarteatri::fetch(client, date_range).await
        })
        .await?;

    let miela = cache_manager
        .get_or_fetch("miela", async || miela::fetch(client, date_range).await)
        .await?;

    let rossetti = cache_manager
        .get_or_fetch("rossetti", async || {
            rossetti::fetch(client, date_range).await
        })
        .await?;

    let verdi = cache_manager
        .get_or_fetch("verdi", async || verdi::fetch(client, date_range).await)
        .await?;

    let mut events: Vec<Event> = [hangarteatri, miela, rossetti, verdi].concat();
    events.sort();

    let variants: Vec<EventVariants> = events
        .into_iter()
        .map(|event| EventVariants {
            id: event.title.clone(),
            title: event.title.clone(),
            category: event.category.clone(),
            description: event.description.clone(),
            events: vec![event],
        })
        .collect();

    Ok(variants)
}
