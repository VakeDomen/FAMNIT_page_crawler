extern crate html5ever;
extern crate url;

use std::io::stdout;
use std::io::Write;
use std::vec;
use url::Url;

use fetch::UrlState;

mod parse;
mod fetch;
mod crawler;

fn main() {
    let start_url_string = "https://www.famnit.upr.si";

    let start_url = Url::parse(start_url_string).unwrap();
    let domain = start_url
        .domain()
        .expect("I can't find a domain in your URL");

    let mut success_count = 0;
    let mut fail_count = 0;
    let mut denied_count = 0;

    let url_word_blacklist = vec![
        "konference".to_owned(),
        "conference".to_owned(),
        "resources".to_owned(),
        "news".to_owned(),
        "novice".to_owned(),
        "project".to_owned(),
        "projekt".to_owned(),
        "dogodek".to_owned(),
        "event".to_owned(),
    ];

    for url_state in crawler::crawl(&domain, &start_url, url_word_blacklist) {
        match url_state {
            UrlState::Accessible(_, parsed) => {
                if parsed {
                    success_count += 1;
                } else {
                    denied_count += 1;
                }
            }
            status => {
                fail_count += 1;
                println!("{}", status);
            }
        }

        print!("Succeeded: {} Failed: {} Denied: {}\r", success_count, fail_count, denied_count);
        stdout().flush().unwrap();
    }
    print!("Succeeded: {} Failed: {}\r", success_count, fail_count);
}