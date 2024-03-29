extern crate hyper;
extern crate url;
extern crate hyper_native_tls;

use std::io::Read;
use std::thread;
use std::time::Duration;
use std::sync::mpsc::channel;
use std::fmt;

use crate::parse;

use self::hyper::Client;
use self::hyper::status::StatusCode;
use self::hyper::net::HttpsConnector;
use self::url::{ParseResult, Url, UrlParser};
use self::hyper_native_tls::NativeTlsClient;

const TIMEOUT: u64 = 10;

#[derive(Debug, Clone)]
pub enum UrlState {
    Accessible(Url, bool),
    BadStatus(Url, StatusCode),
    ConnectionFailed(Url, String),
    TimedOut(Url),
    Malformed(String),
}

impl fmt::Display for UrlState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            UrlState::Accessible(ref url, ref par) => format!("!! (parsed: {}) {}", par, url).fmt(f),
            UrlState::BadStatus(ref url, ref status) => format!("x ({}) {}", status, url).fmt(f),
            UrlState::ConnectionFailed(ref url, ref error) => format!("x (connection failed)  {} -> {}", error, url).fmt(f),
            UrlState::TimedOut(ref url) => format!("x (timed out) {}", url).fmt(f),
            UrlState::Malformed(ref url) => format!("x (malformed) {}", url).fmt(f),
        }
    }
}

fn build_url(domain: &str, path: &str) -> ParseResult<Url> {
    let base_url_string = format!("https://{}", domain);
    let base_url = Url::parse(&base_url_string).unwrap();

    let mut raw_url_parser = UrlParser::new();
    let url_parser = raw_url_parser.base_url(&base_url);

    url_parser.parse(path)
}


pub fn url_status(domain: &str, path: &str) -> UrlState {
    match build_url(domain, path) {
        Ok(url) => {
            let (tx, rx) = channel();
            let req_tx = tx.clone();
            let u = url.clone();

            thread::spawn(move || {
                let client = get_client();
                let url_string = url.serialize();
                let resp = client.get(&url_string).send();

                let _ = req_tx.send(match resp {
                    Ok(r) => if let StatusCode::Ok = r.status {
                        UrlState::Accessible(url, false)
                    } else {
                        UrlState::BadStatus(url, r.status)
                    },
                    Err(e) => UrlState::ConnectionFailed(url, e.to_string()),
                });
            });

            thread::spawn(move || {
                thread::sleep(Duration::from_secs(TIMEOUT));
                let _ = tx.send(UrlState::TimedOut(u));
            });

            rx.recv().unwrap()
        }
        Err(_) => UrlState::Malformed(path.to_owned()),
    }
}

pub fn fetch_url(url: &Url) -> String {
    let client = get_client();

    let url_string = url.serialize();
    let mut res = client
        .get(&url_string)
        .send()
        .ok()
        .expect("could not fetch URL");

    
    let mut body = String::new();
    match res.read_to_string(&mut body) {
        Ok(_) => body,
        Err(_) => String::new(),
    }
}

pub fn fetch_all_urls(url: &Url, save_md: bool) -> Vec<String> {
    let html_src = fetch_url(url);
    let dom = parse::parse_html(&html_src);
    if save_md {
        parse::extract_contents(url.serialize(), dom.document.clone());
    }
    parse::get_urls(dom.document)
}

fn get_client() -> Client {
    let ssl = NativeTlsClient::new().unwrap();
    let connector = HttpsConnector::new(ssl);
    Client::with_connector(connector)
}