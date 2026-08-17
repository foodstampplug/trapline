use anyhow::Result;
use reqwest::blocking::Client;
use reqwest::header::{ETAG, IF_NONE_MATCH};
use std::cell::RefCell;
use std::time::{Duration, Instant};

pub enum Fetched {
    NotModified,
    Body { text: String, etag: String },
}

/// A single-threaded HTTP client with a built-in global throttle.
pub struct Fetcher {
    client: Client,
    min_gap: Duration,
    last: RefCell<Option<Instant>>,
}

impl Fetcher {
    pub fn new(user_agent: &str, requests_per_min: u64) -> Result<Self> {
        let client = Client::builder()
            .user_agent(user_agent)
            .timeout(Duration::from_secs(20))
            .build()?;
        let min_gap = if requests_per_min == 0 {
            Duration::from_millis(0)
        } else {
            Duration::from_millis(60_000 / requests_per_min)
        };
        Ok(Self {
            client,
            min_gap,
            last: RefCell::new(None),
        })
    }

    fn throttle(&self) {
        if let Some(prev) = *self.last.borrow() {
            let elapsed = prev.elapsed();
            if elapsed < self.min_gap {
                std::thread::sleep(self.min_gap - elapsed);
            }
        }
        *self.last.borrow_mut() = Some(Instant::now());
    }

    /// Conditional GET. Sends If-None-Match when an etag is supplied.
    pub fn get(&self, url: &str, etag: Option<&str>) -> Result<Fetched> {
        self.throttle();
        let mut req = self.client.get(url);
        if let Some(t) = etag {
            if !t.is_empty() {
                req = req.header(IF_NONE_MATCH, t);
            }
        }
        let resp = req.send()?;
        if resp.status().as_u16() == 304 {
            return Ok(Fetched::NotModified);
        }
        let new_etag = resp
            .headers()
            .get(ETAG)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let text = resp.text()?;
        Ok(Fetched::Body { text, etag: new_etag })
    }
}
