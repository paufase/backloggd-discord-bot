use chrono::{DateTime, TimeZone, Utc};
use dotenv::dotenv;
use html_escape::decode_html_entities_to_string;
use reqwest::header::{HeaderMap, HeaderValue};
use scraper::Html;
use serde::Deserialize;
use serenity::async_trait;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use serenity::utils::MessageBuilder;
use std::fmt::Display;
use std::str::FromStr;
use std::time::Duration;
use std::{env, fs};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, context: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        loop {
            let logs = get_logs().await;
            println!("Checking logs...");
            for log in logs {
                let cover = get_cover(log.game_url.as_str()).await;
                let channel_id = ChannelId(815716163102179350);
                channel_id
                    .send_message(&context.http, |m| {
                        m.embed(|e| {
                            e.colour(0xbcdefa)
                                .title(
                                    MessageBuilder::new()
                                        .push(
                                            log.game_name.to_string()
                                                + " "
                                                + &*get_stars_text(log.rating),
                                        )
                                        .build(),
                                )
                                .url("https://www.backloggd.com".to_owned() + &*log.game_url)
                                .field(
                                    localize_status(&log.status).to_string()
                                        + " <t:".to_string().as_str()
                                        + get_timestamp(log.timestamp.as_str())
                                            .to_string()
                                            .as_str()
                                        + ":R>",
                                    "",
                                    false,
                                )
                                .thumbnail(
                                    "https://images.igdb.com/igdb/image/upload/t_cover_big/"
                                        .to_owned()
                                        + cover.trim()
                                        + ".png",
                                )
                                .author(|a| {
                                    a.name(log.username.clone())
                                        .url(
                                            "https://www.backloggd.com/u/".to_owned()
                                                + &*log.username,
                                        )
                                        .icon_url(log.avatar_url)
                                })
                        })
                    })
                    .await
                    .expect("TODO: panic message");
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    // Configure the client with your Discord bot token in the environment.
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}

async fn get_cover(game_name: &str) -> String {
    let access_token = env::var("TWITCH_ACCESS_TOKEN").unwrap();
    let client_id = env::var("TWITCH_CLIENT_ID").unwrap();
    let games_api_url = "https://api.igdb.com/v4/games/";
    let cover_api_url = "https://api.igdb.com/v4/covers/";

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", access_token)).unwrap(),
    );
    headers.insert("Client-ID", HeaderValue::from_str(&client_id).unwrap());

    let game_response = client
        .post(games_api_url)
        .headers(headers.clone())
        .body(
            "fields cover; where url = \"https://www.igdb.com".to_owned()
                + &game_name[..game_name.len() - 1]
                + "\";",
        )
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let game_id = serde_json::from_str::<Vec<Game>>(&game_response)
        .unwrap()
        .get(0)
        .unwrap()
        .id;
    let cover_response = client
        .post(cover_api_url)
        .headers(headers)
        .body("fields image_id; where game =".to_owned() + game_id.to_string().as_str() + ";")
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let image_id = serde_json::from_str::<Vec<Cover>>(&cover_response)
        .unwrap()
        .get(0)
        .unwrap()
        .image_id
        .to_string();
    image_id
}

#[derive(Debug, Deserialize)]
struct Game {
    id: i32,
}

#[derive(Debug, Deserialize)]
struct Cover {
    image_id: String,
}

async fn get_logs() -> Vec<Log> {
    let mut logs = Vec::new();
    let binding = fs::read_to_string("src/users.txt").unwrap();
    let usernames = binding.split("\n").collect::<Vec<&str>>();
    for username in usernames {
        for log in print_logs(&username).await {
            logs.push(log);
        }
    }
    return logs;
}

async fn print_logs(username: &str) -> Vec<Log> {
    let mut logs = Vec::new();
    let response = reqwest::get(
        "https://www.backloggd.com/u/".to_owned() + username + "/activity/you/played/",
    )
    .await
    .unwrap()
    .text()
    .await
    .unwrap();
    let document = Html::parse_document(&response);
    let log_selector = scraper::Selector::parse("div.row.activity").unwrap();
    let username_selector = scraper::Selector::parse("h3.mr-2.mb-0.main-header").unwrap();
    let username = document
        .select(&username_selector)
        .map(|x| x.inner_html())
        .next()
        .unwrap()
        .trim()
        .to_string();
    let avatar = scraper::Selector::parse("div.avatar.avatar-static>img").unwrap();
    let avatar_element = document.select(&avatar).next().unwrap();
    let avatar_url = avatar_element
        .value()
        .attr("src")
        .unwrap()
        .trim()
        .to_string();
    let logs_elements = document
        .select(&log_selector)
        .map(|x| x.inner_html())
        .collect::<Vec<String>>();
    for log_element in logs_elements {
        let mut log_element_html = string_to_html(&log_element);
        let game_selector = scraper::Selector::parse("div.col.pl-1>a").unwrap();
        let a_element = log_element_html
            .select(&game_selector)
            .skip(1)
            .next()
            .unwrap();
        let game_url = a_element.value().attr("href").unwrap().trim().to_string();
        let game_name = log_element_html
            .select(&game_selector)
            .skip(1)
            .next()
            .unwrap()
            .inner_html()
            .trim()
            .to_string();
        let status_log = get_status_log(&log_element);
        let stars_selector = scraper::Selector::parse(
            "div.col.pl-1>div.stars-inline.star-ratings-static>div.stars-top",
        )
        .unwrap();
        let element = log_element_html.select(&stars_selector).next().unwrap();
        let style_text = element.value().attr("style").unwrap_or("");
        let numeric_chars = style_text
            .chars()
            .filter(|c| c.is_numeric())
            .collect::<String>();
        let stars = numeric_chars.parse::<f64>().unwrap_or(0.0) * 5.0 / 100.0;
        let timestamp_selector =
            scraper::Selector::parse("div.col-auto>p.mb-0.time-tooltip").unwrap();
        let timestamp = string_to_html(
            log_element_html
                .select(&timestamp_selector)
                .next()
                .unwrap()
                .value()
                .attr("data-tippy-content")
                .unwrap(),
        )
        .select(&scraper::Selector::parse("time").unwrap())
        .next()
        .unwrap()
        .value()
        .attr("datetime")
        .unwrap()
        .to_string();
        if status_log != Status::None {
            logs.push(get_log(
                username.clone(),
                decode_html_entities_to_string(game_name, &mut "".to_string()).to_string(),
                stars,
                status_log,
                game_url,
                avatar_url.clone(),
                timestamp,
            ));
        }
    }
    return logs;
}

fn get_timestamp(timestamp_str: &str) -> i64 {
    let datetime: DateTime<Utc> = Utc
        .datetime_from_str(timestamp_str, "%Y-%m-%dT%H:%M:%SZ")
        .unwrap();
    datetime.timestamp()
}

fn get_stars_text(s: f64) -> String {
    let mut stars = String::new();
    for _ in 0..s as i32 {
        stars.push('★');
    }
    if s.fract() > 0.0 {
        stars.push('½');
    }
    return stars;
}

fn get_log(
    username: String,
    game_name: String,
    rating: f64,
    status: Status,
    game_url: String,
    avatar_url: String,
    timestamp: String,
) -> Log {
    Log {
        username,
        game_name,
        rating,
        status,
        game_url,
        avatar_url,
        timestamp,
    }
}

fn string_to_html(s: &str) -> Html {
    Html::parse_fragment(&s)
}

fn get_status_log(s: &str) -> Status {
    match s {
        s if s.contains("is now playing") => Status::Playing,
        s if s.contains("played") => Status::Played,
        s if s.contains("completed") => Status::Completed,
        s if s.contains("abandoned") => Status::Abandoned,
        s if s.contains("shelved") => Status::Shelved,
        s if s.contains("retired") => Status::Retired,
        _ => Status::None,
    }
}

fn localize_status(status: &Status) -> String {
    match status {
        Status::Playing => "Jugando".to_string(),
        Status::Played => "Terminado".to_string(),
        Status::Completed => "Completado".to_string(),
        Status::Abandoned => "Abandonado".to_string(),
        Status::Shelved => "Dejado en la estantería".to_string(),
        Status::Retired => "Retirado".to_string(),
        Status::None => "No sé que ha hecho".to_string(),
    }
}

#[derive(PartialEq)]
enum Status {
    Playing,
    Played,
    Completed,
    Abandoned,
    Shelved,
    Retired,
    None,
}

struct Log {
    username: String,
    game_name: String,
    rating: f64,
    status: Status,
    game_url: String,
    avatar_url: String,
    timestamp: String,
}
