pub mod hangarteatri;
pub mod miela;
pub mod rossetti;
pub mod verdi;

use anyhow::Result;
use reqwest::Client;

use crate::{dates::DateRange, events::Event};

pub async fn fetch(client: &Client, current_week: &DateRange) -> Result<Vec<Event>> {
    let mut events = Vec::new();
    events.extend(hangarteatri::fetch(client, current_week).await?);
    events.extend(miela::fetch(client, current_week).await?);
    events.extend(rossetti::fetch(client, current_week).await?);
    events.extend(verdi::fetch(client, current_week).await?);
    events.sort();

    Ok(events)
}
