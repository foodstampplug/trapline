use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// One watch target: pages to scan + direct JS URLs, gated by `in_scope`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WatchTarget {
    pub name: String,
    #[serde(default)]
    pub pages: Vec<String>,
    #[serde(default)]
    pub js: Vec<String>,
    /// Safety gate: only hosts matching these suffixes are ever fetched.
    #[serde(default)]
    pub in_scope: Vec<String>,
    /// Phase-3 hook: auto-run Shodan/LeakCheck enrichment on new findings.
    #[serde(default)]
    pub auto_enrich: bool,
}

/// App configuration, persisted as camelCase JSON to match the JS frontend.
/// (The webhook/username/shell/community fields mirror the old Go build; the
/// `deck_*` fields are new for the Deck launcher and have no Go counterpart.)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default)]
    pub webhook_url: String,
    #[serde(default = "default_username")]
    pub username: String,
    #[serde(default)]
    pub shell: String,
    #[serde(default)]
    pub community_discord: String,
    /// Path to the local Trapline Deck project folder. Empty → resolved to
    /// `~/trapline-deck` at runtime by `deck::resolve_deck_dir`.
    #[serde(default)]
    pub deck_path: String,
    /// Port the Deck server listens on (default 8787).
    #[serde(default = "default_deck_port")]
    pub deck_port: u16,
    /// Auth token Trapline owns and passes to Deck via `DECK_TOKEN`. Empty →
    /// generated + persisted on the first successful `deck_start`.
    #[serde(default)]
    pub deck_token: String,
    /// Watch targets (change-detection). Managed via Settings, persisted here.
    #[serde(default)]
    pub watch_targets: Vec<WatchTarget>,
    #[serde(default = "default_watch_interval")]
    pub watch_interval_secs: u64,
    #[serde(default = "default_watch_threshold")]
    pub watch_alert_threshold: i64,
    #[serde(default = "default_watch_rpm")]
    pub watch_max_rpm: u32,
    /// Runtime state: is the scheduler currently on? Auto-resumed on launch.
    /// Not a Settings-form field — preserved across Settings saves.
    #[serde(default)]
    pub watch_enabled: bool,
}

fn default_username() -> String {
    "Trapline".to_string()
}

fn default_deck_port() -> u16 {
    8787
}

fn default_watch_interval() -> u64 {
    1800
}

fn default_watch_threshold() -> i64 {
    50
}

fn default_watch_rpm() -> u32 {
    30
}

impl Default for Config {
    fn default() -> Self {
        Self {
            webhook_url: String::new(),
            username: "Trapline".to_string(),
            shell: String::new(),
            community_discord: String::new(),
            deck_path: String::new(),
            deck_port: 8787,
            deck_token: String::new(),
            watch_targets: Vec::new(),
            watch_interval_secs: 1800,
            watch_alert_threshold: 50,
            watch_max_rpm: 30,
            watch_enabled: false,
        }
    }
}

/// Returns {config_dir}/Trapline/config.json
pub fn config_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("Trapline").join("config.json")
}

/// Legacy path from the old "Quarry" brand
fn legacy_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("Quarry").join("config.json")
}

/// Ensure the Trapline config directory exists
pub fn ensure_config_dir() {
    if let Some(parent) = config_path().parent() {
        let _ = fs::create_dir_all(parent);
    }
}

pub fn load_or_default() -> Config {
    let path = config_path();

    // Try new path first
    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(cfg) = serde_json::from_str::<Config>(&data) {
            return cfg;
        }
    }

    // Migrate from legacy Quarry path
    let legacy = legacy_path();
    if let Ok(data) = fs::read_to_string(&legacy) {
        if let Ok(cfg) = serde_json::from_str::<Config>(&data) {
            // Save to new location and return
            save(&cfg);
            return cfg;
        }
    }

    Config::default()
}

pub fn save(cfg: &Config) {
    ensure_config_dir();
    if let Ok(data) = serde_json::to_string_pretty(cfg) {
        let _ = fs::write(config_path(), data);
    }
}

/// Carry Deck fields (which the Settings form does not round-trip) from the
/// existing config onto an incoming one, so saving Settings never wipes them.
/// Also preserves watch_enabled (runtime state managed by the scheduler).
pub fn preserve_deck_fields(mut incoming: Config, current: &Config) -> Config {
    incoming.deck_path = current.deck_path.clone();
    incoming.deck_port = current.deck_port;
    incoming.deck_token = current.deck_token.clone();
    incoming.watch_enabled = current.watch_enabled;
    incoming
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_config_json_loads_with_deck_defaults() {
        // A config.json written before the Deck fields existed.
        let old = r#"{"webhookUrl":"","username":"Trapline","shell":"","communityDiscord":""}"#;
        let cfg: Config = serde_json::from_str(old).expect("old config must still parse");
        assert_eq!(cfg.deck_port, 8787);
        assert_eq!(cfg.deck_path, "");
        assert_eq!(cfg.deck_token, "");
    }

    #[test]
    fn default_has_deck_fields() {
        let cfg = Config::default();
        assert_eq!(cfg.deck_port, 8787);
        assert!(cfg.deck_path.is_empty());
        assert!(cfg.deck_token.is_empty());
    }

    #[test]
    fn preserve_deck_fields_carries_deck_from_current() {
        let mut current = Config::default();
        current.deck_path = "D:/custom/trapline-deck".into();
        current.deck_port = 9001;
        current.deck_token = "cafebabecafebabecafebabecafebabe".into();
        // Incoming (from the Settings form) has default deck fields:
        let incoming = Config { webhook_url: "wh".into(), ..Config::default() };
        let merged = preserve_deck_fields(incoming, &current);
        assert_eq!(merged.deck_path, "D:/custom/trapline-deck");
        assert_eq!(merged.deck_port, 9001);
        assert_eq!(merged.deck_token, "cafebabecafebabecafebabecafebabe");
        assert_eq!(merged.webhook_url, "wh"); // non-deck fields still come from incoming
    }

    #[test]
    fn old_config_json_loads_with_watch_defaults() {
        // A config.json written before the watch fields existed.
        let old = r#"{"webhookUrl":"","username":"Trapline","shell":"","communityDiscord":""}"#;
        let cfg: Config = serde_json::from_str(old).expect("old config must still parse");
        assert_eq!(cfg.watch_interval_secs, 1800);
        assert_eq!(cfg.watch_alert_threshold, 50);
        assert_eq!(cfg.watch_max_rpm, 30);
        assert!(!cfg.watch_enabled);
        assert!(cfg.watch_targets.is_empty());
    }

    #[test]
    fn watch_target_round_trips_camelcase() {
        let t = WatchTarget {
            name: "acme".into(),
            pages: vec!["https://app.acme.com".into()],
            js: vec![],
            in_scope: vec!["acme.com".into()],
            auto_enrich: true,
        };
        let j = serde_json::to_string(&t).unwrap();
        assert!(j.contains("\"inScope\""), "expected camelCase inScope, got {j}");
        assert!(j.contains("\"autoEnrich\""));
        let back: WatchTarget = serde_json::from_str(&j).unwrap();
        assert_eq!(back.name, "acme");
        assert_eq!(back.in_scope, vec!["acme.com".to_string()]);
    }

    #[test]
    fn preserve_deck_fields_also_carries_watch_enabled() {
        let mut current = Config::default();
        current.watch_enabled = true; // scheduler turned it on at runtime
        // Incoming from the Settings form has watch_enabled = false (form doesn't own it):
        let incoming = Config { webhook_url: "wh".into(), ..Config::default() };
        let merged = preserve_deck_fields(incoming, &current);
        assert!(merged.watch_enabled, "runtime watch_enabled must survive a Settings save");
        assert_eq!(merged.webhook_url, "wh");
    }
}
