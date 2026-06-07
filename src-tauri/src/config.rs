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
}

fn default_username() -> String {
    "Trapline".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            webhook_url: String::new(),
            username: "Trapline".to_string(),
            shell: String::new(),
            community_discord: String::new(),
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
