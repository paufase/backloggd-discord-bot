use std::{env, fs};
use scraper::Html;

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use dotenv::dotenv;
use serenity::utils::MessageBuilder;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, context: Context, msg: Message) {
        if msg.content == "!backloggd" {
            let channel = match msg.channel_id.to_channel(&context).await {
                Ok(channel) => channel,
                Err(why) => {
                    println!("Error getting channel: {:?}", why);

                    return;
                },
            };
            let logs = check_feeds().await;
            for log in logs {
                msg.channel_id.send_message(&context.http, |m| {
                    m.embed(|e| e
                        .colour(0xbcdefa)
                        .title(MessageBuilder::new()
                            .push(log.game_name)
                            .push(" ")
                            .push(getStarsText(log.rating))
                            .build())
                        .field(log.username.clone() + " " + &*localize_status(&log.status), "", false)
                        .image("https://www.backloggd.com/packs/media/images/meta_banner-d63b2a0bc9b9184fa61ddf135435c219.jpg")
                        .footer(|f| {
                            f.text("🕒 Stats last updated @");
                            f
                        })
                    )
                }).await.expect("TODO: panic message");
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
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
    let mut client =
        Client::builder(token, intents).event_handler(Handler).await.expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}

async fn check_feeds() -> Vec<Log> {
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
    let response =
        reqwest::get("https://www.backloggd.com/u/".to_owned() + username + "/activity/you/played/")
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
    let document = scraper::Html::parse_document(&response);
    let log_selector = scraper::Selector::parse("div.row.activity").unwrap();
    let username_selector = scraper::Selector::parse("h3.mr-2.mb-0.main-header").unwrap();
    let username = document
        .select(&username_selector)
        .map(|x| x.inner_html())
        .next()
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
        let game_name = log_element_html
            .select(&game_selector)
            .skip(1)
            .next()
            .unwrap()
            .inner_html()
            .trim()
            .to_string();
        let status_log = get_status_log(&log_element);
        let stars_selector = scraper::Selector::parse("div.col.pl-1>div.stars-inline.star-ratings-static>div.stars-top").unwrap();
        let element = log_element_html.select(&stars_selector).next().unwrap();
        let style_text = element.value().attr("style").unwrap_or("");
        let numeric_chars = style_text.chars().filter(|c| c.is_numeric()).collect::<String>();
        let stars = numeric_chars.parse::<f64>().unwrap_or(0.0) * 5.0 / 100.0;
        if status_log != Status::None {
            logs.push(get_log(username.clone(), game_name, stars, status_log));
        }
    }
    return logs;
}

fn getStarsText(s: f64) -> String {
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
) -> Log {
    Log {
        username,
        game_name,
        rating,
        status,
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
        Status::Playing => "está jugando a".to_string(),
        Status::Played => "ha jugado a".to_string(),
        Status::Completed => "ha completado".to_string(),
        Status::Abandoned => "ha abandonado".to_string(),
        Status::Shelved => "ha dejado en la estantería".to_string(),
        Status::Retired => "ha retirado".to_string(),
        Status::None => "no ha hecho nada con".to_string(),
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
}
