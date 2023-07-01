fn main() {
    let response =
        reqwest::blocking::get("https://www.backloggd.com/u/Winnie/activity/you/played,finished/")
            .unwrap()
            .text()
            .unwrap();
    let document = scraper::Html::parse_document(&response);
    let title_selector = scraper::Selector::parse("div#activities-list>div").unwrap();
    let titles = document.select(&title_selector).map(|x| x.inner_html());
    for title in titles {
        println!("{}", title);
    }
    println!("Hello, world!");
}
