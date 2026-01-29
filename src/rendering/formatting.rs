use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;

use crate::{dates::TimeFrame, events::Event, rendering::TemplateEvent};

/// Films have multiple variants that are saved as different [Event]s, but should visually
/// be displayed as the same event. For instance, showings of a movie in 2D, in 3D and
/// in original language. This function combines similar movie showings into one
/// [TemplateEvent].
pub(super) fn preprocess_films(events: Vec<Event>) -> Vec<TemplateEvent> {
    // Group by title
    let mut groups: HashMap<String, Vec<Event>> = HashMap::new();
    for event in &events {
        groups
            .entry(event.title.clone())
            .or_insert_with(Vec::new)
            .push(event.clone());
    }

    let mut results = Vec::new();

    for (title, events) in groups.into_iter() {
        // Put base variant first
        // events.sort_by(|a, b| a.tags.len().cmp(&b.tags.len()));
        // Copy description and summary over
        let description = events[0].description.clone();
        let summary = events[0].summary.clone();

        // Collect all unique tags available for this movie
        let mut all_tags: Vec<String> = events
            .iter()
            .fold(HashSet::new(), |mut acc, e| {
                acc.extend(e.tags.iter().cloned());
                acc
            })
            .into_iter()
            .collect();
        all_tags.sort();

        // Aggregate dates
        // Map: Date -> Set of tags available on that date
        let mut date_map: HashMap<NaiveDate, HashSet<String>> = HashMap::new();
        for e in &events {
            if let Some(TimeFrame::Dates(dates)) = &e.time_frame {
                for d in dates.dates() {
                    date_map
                        .entry(*d)
                        .or_default()
                        .extend(e.tags.iter().cloned());
                }
            }
        }

        // Aggregate locations
        // Map: Location -> Set of tags available at that location
        let mut loc_map: HashMap<String, HashSet<String>> = HashMap::new();
        for e in &events {
            for loc in &e.locations {
                loc_map
                    .entry(loc.clone())
                    .or_default()
                    .extend(e.tags.iter().cloned());
            }
        }

        // Format locations
        // Result: ["Venue A", "Venue B (also 3D)"]
        let mut sorted_locs: Vec<String> = loc_map.keys().cloned().collect();
        sorted_locs.sort();

        let formatted_locations: Vec<String> = sorted_locs
            .into_iter()
            .map(|loc| {
                let tags = &loc_map[&loc];
                if tags.is_empty() {
                    loc
                } else {
                    let tag_str = tags
                        .iter()
                        .map(|t| format!("{}", t))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{} ({})", loc, tag_str)
                }
            })
            .collect();

        // Format TimeFrame
        // Result: "il 14/02, 15/02 (anche 3D), 16/02 (anche Originale)"
        let mut sorted_dates: Vec<NaiveDate> = date_map.keys().cloned().collect();
        sorted_dates.sort();

        let formatted_time_frame = if sorted_dates.is_empty() {
            None
        } else {
            let parts: Vec<String> = sorted_dates
                .into_iter()
                .map(|d| {
                    let tags = &date_map[&d];
                    let date_str = d.format("%d/%m").to_string();
                    if tags.is_empty() {
                        date_str
                    } else {
                        let tag_str = tags
                            .iter()
                            .map(|t| format!("{}", t))
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("{} ({})", date_str, tag_str)
                    }
                })
                .collect();

            Some(super::fmt_date_parts(parts))
        };

        results.push(TemplateEvent {
            title,
            tags: all_tags,
            locations: formatted_locations,
            time_frame: formatted_time_frame,
            summary,
            description,
        });
    }

    // Sort movies alphabetically
    results.sort_by(|a, b| a.title.cmp(&b.title));
    results
}
