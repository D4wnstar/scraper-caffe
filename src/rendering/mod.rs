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
        Self {
            name: cat.name,
            events: cat.events.into_iter().map(TemplateEvent::from).collect(),
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
    if set.dates().len() == 1 {
        format!("il {}", set.first().format("%d/%m"))
    } else {
        let str = set
            .dates()
            .iter()
            .enumerate()
            .fold("il ".to_string(), |acc, (i, date)| {
                let str = date.format("%d/%m");
                if i == set.dates().len() - 1 {
                    format!("{acc} e {str}")
                } else if i == set.dates().len() - 2 {
                    format!("{acc} {str}")
                } else {
                    format!("{acc} {str}, ")
                }
            });

        format!("{str}")
    }
}

fn fmt_date_range(range: &DateRange) -> String {
    format!(
        "dal {} al {}",
        range.start.format("%d/%m/%Y"),
        range.end.format("%d/%m/%Y")
    )
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
