use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use anyhow::Result;
use convert_case::{Case, Casing};
use indicatif::{ProgressBar, ProgressFinish, ProgressIterator, ProgressStyle};
use reqwest::{Client, Response};
use serde_json::Value;

use crate::{events::Event, utils::PROGRESS_BAR_TEMPLATE, venues::cinemas::MovieGroup};

pub async fn fetch(client: &Client) -> Result<Vec<MovieGroup>> {
    // The Space's website is a Next.js app and contains absolutely zero functional
    // HTML without JavaScript. It does contain a JSON object that contains a bunch of content,
    // but only a few movies Thankfully, the movies are taken from an server API route that
    // returns a nice and convenient list of movies and all their metadata (except showtimes).
    // It's not really public, so sometimes it throws 401 Unauthorized errors. It's weirdly
    // inconsistent though, same API and same user agent can cause different statuses even
    // across a few seconds.
    // Also see for date-specific movie lists:
    // https://www.thespacecinema.it/api/microservice/showings/cinemas/1011/films?showingDate=2026-01-14T00:00:00&minEmbargoLevel=3&includesSession=true&includeSessionAttributes=true
    let url = "https://www.thespacecinema.it/api/microservice/showings/cinemas/1011/films?minEmbargoLevel=2&includesSession=false&includeSessionAttributes=true";

    let mut listings: Vec<Value> = Vec::new();
    let mut attempt = 1;
    while attempt <= 3 {
        let res = call_api(client, url).await;
        let status = res.status();
        let json = res.json::<Value>().await;
        match json {
            Ok(json) => {
                listings = json["result"].as_array().unwrap().to_vec();
                break;
            }
            Err(e) => {
                eprintln!(
                    "Status Code: {status}. Error: {e}. Attempt: {attempt} of 3. Retrying in 10 seconds..."
                );
                attempt += 1;
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
    }

    let progress = ProgressBar::new(listings.len() as u64)
        .with_style(ProgressStyle::with_template(PROGRESS_BAR_TEMPLATE).unwrap())
        .with_message("Fetching The Space")
        .with_finish(ProgressFinish::AndLeave);

    let mut movie_groups: HashMap<String, MovieGroup> = HashMap::new();

    for listing in listings.iter().progress_with(progress) {
        let (title, base_title, _) = super::clean_title(listing["filmTitle"].as_str().unwrap());
        let description = listing["synopsisShort"].as_str().unwrap();

        // These contain properties for the showings that we need for tags. However
        // each listing isn't a single showing, but rather a collection of all showings that are
        // done in the cinema. As such, each attribute is counted as a separate [Event]
        // For instance, a movie with 3D and LINGUA ORIGINALE is actually three
        // events, one in 2D, one in 3D and one in original language
        let attributes = listing["sessionAttributes"].as_array().unwrap();
        for attr in attributes {
            let tags = match attr["name"].as_str().unwrap() {
                "2D" => HashSet::new(),
                "3D" => HashSet::from(["3D".to_string()]),
                "LINGUA ORIGINALE" => HashSet::from(["Originale Sottotitolato".to_string()]),
                _ => continue,
            };
            let id = super::make_id(&base_title, &tags);

            let movie = Event::new(
                &title.from_case(Case::Sentence).to_case(Case::Title),
                HashSet::from_iter(["The Space".to_string()]),
                "Film",
            )
            .with_id(id)
            .with_tags(tags);

            movie_groups
                .entry(base_title.clone())
                .and_modify(|group| {
                    super::add_or_merge_to_group(group, &movie);
                    // Prioritize The Space descriptions
                    group.description = Some(description.to_string());
                })
                .or_insert_with(|| MovieGroup {
                    title: base_title.clone(),
                    description: Some(description.to_string()),
                    movies: HashSet::from([movie]),
                });
        }
    }

    return Ok(movie_groups.into_values().collect());
}

async fn call_api(client: &Client, url: &str) -> Response {
    return client
            .get(url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (X11; Linux x86_64; rv:146.0) Gecko/20100101 Firefox/146.0",
            )
            .header("Accept", "application/json")
            .header("Accept-Language", "en-US,en;q=0.5")
            // .header("Accept-Encoding", "gzip, deflate, br, zstd") // This one seems to cause JSON deserialization failure
            .header(
                "Referer",
                "https://www.thespacecinema.it/cinema/trieste/al-cinema",
            )
            .header("Content-Type", "application/json")
            .header("DNT", "1")
            .header("Sec-GPC", "1")
            .header("Connection", "keep-alive")
            .header("Cookie", "cinemaId=1011; cinemaName=trieste; analyticsCinemaName=Trieste; cinemaCurrency=EUR; isSecondaryMarket=false; hasLayout=true; OptanonConsent=isGpcEnabled=1&datestamp=Mon+Jan+19+2026+09%3A43%3A52+GMT%2B0100+(Ora+standard+dell%E2%80%99Europa+centrale)&version=6.30.0&isIABGlobal=false&hosts=&genVendors=&consentId=842f2ecc-b4fd-464a-ac38-bc9188b36abf&interactionCount=2&landingPath=NotLandingPage&groups=C0001%3A1%2CC0002%3A0%2CC0003%3A1%2CC0004%3A0%2CC0005%3A0&geolocation=%3B&AwaitingReconsent=false; OptanonAlertBoxClosed=2026-01-14T11:19:07.215Z; microservicesRefreshToken=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJqdGkiOiIxNzRlM2YxMS02ZDRmLTQ5YTAtOGNkNC01ODRlZGVmZGMxMDYiLCJDb3VudHJ5IjoiSVQiLCJJc0Fub255bW91cyI6IlRydWUiLCJuYmYiOjE3Njg4MTIyMzAsImV4cCI6MTc2OTQxNzAzMCwiaXNzIjoiQXV0aFByb2QifQ.VAN2ykvnCgs9I7T6oJwsTMuiesozb-pxpVnwApNWhBs; refreshTokenExpirationTime=2026-01-26T08%3A43%3A50Z; vuecinemas-it#lang=it-IT; ASP.NET_SessionId=mklkvbdkfi2yoocgiy3acwjj; hasLayout=true; microservicesToken=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJqdGkiOiIxNzRlM2YxMS02ZDRmLTQ5YTAtOGNkNC01ODRlZGVmZGMxMDYiLCJDb3VudHJ5IjoiSVQiLCJBdXRoIjoiMyIsIlNob3dpbmciOiIzIiwiQm9va2luZyI6IjMiLCJQYXltZW50IjoiMyIsIlBhcnRuZXIiOiIwIiwiTG95YWx0eSI6IjMiLCJDYW1wYWlnblRyYWNraW5nQ29kZSI6IiIsIkNsaWVudE5hbWUiOiIiLCJuYmYiOjE3Njg4MTIyMzAsImV4cCI6MTc2ODg1NTQzMCwiaXNzIjoiUHJvZCJ9.bJYIMejFyw_kGTgvyrecP82TI2PtBu-axU7fdzf1F4U; accessTokenExpirationTime=2026-01-19T20%3A43%3A50Z; __cflb=02DiuE2nAGMFu3TxDikSow61ukPuq1im3HgXQDd5bwV5W")
            .header("Sec-Fetch-Dest", "empty")
            .header("Sec-Fetch-Mode", "cors")
            .header("Sec-Fetch-Site", "same-origin")
            .header("Priority", "u=0")
            .header("Pragma", "no-cache")
            .header("Cache-Control", "no-cache")
            .header("TE", "trailers")
            .send()
            .await
            .unwrap();
}

/*
This command is copy-pasted as cURL from the network tab after accessing it in Firefox.
It consistently works in a browser to fetch movies from the The Space API. The session cookies
are timed.

curl 'https://www.thespacecinema.it/api/microservice/showings/cinemas/1011/films?minEmbargoLevel=2&includesSession=false&includeSessionAttributes=true' \
  --compressed \
  -H 'User-Agent: Mozilla/5.0 (X11; Linux x86_64; rv:146.0) Gecko/20100101 Firefox/146.0' \
  -H 'Accept: application/json' \
  -H 'Accept-Language: en-US,en;q=0.5' \
  -H 'Accept-Encoding: gzip, deflate, br, zstd' \
  -H 'Referer: https://www.thespacecinema.it/cinema/trieste/al-cinema' \
  -H 'Content-Type: application/json' \
  -H 'DNT: 1' \
  -H 'Sec-GPC: 1' \
  -H 'Connection: keep-alive' \
  -H 'Cookie: cinemaId=1011; cinemaName=trieste; analyticsCinemaName=Trieste; cinemaCurrency=EUR; isSecondaryMarket=false; hasLayout=true; OptanonConsent=isGpcEnabled=1&datestamp=Wed+Jan+14+2026+16%3A32%3A05+GMT%2B0100+(Ora+standard+dell%E2%80%99Europa+centrale)&version=6.30.0&isIABGlobal=false&hosts=&genVendors=&consentId=842f2ecc-b4fd-464a-ac38-bc9188b36abf&interactionCount=2&landingPath=NotLandingPage&groups=C0001%3A1%2CC0002%3A0%2CC0003%3A1%2CC0004%3A0%2CC0005%3A0&geolocation=%3B&AwaitingReconsent=false; OptanonAlertBoxClosed=2026-01-14T11:19:07.215Z; microservicesToken=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJqdGkiOiIxNzRlM2YxMS02ZDRmLTQ5YTAtOGNkNC01ODRlZGVmZGMxMDYiLCJDb3VudHJ5IjoiSVQiLCJBdXRoIjoiMyIsIlNob3dpbmciOiIzIiwiQm9va2luZyI6IjMiLCJQYXltZW50IjoiMyIsIlBhcnRuZXIiOiIwIiwiTG95YWx0eSI6IjMiLCJDYW1wYWlnblRyYWNraW5nQ29kZSI6IiIsIkNsaWVudE5hbWUiOiIiLCJuYmYiOjE3NjgzODk1NDMsImV4cCI6MTc2ODQzMjc0MywiaXNzIjoiUHJvZCJ9.y8Nbo_XAQPqFykSA5HLUv__c_m9Dx7moaYZVrbEiOxM; microservicesRefreshToken=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJqdGkiOiIxNzRlM2YxMS02ZDRmLTQ5YTAtOGNkNC01ODRlZGVmZGMxMDYiLCJDb3VudHJ5IjoiSVQiLCJJc0Fub255bW91cyI6IlRydWUiLCJuYmYiOjE3NjgzODk1NDMsImV4cCI6MTc2ODk5NDM0MywiaXNzIjoiQXV0aFByb2QifQ.BiJPBDrrEyofEsf4P0RVP9zxp9s8MEtwdJwTFI6xuHg; accessTokenExpirationTime=2026-01-14T23%3A19%3A03Z; refreshTokenExpirationTime=2026-01-21T11%3A19%3A03Z; __cflb=02DiuE2nAGMFu3TxDimEkxSkFKsPnnKB65aKFsMoLTtNQ; vuecinemas-it#lang=it-IT; ASP.NET_SessionId=mklkvbdkfi2yoocgiy3acwjj;' \
  -H 'Sec-Fetch-Dest: empty' \
  -H 'Sec-Fetch-Mode: cors' \
  -H 'Sec-Fetch-Site: same-origin' \
  -H 'Priority: u=0' \
  -H 'Pragma: no-cache' \
  -H 'Cache-Control: no-cache' \
  -H 'TE: trailers'
*/
