mod formatting;

use anyhow::Result;
use handlebars::{Context, Handlebars, Helper, HelperDef, HelperResult, Output, RenderContext};
use serde::{Deserialize, Serialize};

use crate::{
    dates::{DateRange, DateSet, TimeFrame},
    events::{Category, Event},
};

#[derive(Serialize, Deserialize)]
struct TemplateData {
    start_date: String,
    end_date: String,
    current_date: String,
    categories: Vec<TemplateCategory>,
}

#[derive(Serialize, Deserialize)]
struct TemplateCategory {
    name: String,
    events: Vec<TemplateEvent>,
}

impl From<Category> for TemplateCategory {
    fn from(cat: Category) -> Self {
        let events = match cat.name.as_str() {
            "Film" => formatting::preprocess_films(cat.events),
            _ => cat.events.into_iter().map(TemplateEvent::from).collect(),
        };

        Self {
            name: cat.name,
            events,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct TemplateEvent {
    pub title: String,
    pub tags: Vec<String>,
    pub locations: Vec<String>,
    pub time_frame: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
}

impl From<Event> for TemplateEvent {
    fn from(value: Event) -> Self {
        let mut tags: Vec<String> = value.tags.into_iter().collect();
        tags.sort();
        let mut locations: Vec<String> = value.locations.into_iter().collect();
        locations.sort();
        let time_frame = value.time_frame.map(|tf| match tf {
            TimeFrame::Dates(set) => fmt_date_set(&set),
            TimeFrame::Period(range) => fmt_date_range(&range),
        });

        Self {
            title: value.title,
            tags,
            locations,
            time_frame,
            summary: value.summary,
            description: value.description,
        }
    }
}

pub fn render_to_html(categories: Vec<Category>, date_range: &DateRange) -> Result<String> {
    println!("Converting to HTML...");
    let data = TemplateData {
        start_date: date_range.start.format("%d/%m").to_string(),
        end_date: date_range.end.format("%d/%m").to_string(),
        current_date: chrono::Local::now().format("%d/%m/%Y").to_string(),
        categories: categories.into_iter().map(|c| c.into()).collect(),
    };

    let mut handlebars = Handlebars::new();
    handlebars.register_template_file("qsat", "src/rendering/template.html")?;
    handlebars.register_helper("uppercase", Box::new(Uppercase));
    handlebars.register_helper("join", Box::new(Join));

    let html = handlebars.render("qsat", &data)?;

    Ok(html)
}

fn fmt_date_set(set: &DateSet) -> String {
    let parts: Vec<String> = set
        .dates()
        .iter()
        .map(|d| d.format("%d/%m").to_string())
        .collect();

    fmt_date_parts(parts)
}

fn fmt_date_range(range: &DateRange) -> String {
    format!(
        "dal {} al {}",
        range.start.format("%d/%m/%Y"),
        range.end.format("%d/%m/%Y")
    )
}

/// Helper to format a list of strings into an Italian enumeration (e.g., "il A, B e C")
fn fmt_date_parts(mut parts: Vec<String>) -> String {
    if parts.is_empty() {
        return String::new();
    }
    if parts.len() == 1 {
        return format!("il {}", parts[0]);
    }

    let last = parts.pop().unwrap();
    let init = parts.join(", ");
    format!("il {} e {}", init, last)
}

struct Uppercase;
impl HelperDef for Uppercase {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let text = h
            .param(0)
            .and_then(|v| v.value().as_str())
            .unwrap_or_default();
        out.write(&text.to_uppercase())?;
        Ok(())
    }
}

struct Join;
impl HelperDef for Join {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let array = h.param(0).unwrap().value().as_array();
        let sep = h.param(0).and_then(|v| v.value().as_str()).unwrap_or(", ");

        if let Some(vec) = array {
            let strings: Vec<String> = vec
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();
            out.write(&strings.join(sep))?;
        }

        Ok(())
    }
}
