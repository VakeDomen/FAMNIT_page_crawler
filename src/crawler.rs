use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use url::Url;

use fetch::{fetch_all_urls, url_status, UrlState};

use crate::fetch;

const THREADS: i32 = 20;

pub struct Crawler {
    to_visit: Arc<Mutex<Vec<String>>>,
    active_count: Arc<Mutex<i32>>,
    parsed_count: Arc<Mutex<i32>>,
    url_states: Receiver<UrlState>,
}

impl Iterator for Crawler {
    type Item = UrlState;

    fn next(&mut self) -> Option<UrlState> {
        loop {
            match self.url_states.try_recv() {
                Ok(state) => return Some(state),
                Err(_) => {
                    let to_visit_val = self.to_visit.lock().unwrap();
                    let active_count_val = self.active_count.lock().unwrap();
                    let parsed_count_val = self.parsed_count.lock().unwrap();

                    if to_visit_val.is_empty() && *active_count_val == 0 {
                        return None;
                    } else {
                        continue;
                    }
                }
            }
        }
    }
}

fn crawl_worker_thread(
    domain: &str,
    to_visit: Arc<Mutex<Vec<String>>>,
    visited: Arc<Mutex<HashSet<String>>>,
    active_count: Arc<Mutex<i32>>,
    parsed_count: Arc<Mutex<i32>>,
    url_states: Sender<UrlState>,
    url_word_blacklist: Vec<String>,
    save_md: bool,
) {
    loop {
        let current;
        {
            let mut to_visit_val = to_visit.lock().unwrap();
            let mut active_count_val = active_count.lock().unwrap();
            if to_visit_val.is_empty() {
                if *active_count_val > 0 {
                    continue;
                } else {
                    break;
                }
            };
            current = to_visit_val.pop().unwrap();
            *active_count_val += 1;
            assert!(*active_count_val <= THREADS);
        }

        {
            let mut visited_val = visited.lock().unwrap();
            if visited_val.contains(&current) {
                let mut active_count_val = active_count.lock().unwrap();
                *active_count_val -= 1;
                continue;
            } else {
                visited_val.insert(current.to_owned());
            }
        }

        let mut state = url_status(&domain, &current);
        if let UrlState::Accessible(ref url, ref mut parsed) = state.clone() {
            if url.domain() == Some(&domain) {

                let mut should_skip = false;
                for word in &url_word_blacklist {
                    if url.serialize().contains(word) {
                        should_skip = true;
                        break;
                    }
                }

                if !should_skip {
                    // continue;
                    let new_urls = fetch_all_urls(&url, save_md);
    

                    {
                        let mut parsed_count_val = parsed_count.lock().unwrap();
                        *parsed_count_val += 1;
                        state = UrlState::Accessible(url.clone(), true);
                    }

                    let mut to_visit_val = to_visit.lock().unwrap();
                    for new_url in new_urls {
                        let parsed_url = match Url::parse(&new_url) {
                            Ok(url) => url,
                            Err(_) => continue,
                        };
                        if parsed_url.domain() == Some(domain) {
                            if parsed_url.serialize().contains("staff") {
                                to_visit_val.push(new_url);
                            }
                        }
                    }
                }
            }
        }

        {
            let mut active_count_val = active_count.lock().unwrap();
            *active_count_val -= 1;
            assert!(*active_count_val >= 0);
        }

        url_states.send(state).unwrap();
    }
}

pub fn crawl(
    domain: &str, 
    start_url: &Url, 
    url_word_blacklist: Vec<String>,
    save_md: bool,
) -> Crawler {
    let to_visit = Arc::new(Mutex::new(vec![start_url.serialize()]));
    let active_count = Arc::new(Mutex::new(0));
    let parsed_count = Arc::new(Mutex::new(0));
    let visited = Arc::new(Mutex::new(HashSet::new()));

    let (tx, rx) = channel();

    let crawler = Crawler {
        to_visit: to_visit.clone(),
        active_count: active_count.clone(),
        parsed_count: parsed_count.clone(),
        url_states: rx,
    };

    for _ in 0..THREADS {
        let domain = domain.to_owned();
        let to_visit = to_visit.clone();
        let visited = visited.clone();
        let active_count = active_count.clone();
        let parsed_count = parsed_count.clone();
        let tx = tx.clone();
        let blacklist = url_word_blacklist.clone();
        thread::spawn(move || {
            crawl_worker_thread(
                &domain, 
                to_visit, 
                visited, 
                active_count, 
                parsed_count,
                tx, 
                blacklist,
                save_md,
            );
        });
    }

    crawler
}