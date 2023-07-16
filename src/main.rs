use std::fs;
use std::io::prelude::*;
use scraper::Html;
use serenity::prelude::*;

mod database;
struct Handler;

fn main() {
    let binding = fs::read_to_string("src/users.txt").unwrap();
    let usernames = binding.split("\n").collect::<Vec<&str>>();
    for username in usernames {
        print_logs(&username);
    }
}

fn print_logs(username: &str) {
    let response =
        reqwest::blocking::get("https://www.backloggd.com/u/".to_owned() + username + "/activity/you/played/")
            .unwrap()
            .text()
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
            println!("{} {} {} con {}/5", username, localize_status(status_log), game_name, stars);
        }
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

fn localize_status(status: Status) -> String {
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

fn get_log(
    username: String,
    game_name: String,
    rating: Option<f32>,
    status: Status,
    timestamp: i32,
) -> Log {
    Log {
        username,
        game_name,
        rating,
        status,
        timestamp,
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
    rating: Option<f32>,
    status: Status,
    timestamp: i32,
}
