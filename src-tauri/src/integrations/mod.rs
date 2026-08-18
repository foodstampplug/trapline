pub mod commands;
pub mod leakcheck;
pub mod shodan;

use std::time::Duration;

/// Shared HTTP client for third-party integration lookups (async, rustls).
pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}
