//! In-memory config the engine reads. Built each cycle by the scheduler from the
//! app-level `crate::config::Config` — it is never parsed from a file.

#[derive(Debug, Clone)]
pub struct Config {
    pub interval_secs: u64,
    pub alert_threshold: i64,
    pub discord_webhook: String,
    pub discord_username: String,
    pub database: String,
    pub user_agent: String,
    pub max_requests_per_min: u64,
    pub probe_source_maps: bool,
    pub targets: Vec<Target>,
}

#[derive(Debug, Clone)]
pub struct Target {
    pub name: String,
    pub pages: Vec<String>,
    pub js: Vec<String>,
    /// Safety gate: only hosts matching these suffixes are ever fetched.
    pub in_scope: Vec<String>,
}

/// Default UA string (mirrors the standalone build).
pub fn default_user_agent() -> String {
    "Mozilla/5.0 (compatible; TraplineWatch/1.0; +https://trapline.xyz)".to_string()
}
