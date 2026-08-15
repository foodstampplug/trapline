use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Mirrors Go Config struct — JSON field names must be camelCase to match JS expectations
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
    #[serde(default)]
    pub deck_path: String,
    #[serde(default = "default_deck_port")]
    pub deck_port: u16,
    #[serde(default)]
    pub deck_token: String,
}

fn default_username() -> String {
    "Trapline".to_string()
}

fn default_deck_port() -> u16 {
    8787
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
pub fn preserve_deck_fields(mut incoming: Config, current: &Config) -> Config {
    incoming.deck_path = current.deck_path.clone();
    incoming.deck_port = current.deck_port;
    incoming.deck_token = current.deck_token.clone();
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
}
