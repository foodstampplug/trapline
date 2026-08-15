use std::net::{IpAddr, UdpSocket};
use std::path::{Path, PathBuf};

use crate::AppState;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::State;

/// The single running Deck process (Deck is one-instance).
struct DeckProc {
    pid: u32,
    port: u16,
    token: String,
    lan_url: String,
    qr_svg: String,
}
static DECK: Lazy<Mutex<Option<DeckProc>>> = Lazy::new(|| Mutex::new(None));

static DECK_STARTING: AtomicBool = AtomicBool::new(false);
struct StartingGuard;
impl Drop for StartingGuard {
    fn drop(&mut self) {
        DECK_STARTING.store(false, Ordering::SeqCst);
    }
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeckStatus {
    pub running: bool,
    pub port: u16,
    pub url: String,
    pub lan_url: String,
    pub token: String,
    pub qr_svg: String,
    pub message: String,
}

fn node_on_path() -> bool {
    // Uses the `which` crate rather than spawning `where`/`which`, so there's no
    // brief console-window flash on Windows.
    which::which("node").is_ok()
}

async fn health_ok(client: &reqwest::Client, port: u16, token: &str) -> bool {
    let url = format!("http://127.0.0.1:{}/api/health", port);
    match client
        .get(&url)
        .bearer_auth(token)
        .timeout(Duration::from_millis(800))
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

#[tauri::command]
pub fn deck_status() -> DeckStatus {
    let guard = DECK.lock().unwrap();
    match guard.as_ref() {
        Some(p) => DeckStatus {
            running: true,
            port: p.port,
            url: local_url(p.port),
            lan_url: p.lan_url.clone(),
            token: p.token.clone(),
            qr_svg: p.qr_svg.clone(),
            message: "running".into(),
        },
        None => DeckStatus {
            running: false,
            message: "stopped".into(),
            ..Default::default()
        },
    }
}

#[tauri::command]
pub fn deck_stop() -> Result<(), String> {
    let taken = DECK.lock().unwrap().take();
    if let Some(p) = taken {
        crate::commands::kill_process(p.pid);
    }
    Ok(())
}

#[tauri::command]
pub fn deck_set_folder(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let dir = resolve_deck_dir(&path);
    if !runner_entry(&dir).exists() {
        return Err(format!(
            "No Deck server found at {} (expected server/runner.mjs).",
            dir.display()
        ));
    }
    let mut cfg = state.config.lock().unwrap();
    cfg.deck_path = path;
    crate::config::save(&cfg);
    Ok(())
}

#[tauri::command]
pub async fn deck_start(state: State<'_, AppState>) -> Result<DeckStatus, String> {
    if DECK_STARTING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("Deck is already starting — hold on.".into());
    }
    let _starting = StartingGuard; // resets the flag on every return/early-exit

    // One HTTP client reused across the idempotency check and the health poll.
    let client = reqwest::Client::new();

    // Idempotent: if already tracked and healthy, just return it.
    let existing = {
        let guard = DECK.lock().unwrap();
        guard.as_ref().map(|p| (p.pid, p.port, p.token.clone()))
    };
    if let Some((pid, port, token)) = existing {
        if health_ok(&client, port, &token).await {
            return Ok(deck_status());
        }
        // stale/unhealthy — kill the old process to free the port before restart
        crate::commands::kill_process(pid);
        DECK.lock().unwrap().take();
    }

    // Read config (guard dropped before any await/spawn).
    let (deck_path, port, mut token) = {
        let cfg = state.config.lock().unwrap();
        (cfg.deck_path.clone(), cfg.deck_port, cfg.deck_token.clone())
    };

    let dir = resolve_deck_dir(&deck_path);
    if !node_on_path() {
        return Err("Node.js not found on PATH. Install Node, or fix your PATH, then try again.".into());
    }
    if !runner_entry(&dir).exists() {
        return Err(format!(
            "Deck not found at {}. Set the Deck folder in this panel.",
            dir.display()
        ));
    }

    // Generate + persist a token on first use.
    if token.trim().is_empty() {
        token = gen_token();
        let mut cfg = state.config.lock().unwrap();
        cfg.deck_token = token.clone();
        crate::config::save(&cfg);
    }

    // Spawn the long-lived Node server. stdout/stderr MUST be null.
    let server_dir = dir.join("server");
    let mut cmd = Command::new("node");
    cmd.arg("runner.mjs")
        .current_dir(&server_dir)
        .env("DECK_TOKEN", &token)
        .env("PORT", port.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to launch node: {}", e))?;
    let pid = child.id();
    // Dropping a std Child does NOT kill it; the server keeps running. We track
    // the pid and kill via kill_process on stop/exit. On Unix, reap the child
    // once it exits so it can't linger as a zombie (Windows: taskkill handles it).
    #[cfg(not(target_os = "windows"))]
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    #[cfg(target_os = "windows")]
    drop(child);

    // Register immediately so deck_stop / the exit handler can kill it even
    // during the health-poll window. lan/qr get filled in on success below.
    *DECK.lock().unwrap() = Some(DeckProc {
        pid,
        port,
        token: token.clone(),
        lan_url: String::new(),
        qr_svg: String::new(),
    });

    // Health poll: up to 25 * 200ms = 5s.
    let mut up = false;
    for _ in 0..25 {
        if health_ok(&client, port, &token).await {
            up = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    if !up {
        crate::commands::kill_process(pid);
        DECK.lock().unwrap().take();
        return Err(format!(
            "Deck did not come up on port {} (is the port already in use?).",
            port
        ));
    }

    // LAN + QR (best-effort).
    let (lan, qr) = match lan_ip() {
        Some(ip) => (lan_url(&ip, port), qr_svg(&phone_url(&ip, port, &token))),
        None => (String::new(), String::new()),
    };
    // Fill in lan/qr on the already-registered entry (skip gracefully if the
    // exit handler cleared DECK during the poll).
    if let Some(p) = DECK.lock().unwrap().as_mut() {
        p.lan_url = lan;
        p.qr_svg = qr;
    }
    Ok(deck_status())
}

/// 16 random bytes as 32 lowercase hex chars. Uses the OS RNG.
pub fn gen_token() -> String {
    use std::fmt::Write;
    let mut buf = [0u8; 16];
    getrandom::getrandom(&mut buf).expect("OS RNG unavailable");
    let mut s = String::with_capacity(32);
    for b in buf {
        let _ = write!(s, "{:02x}", b);
    }
    s
}

/// The Deck project folder: explicit config path if set, else ~/trapline-deck.
pub fn resolve_deck_dir(config_deck_path: &str) -> PathBuf {
    if !config_deck_path.trim().is_empty() {
        PathBuf::from(config_deck_path.trim())
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("trapline-deck")
    }
}

/// The Node entry point inside a Deck folder: <dir>/server/runner.mjs
pub fn runner_entry(deck_dir: &Path) -> PathBuf {
    deck_dir.join("server").join("runner.mjs")
}

pub fn local_url(port: u16) -> String {
    format!("http://localhost:{}", port)
}

pub fn lan_url(ip: &str, port: u16) -> String {
    format!("http://{}:{}", ip, port)
}

/// Phone URL with the token embedded so scanning auto-authenticates
/// (Deck's server accepts `?token=`).
pub fn phone_url(ip: &str, port: u16, token: &str) -> String {
    format!("http://{}:{}/?token={}", ip, port, token)
}

/// Best-effort primary LAN IPv4. The UDP "connect" only makes the OS pick a
/// source interface; no packets are actually sent. Returns None if undetectable.
pub fn lan_ip() -> Option<String> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    match sock.local_addr().ok()?.ip() {
        IpAddr::V4(v4) if !v4.is_loopback() && !v4.is_unspecified() => Some(v4.to_string()),
        _ => None,
    }
}

/// Render `data` as an inline SVG QR code string. Empty string on error.
pub fn qr_svg(data: &str) -> String {
    use qrcode::render::svg;
    use qrcode::QrCode;
    match QrCode::new(data.as_bytes()) {
        Ok(code) => code
            .render::<svg::Color>()
            .min_dimensions(200, 200)
            .quiet_zone(true)
            .build(),
        Err(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_32_hex_and_random() {
        let a = gen_token();
        let b = gen_token();
        assert_eq!(a.len(), 32);
        assert!(
            a.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "token must be lowercase hex, got {a}"
        );
        assert_ne!(a, b, "two tokens should differ");
    }

    #[test]
    fn resolve_deck_dir_prefers_explicit_path() {
        let p = resolve_deck_dir("D:/somewhere/trapline-deck");
        assert_eq!(p, std::path::PathBuf::from("D:/somewhere/trapline-deck"));
    }

    #[test]
    fn resolve_deck_dir_defaults_to_home_subdir() {
        let p = resolve_deck_dir("   "); // blank/whitespace -> default
        assert!(p.ends_with("trapline-deck"), "got {}", p.display());
    }

    #[test]
    fn runner_entry_is_server_runner_mjs() {
        let p = runner_entry(std::path::Path::new("/x/trapline-deck"));
        assert!(p.ends_with("server/runner.mjs") || p.ends_with("server\\runner.mjs"));
    }

    #[test]
    fn url_builders_format_correctly() {
        assert_eq!(local_url(8787), "http://localhost:8787");
        assert_eq!(lan_url("192.168.1.24", 8787), "http://192.168.1.24:8787");
        assert_eq!(
            phone_url("192.168.1.24", 8787, "abc123"),
            "http://192.168.1.24:8787/?token=abc123"
        );
    }

    #[test]
    fn qr_svg_is_nonempty_svg() {
        let svg = qr_svg("http://192.168.1.24:8787/?token=abc123");
        assert!(svg.contains("<svg"), "expected an svg string");
    }

    #[test]
    fn deckstatus_serializes_camelcase() {
        let s = DeckStatus {
            running: true,
            port: 8787,
            url: "http://localhost:8787".into(),
            lan_url: "http://192.168.1.24:8787".into(),
            token: "abc".into(),
            qr_svg: "<svg/>".into(),
            message: "running".into(),
        };
        let j = serde_json::to_string(&s).unwrap();
        assert!(j.contains("\"lanUrl\""));
        assert!(j.contains("\"qrSvg\""));
        assert!(!j.contains("lan_url"));
    }

    #[test]
    fn status_when_stopped_is_not_running() {
        let s = deck_status();
        assert!(!s.running);
    }
}
