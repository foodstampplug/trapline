use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Mirrors Go HunterFinding struct — all camelCase JSON tags
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HunterFinding {
    pub id: String,
    #[serde(default)]
    pub program_name: String,
    #[serde(default)]
    pub platform: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub steps: String,
    #[serde(default)]
    pub evidence: String,
    #[serde(default)]
    pub impact: String,
    #[serde(default)]
    pub remediation: String,
    #[serde(default)]
    pub cvss: String,
    #[serde(default)]
    pub cvss_score: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub cmdline: String,
    #[serde(default = "now_iso")]
    pub created_at: String,
    #[serde(default = "now_iso")]
    pub updated_at: String,
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn findings_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("Trapline").join("findings.json")
}

fn load_all() -> Vec<HunterFinding> {
    let path = findings_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_all(findings: &[HunterFinding]) -> Result<(), String> {
    let path = findings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let data = serde_json::to_string_pretty(findings).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| e.to_string())
}

/// Upsert a finding by ID (parsed from a JSON string)
pub fn save(data: &str) -> Result<(), String> {
    let mut incoming: HunterFinding = serde_json::from_str(data)
        .map_err(|e| format!("invalid finding JSON: {}", e))?;
    incoming.updated_at = now_iso();

    let mut all = load_all();
    if let Some(pos) = all.iter().position(|f| f.id == incoming.id) {
        // Preserve original created_at on update
        incoming.created_at = all[pos].created_at.clone();
        all[pos] = incoming;
    } else {
        if incoming.created_at.is_empty() {
            incoming.created_at = now_iso();
        }
        all.push(incoming);
    }
    save_all(&all)
}

/// Return all findings as a JSON string
pub fn load() -> Result<String, String> {
    let all = load_all();
    serde_json::to_string(&all).map_err(|e| e.to_string())
}

/// Delete a finding by ID
pub fn delete(id: &str) -> Result<(), String> {
    let mut all = load_all();
    let before = all.len();
    all.retain(|f| f.id != id);
    if all.len() == before {
        return Err(format!("finding {} not found", id));
    }
    save_all(&all)
}
