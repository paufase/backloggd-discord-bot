mod database;

fn main() {
    let response =
        reqwest::blocking::get("https://www.backloggd.com/u/Winnie/activity/you/played,finished/")
            .unwrap()
            .text()
            .unwrap();
    let document = scraper::Html::parse_document(&response);
    let title_selector = scraper::Selector::parse("div.row.activity>div.col.pl-1>a").unwrap();
    let mut elements = document.select(&title_selector).map(|x| x.inner_html()); // Don't want to make it mut, but I don't know how to do it otherwise
    let user = elements.next().unwrap();
    let games = elements.step_by(2);
    for game in games {
        println!("{} se ha pasado {}", user, game);
    }
}
