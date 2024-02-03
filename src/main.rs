use chrono::NaiveDate;
use chrono::{DateTime, Local, TimeZone, Utc};
use dotenv::dotenv;
use html_escape::decode_html_entities_to_string;
use reqwest::header::{HeaderMap, HeaderValue};
use scraper::Html;
use serde::Deserialize;
use serenity::all::CreateEmbed;
use serenity::all::CreateEmbedAuthor;
use serenity::all::CreateMessage;
use serenity::async_trait;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use serenity::utils::MessageBuilder;
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::BufWriter;
use std::io::Write;
use std::time::Duration;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, context: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        loop {
            println!("Checking logs at {}", Local::now());
            let logs = get_logs().await;
            if !logs.is_empty() {
                refresh_twitch_token().await;
            }
            for log in logs {
                let cover = get_cover(log.game_url.as_str()).await;
                let avatar_url = get_avatar_url(&log.username).await;
                let channel_id = ChannelId::new(env::var("DISCORD_CHANNEL_ID").expect("Expected a discord channel id in the environment").parse().unwrap());
                channel_id
                    .send_message(
                        &context.http,
                        CreateMessage::new().embed(create_embed(log, avatar_url, cover)),
                    )
                    .await
                    .expect("TODO: panic message");
            }
            tokio::time::sleep(
                Duration::from_secs(env::var("SECONDS_UNTIL_NEXT_CHECK")
                    .expect("Environment variable SECONDS_UNTIL_NEXT_CHECK is missing")
                    .parse::<u64>()
                    .expect("SECONDS_UNTIL_NEXT_CHECK is not a number"))).await;
        }
    }
}

fn create_embed(log: Log, avatar_url: String, cover: Option<String>) -> CreateEmbed {
    println!("{} logged by {}", log.game_name.clone(), log.username.clone());
    let embed = CreateEmbed::new()
        .colour(0xbcdefa)
        .title(
            MessageBuilder::new()
                .push(log.game_name.to_string() + " " + &*get_stars_text(log.rating))
                .build(),
        )
        .url("https://www.backloggd.com".to_owned() + &*log.game_url)
        .field(
            localize_status(&log.status)
                + " <t:".to_string().as_str()
                + get_timestamp(log.timestamp.as_str()).to_string().as_str()
                + ":R>",
            "",
            false,
        )
        .author({
            let author = CreateEmbedAuthor::new(log.username.clone())
                .url("https://www.backloggd.com/u/".to_owned() + &*log.username)
                .icon_url(avatar_url);
            author
        });
    if let Some(cover) = cover {
        return embed.clone().thumbnail(
            "https://images.igdb.com/igdb/image/upload/t_cover_big/".to_owned()
                + cover.trim()
                + ".png",
        );
    }
    return embed;
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    refresh_twitch_token().await;
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let token = env::var("DISCORD_TOKEN").expect("Expected a discord token in the environment");
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}

async fn refresh_twitch_token() {
    let client_id =
        env::var("TWITCH_CLIENT_ID").expect("Expected a twitch client id in the environment");
    let client_secret = env::var("TWITCH_CLIENT_SECRET")
        .expect("Expected a twitch client secret in the environment");
    let token_generation_date = env::var("TWITCH_TOKEN_GENERATION_DATE").map_or_else(
        |_| NaiveDate::from_ymd_opt(1997, 12, 28).unwrap(),
        |date_string| {
            NaiveDate::parse_from_str(date_string.as_str(), "%Y-%m-%d")
                .unwrap_or_else(|_| Utc::now().naive_utc().date())
        },
    );
    if (Utc::now().naive_utc().date() - token_generation_date).num_days() < 30 ||  token_generation_date > Utc::now().naive_utc().date() {
        return;
    }
    let client = reqwest::Client::new();
    let response = client
        .post("https://id.twitch.tv/oauth2/token")
        .body(
            "client_id=".to_owned()
                + &client_id
                + "&client_secret="
                + &client_secret
                + "&grant_type=client_credentials",
        )
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let response = serde_json::from_str::<serde_json::Value>(&response).unwrap();
    let access_token = response.get("access_token").unwrap().as_str().unwrap();
    let file_content = fs::read_to_string(".env").unwrap();
    let mut lines: Vec<String> = file_content.lines().map(String::from).collect();

    for line in &mut lines {
        if line.starts_with("TWITCH_ACCESS_TOKEN=") {
            *line = format!("TWITCH_ACCESS_TOKEN={}", access_token);
        } else if line.starts_with("TWITCH_TOKEN_GENERATION_DATE=") {
            *line = format!(
                "TWITCH_TOKEN_GENERATION_DATE={}",
                Utc::now().naive_utc().date()
            );
        }
    }

    let file = fs::File::create(".env").unwrap();
    let mut writer = BufWriter::new(file);
    for line in &lines {
        writeln!(writer, "{}", line).unwrap();
    }
}

async fn get_cover(game_name: &str) -> Option<String> {
    let access_token =
        env::var("TWITCH_ACCESS_TOKEN").expect("Expected a twitch access token in the environment");
    let client_id =
        env::var("TWITCH_CLIENT_ID").expect("Expected a twitch client id in the environment");
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
    let maybe_image_id = serde_json::from_str::<Vec<Cover>>(&cover_response);
    if maybe_image_id.as_ref().unwrap().get(0).is_none() {
        return None;
    }
    let image_id = maybe_image_id.unwrap().get(0).unwrap().image_id.to_string();
    Some(image_id)
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
    let response = reqwest::get(
        "https://www.backloggd.com/u/spanishtoboggan/activity/friends/played,finished/",
    )
    .await
    .unwrap()
    .text()
    .await
    .unwrap();
    let document = Html::parse_document(&response);
    let log_selector = scraper::Selector::parse("div.row.activity").unwrap();
    let logs_elements = document
        .select(&log_selector)
        .map(|x| x.inner_html())
        .collect::<Vec<String>>();
    for log_element in logs_elements {
        let log_element_html = string_to_html(&log_element);
        let game_selector = scraper::Selector::parse("div.col.pl-1>a").unwrap();
        let a_element = log_element_html.select(&game_selector).nth(1).unwrap();
        let game_url = a_element.value().attr("href").unwrap().trim().to_string();
        let game_name = log_element_html
            .select(&game_selector)
            .nth(1)
            .unwrap()
            .inner_html()
            .trim()
            .to_string();
        let username = log_element_html
            .select(&game_selector)
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
        let mut stars = 0.0;
        let element = log_element_html.select(&stars_selector).next();
        if element.is_some() {
            let style_text = element.unwrap().value().attr("style").unwrap_or("");
            let numeric_chars = style_text
                .chars()
                .filter(|c| c.is_numeric())
                .collect::<String>();
            stars = numeric_chars.parse::<f64>().unwrap_or(0.0) * 5.0 / 100.0;
        }
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
        if (status_log != Status::Completed)
            && has_not_passed_more_than_an_hour(&timestamp)
        {
            logs.push(get_log(
                username.clone(),
                decode_html_entities_to_string(game_name, &mut "".to_string()).to_string(),
                stars,
                status_log,
                game_url,
                timestamp,
            ));
        }
    }
    logs.dedup_by_key(|log| {
        let mut hasher = DefaultHasher::new();
        (&log.game_name, &log.username).hash(&mut hasher);
        hasher.finish()
    });
    logs
}

async fn get_avatar_url(username: &str) -> String {
    let response = reqwest::get("https://www.backloggd.com/u/".to_string() + username)
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let document = Html::parse_document(&response);
    let avatar = scraper::Selector::parse("div.avatar.avatar-static>img").unwrap();
    let avatar_element = document.select(&avatar).next().unwrap();
    avatar_element
        .value()
        .attr("src")
        .unwrap()
        .trim()
        .to_string()
}

fn has_not_passed_more_than_an_hour(timestamp: &str) -> bool {
    let timestamp = get_timestamp(timestamp);
    let now = Utc::now().timestamp();
    now - timestamp < 1800
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
    stars
}

fn get_log(
    username: String,
    game_name: String,
    rating: f64,
    status: Status,
    game_url: String,
    timestamp: String,
) -> Log {
    Log {
        username,
        game_name,
        rating,
        status,
        game_url,
        timestamp,
    }
}

fn string_to_html(s: &str) -> Html {
    Html::parse_fragment(s)
}

fn get_status_log(s: &str) -> Status {
    match s {
        s if s.contains("now playing") => Status::Playing,
        s if s.contains("played") || s.contains("finished") => Status::Played,
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
    timestamp: String,
}
