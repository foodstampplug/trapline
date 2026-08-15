use crate::config::Config;
use crate::discord;
use crate::findings;
use crate::flags::{color_for_findings, describe_findings};
use crate::runner;
use crate::session;
use crate::AppState;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

// ── Global PID registry ───────────────────────────────────────────────────────
// Stored in a process-lifetime static so the spawned runner thread can access
// it without holding a reference back to Tauri's managed AppState.
static GLOBAL_PIDS: Lazy<Arc<Mutex<HashMap<String, u32>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// ── Tool check ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub found: bool,
    pub hint: String,
}

const TOOLS: &[(&str, &str)] = &[
    ("subfinder",   "go install -v github.com/projectdiscovery/subfinder/v2/cmd/subfinder@latest"),
    ("httpx",       "go install -v github.com/projectdiscovery/httpx/cmd/httpx@latest"),
    ("nuclei",      "go install -v github.com/projectdiscovery/nuclei/v3/cmd/nuclei@latest"),
    ("dnsx",        "go install -v github.com/projectdiscovery/dnsx/cmd/dnsx@latest"),
    ("katana",      "go install -v github.com/projectdiscovery/katana/cmd/katana@latest"),
    ("assetfinder", "go install github.com/tomnomnom/assetfinder@latest"),
    ("waybackurls", "go install github.com/tomnomnom/waybackurls@latest"),
    ("gau",         "go install github.com/lc/gau/v2/cmd/gau@latest"),
    ("ffuf",        "go install github.com/ffuf/ffuf/v2@latest"),
    ("gobuster",    "go install github.com/OJ/gobuster/v3@latest"),
    ("naabu",       "go install -v github.com/projectdiscovery/naabu/v2/cmd/naabu@latest"),
    ("amass",       "go install -v github.com/owasp-amass/amass/v4/...@latest"),
    ("gf",          "go install github.com/tomnomnom/gf@latest"),
    ("anew",        "go install github.com/tomnomnom/anew@latest"),
    ("qsreplace",   "go install github.com/tomnomnom/qsreplace@latest"),
    ("dalfox",      "go install github.com/hahwul/dalfox/v2@latest"),
    ("subjs",       "go install github.com/lc/subjs@latest"),
    ("trufflehog",  "curl -sSfL https://raw.githubusercontent.com/trufflesecurity/trufflehog/main/scripts/install.sh | sh -s -- -b /usr/local/bin"),
    ("jq",          "https://jqlang.github.io/jq/download/"),
    ("aws",         "pip install awscli"),
    ("curl",        "curl comes pre-installed on Windows 10+ / macOS / Linux"),
    ("nslookup",    "nslookup comes pre-installed with your OS"),
];

#[tauri::command]
pub fn tool_check() -> Vec<ToolInfo> {
    TOOLS
        .iter()
        .map(|(name, hint)| ToolInfo {
            name: name.to_string(),
            found: which_tool(name),
            hint: hint.to_string(),
        })
        .collect()
}

fn which_tool(name: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("where")
            .arg(name)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("which")
            .arg(name)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

// ── Run / cancel commands ─────────────────────────────────────────────────────

#[tauri::command]
pub fn run_command(
    app: AppHandle,
    id: String,
    cmdline: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let shell = {
        state.config.lock().unwrap().shell.clone()
    };
    let pids = Arc::clone(&GLOBAL_PIDS);
    let id_t = id.clone();

    std::thread::spawn(move || {
        runner::run_command(app, id_t, cmdline, shell, pids);
    });

    Ok(())
}

#[tauri::command]
pub fn cancel_command(id: String) -> Result<(), String> {
    let pids = GLOBAL_PIDS.lock().unwrap();
    if let Some(&pid) = pids.get(&id) {
        drop(pids); // release lock before killing
        kill_process(pid);
        Ok(())
    } else {
        Err(format!("no running command with id {}", id))
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn kill_process(pid: u32) {
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/T", "/PID", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn kill_process(pid: u32) {
    // SIGTERM to the process group so child shells clean up
    unsafe {
        libc::kill(-(pid as i32), libc::SIGTERM);
    }
}

// ── Send card / loot to Discord ───────────────────────────────────────────────

/// We keep a registry of raw output text per command-id so send_card can upload it.
/// Populated by a thin wrapper around runner::run_command (see send_card_lines_store).
static CARD_OUTPUT: Lazy<Arc<Mutex<HashMap<String, (String, Vec<crate::flags::Finding>)>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Called from lib.rs to wrap the runner and capture output for Discord sends.
/// For simplicity we re-use run_command directly and accept that send_card will
/// have whatever has accumulated. The full store happens inside the runner thread —
/// we do the capture in runner.rs via the global CARD_OUTPUT handle.
#[tauri::command]
pub async fn send_card(
    id: String,
    title: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let (webhook_url, username) = {
        let cfg = state.config.lock().unwrap();
        (cfg.webhook_url.clone(), cfg.username.clone())
    };
    if webhook_url.is_empty() {
        return Err("No webhook URL configured — open Settings first.".to_string());
    }

    // Retrieve stored output for this command id
    let (output_text, findings_list) = {
        let store = CARD_OUTPUT.lock().unwrap();
        store
            .get(&id)
            .cloned()
            .unwrap_or_else(|| (String::from("(output not captured)"), vec![]))
    };

    let color = color_for_findings(&findings_list);
    let desc = describe_findings(&findings_list);
    let filename = format!(
        "trapline-{}.txt",
        &id[..std::cmp::min(8, id.len())]
    );
    let data = output_text.into_bytes();

    discord::send_file(&webhook_url, &username, &title, &desc, &filename, data, color).await
}

/// Store command output so send_card can retrieve it.
#[allow(dead_code)]
pub fn store_card_output(id: &str, text: String, findings: Vec<crate::flags::Finding>) {
    let mut store = CARD_OUTPUT.lock().unwrap();
    store.insert(id.to_string(), (text, findings));
}

#[tauri::command]
pub async fn send_loot(
    markdown: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let (webhook_url, username) = {
        let cfg = state.config.lock().unwrap();
        (cfg.webhook_url.clone(), cfg.username.clone())
    };
    if webhook_url.is_empty() {
        return Err("No webhook URL configured — open Settings first.".to_string());
    }

    let finding_count = markdown
        .lines()
        .filter(|l| l.starts_with("- **"))
        .count();
    let desc = format!("{} findings from this session", finding_count);
    let data = markdown.into_bytes();

    discord::send_file(
        &webhook_url,
        &username,
        "Trapline Loot Drop",
        &desc,
        "loot.md",
        data,
        0xff8c42,
    )
    .await
}

// ── Config ────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
pub fn set_config(config: Config, state: State<'_, AppState>) -> Result<(), String> {
    let merged = {
        let cur = state.config.lock().unwrap();
        crate::config::preserve_deck_fields(config, &cur)
    };
    crate::config::save(&merged);
    *state.config.lock().unwrap() = merged;
    Ok(())
}

#[tauri::command]
pub async fn test_webhook(url: String, state: State<'_, AppState>) -> Result<(), String> {
    if url.is_empty() {
        return Err("URL is empty".to_string());
    }
    let username = state.config.lock().unwrap().username.clone();
    discord::send_embed(
        &url,
        &username,
        "Trapline test",
        "Webhook connection confirmed. You're ready to ship loot.",
        0x5865F2,
    )
    .await
}

// ── URL opener ────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn open_url(url: String, app: AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| e.to_string())
}

// ── Session ───────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn save_session(data: String) -> Result<(), String> {
    session::save(&data)
}

#[tauri::command]
pub fn load_session() -> Result<String, String> {
    session::load()
}

#[tauri::command]
pub fn clear_session() -> Result<(), String> {
    session::clear()
}

// ── Findings ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn save_finding(data: String) -> Result<(), String> {
    findings::save(&data)
}

#[tauri::command]
pub fn load_findings() -> Result<String, String> {
    findings::load()
}

#[tauri::command]
pub fn delete_finding(id: String) -> Result<(), String> {
    findings::delete(&id)
}
