# Deck Launcher Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a header "Deck ▸" button to Trapline that starts/stops the local Trapline Deck Node server, health-checks it, and shows a LAN QR code (with the token embedded) that opens Deck on a phone, pre-authenticated.

**Architecture:** A new Rust module `deck.rs` shells out to `node runner.mjs` (cwd = the Deck folder's `server/`), passes the auth token via env, polls `/api/health` over `reqwest`, detects the LAN IP with a zero-dependency UDP trick, and renders a QR SVG with the `qrcode` crate. Four Tauri commands (`deck_start`/`deck_stop`/`deck_status`/`deck_set_folder`) drive a new `#deck` modal in the vanilla-JS frontend that mirrors the existing `#settings` modal.

**Tech Stack:** Tauri v2, Rust (serde, reqwest async, once_cell, dirs, + new `qrcode` and `getrandom`), vanilla JS/CSS frontend built by Vite. Node.js on the user's PATH runs Deck.

## Global Constraints

- **Config compatibility:** every new `Config` field is `#[serde(default)]` (camelCase) — an existing `%APPDATA%\Trapline\config.json` with none of the new fields MUST still load.
- **Shell-out model only:** launch Deck via `node runner.mjs`; do NOT bundle Node or re-implement Deck in Rust.
- **Long-lived child:** the spawned `node` process's stdout/stderr MUST be `Stdio::null()` (an unread pipe would eventually block the server). On Windows add `CREATE_NO_WINDOW` (`0x08000000`).
- **No MutexGuard held across an `.await`** in async commands (Tauri requires `Send` futures; `std::sync::MutexGuard` is not `Send`).
- **Token model:** Trapline owns the token (generate its own; never the demo `testtoken123`) and passes it to Deck via `DECK_TOKEN`.
- **Default port:** `8787`. **Default Deck folder:** `~/trapline-deck`.
- **Follow existing patterns:** new modal reuses classes `modal`, `modal-card`, `modal-head`, `modal-sub`, `modal-actions`, `spacer`, `run-btn`, `ghost-btn`, `icon-btn`; JS reuses the in-scope `$()`, `invoke()`, `toast()`, `errMsg()` helpers.
- **Branch:** all work on `deck-launcher` (already created).

---

### Task 1: Config fields (backward-compatible)

**Files:**
- Modify: `src-tauri/src/config.rs`
- Modify: `src-tauri/Cargo.toml` (add deps used by later tasks, done here so one `cargo build` covers them)

**Interfaces:**
- Produces: `Config { deck_path: String, deck_port: u16, deck_token: String }` (plus existing fields), serde camelCase `deckPath`/`deckPort`/`deckToken`.

- [ ] **Step 1: Add the two new crates to `Cargo.toml`**

In `[dependencies]` of `src-tauri/Cargo.toml`, add:

```toml
qrcode = "0.14"
getrandom = "0.2"
```

(`qrcode` is used in Task 2 for SVG rendering; `getrandom` for token generation. Added now so the workspace compiles once.)

- [ ] **Step 2: Write the failing test for backward-compat + defaults**

Append to `src-tauri/src/config.rs`:

```rust
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
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cd src-tauri && cargo test --lib config::tests`
Expected: FAIL — `no field 'deck_port' on type 'Config'`.

- [ ] **Step 4: Add the fields + default fn**

In `src-tauri/src/config.rs`, add to the `Config` struct (after `community_discord`):

```rust
    #[serde(default)]
    pub deck_path: String,
    #[serde(default = "default_deck_port")]
    pub deck_port: u16,
    #[serde(default)]
    pub deck_token: String,
```

Add near `default_username`:

```rust
fn default_deck_port() -> u16 {
    8787
}
```

In the `impl Default for Config` block, add to the returned struct:

```rust
            deck_path: String::new(),
            deck_port: 8787,
            deck_token: String::new(),
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cd src-tauri && cargo test --lib config::tests`
Expected: PASS (2 tests).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/config.rs src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat(deck): add backward-compatible deck config fields"
```

---

### Task 2: `deck.rs` pure helpers

**Files:**
- Create: `src-tauri/src/deck.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod deck;` so the module compiles and its tests run)

**Interfaces:**
- Produces (used by Task 3):
  - `gen_token() -> String` (32 lowercase hex chars)
  - `resolve_deck_dir(config_deck_path: &str) -> std::path::PathBuf`
  - `runner_entry(deck_dir: &std::path::Path) -> std::path::PathBuf`
  - `local_url(port: u16) -> String`, `lan_url(ip: &str, port: u16) -> String`, `phone_url(ip: &str, port: u16, token: &str) -> String`
  - `lan_ip() -> Option<String>`
  - `qr_svg(data: &str) -> String`

- [ ] **Step 1: Register the module**

In `src-tauri/src/lib.rs`, add to the top module list (after `mod config;`):

```rust
mod deck;
```

- [ ] **Step 2: Write the failing tests**

Create `src-tauri/src/deck.rs` with ONLY the tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_32_hex_and_random() {
        let a = gen_token();
        let b = gen_token();
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
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
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cd src-tauri && cargo test --lib deck::tests`
Expected: FAIL — `cannot find function 'gen_token'` etc.

- [ ] **Step 4: Implement the helpers**

Prepend to `src-tauri/src/deck.rs` (above the test module):

```rust
use std::net::{IpAddr, UdpSocket};
use std::path::{Path, PathBuf};

/// 16 random bytes as 32 lowercase hex chars. Uses the OS RNG.
pub fn gen_token() -> String {
    let mut buf = [0u8; 16];
    getrandom::getrandom(&mut buf).expect("OS RNG unavailable");
    buf.iter().map(|b| format!("{:02x}", b)).collect()
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
        IpAddr::V4(v4) if !v4.is_loopback() => Some(v4.to_string()),
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
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cd src-tauri && cargo test --lib deck::tests`
Expected: PASS (6 tests).
If `qrcode`'s `render::svg` path fails to resolve, run `cargo tree -p qrcode` and confirm the version; the `render` module is not feature-gated in 0.14, so a plain `qrcode = "0.14"` is sufficient.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/deck.rs src-tauri/src/lib.rs
git commit -m "feat(deck): pure helpers for token, paths, urls, lan-ip, qr"
```

---

### Task 3: `deck.rs` process control + Tauri commands + wiring

**Files:**
- Modify: `src-tauri/src/deck.rs` (add state, status struct, commands)
- Modify: `src-tauri/src/commands.rs` (make `kill_process` reusable)
- Modify: `src-tauri/src/lib.rs` (register commands + app-exit cleanup)

**Interfaces:**
- Consumes (from Task 2): `gen_token`, `resolve_deck_dir`, `runner_entry`, `local_url`, `lan_url`, `phone_url`, `lan_ip`, `qr_svg`.
- Consumes (from Task 1): `Config.deck_path`, `Config.deck_port`, `Config.deck_token`.
- Produces (used by Task 5): commands `deck_start` → `DeckStatus`, `deck_stop` → `()`, `deck_status` → `DeckStatus`, `deck_set_folder(path)` → `()`. `DeckStatus` serializes camelCase: `running, port, url, lanUrl, token, qrSvg, message`.

- [ ] **Step 1: Make `kill_process` reusable**

In `src-tauri/src/commands.rs`, change both `kill_process` definitions from `fn kill_process` to `pub(crate) fn kill_process` (the Windows `#[cfg(target_os = "windows")]` one and the non-Windows one).

- [ ] **Step 2: Write the failing tests for status serialization**

Add these tests to the existing `#[cfg(test)] mod tests` in `src-tauri/src/deck.rs`:

```rust
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
```

- [ ] **Step 3: Run to verify failure**

Run: `cd src-tauri && cargo test --lib deck::tests`
Expected: FAIL — `cannot find type 'DeckStatus'` / `cannot find function 'deck_status'`.

- [ ] **Step 4: Implement state, status, and commands**

Add to the top of `src-tauri/src/deck.rs` (below the `use` lines from Task 2):

```rust
use crate::AppState;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::process::{Command, Stdio};
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
    #[cfg(target_os = "windows")]
    let finder = "where";
    #[cfg(not(target_os = "windows"))]
    let finder = "which";
    Command::new(finder)
        .arg("node")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn health_ok(port: u16, token: &str) -> bool {
    let url = format!("http://127.0.0.1:{}/api/health", port);
    let client = reqwest::Client::new();
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
    // Idempotent: if already tracked and healthy, just return it.
    let existing = {
        let guard = DECK.lock().unwrap();
        guard.as_ref().map(|p| (p.port, p.token.clone()))
    };
    if let Some((port, token)) = existing {
        if health_ok(port, &token).await {
            return Ok(deck_status());
        }
        // stale — drop and restart
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
    // pid and kill via kill_process on stop/exit.

    // Health poll: up to 25 * 200ms = 5s.
    let mut up = false;
    for _ in 0..25 {
        if health_ok(port, &token).await {
            up = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    if !up {
        crate::commands::kill_process(pid);
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

    *DECK.lock().unwrap() = Some(DeckProc {
        pid,
        port,
        token,
        lan_url: lan,
        qr_svg: qr,
    });
    Ok(deck_status())
}
```

- [ ] **Step 5: Register commands + app-exit cleanup in `lib.rs`**

In `src-tauri/src/lib.rs`, add the four commands to the `tauri::generate_handler!` list (after `commands::clear_session,`):

```rust
            deck::deck_start,
            deck::deck_stop,
            deck::deck_status,
            deck::deck_set_folder,
```

Then replace the final `.run(tauri::generate_context!()) .expect("error while running Trapline");` with a build+run that stops Deck on exit:

```rust
        .build(tauri::generate_context!())
        .expect("error while building Trapline")
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                let _ = deck::deck_stop();
            }
        });
```

- [ ] **Step 6: Run the tests + a full compile**

Run: `cd src-tauri && cargo test --lib deck::tests && cargo build`
Expected: tests PASS (8 total in deck), `cargo build` succeeds.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/deck.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(deck): start/stop/status/set-folder commands + exit cleanup"
```

---

### Task 4: Frontend — header button, `#deck` modal, styles

**Files:**
- Modify: `index.html` (header button + new modal)
- Modify: `src/style.css` (QR + row styling)

**Interfaces:**
- Produces (used by Task 5): DOM ids `#deckBtn`, `#deck`, `#deckStatusDot`, `#deckStatusText`, `#deckStartBtn`, `#deckStopBtn`, `#deckRunning`, `#deckQr`, `#deckLanUrl`, `#deckLanRow`, `#deckToken`, `#deckTunnelCmd`, `#deckOpenBtn`, `#deckCopyLan`, `#deckCopyToken`, `#deckCopyTunnel`, `#deckFolder`, `#deckSaveFolder`, `#closeDeck`.

- [ ] **Step 1: Add the header button**

In `index.html`, inside `<div class="topbar-actions">`, add immediately before the `settingsBtn` line:

```html
      <button id="deckBtn" class="icon-btn" title="Deck — run the phone remote">📡</button>
```

- [ ] **Step 2: Add the `#deck` modal**

In `index.html`, add right after the closing `</div>` of the Settings modal (after the `id="settings"` block, before `id="findingsPanel"`):

```html
  <!-- Deck launcher modal -->
  <div id="deck" class="modal hidden">
    <div class="modal-card">
      <div class="modal-head">
        <h2>Deck <span class="deck-dot" id="deckStatusDot"></span><span id="deckStatusText" class="muted-count">stopped</span></h2>
        <button id="closeDeck" class="icon-btn sm" title="Close">✕</button>
      </div>
      <p class="modal-sub">Run the phone remote on this machine, then scan the QR to open it on your phone — already logged in — over your Wi-Fi.</p>

      <div class="deck-controls">
        <button id="deckStartBtn" class="run-btn">Start Deck ▸</button>
        <button id="deckStopBtn" class="ghost-btn hidden">Stop</button>
        <button id="deckOpenBtn" class="ghost-btn hidden">Open on this PC</button>
      </div>

      <div id="deckRunning" class="deck-running hidden">
        <div class="deck-qr" id="deckQr"></div>
        <div id="deckLanRow" class="deck-row">
          <label>On this Wi-Fi</label>
          <div class="deck-copy"><code id="deckLanUrl"></code><button id="deckCopyLan" class="ghost-btn sm">Copy</button></div>
        </div>
        <div class="deck-row">
          <label>Token</label>
          <div class="deck-copy"><code id="deckToken" class="mono"></code><button id="deckCopyToken" class="ghost-btn sm">Copy</button></div>
        </div>
        <div class="deck-row">
          <label>Off your network? Run a tunnel, then open the printed URL</label>
          <div class="deck-copy"><code id="deckTunnelCmd" class="mono"></code><button id="deckCopyTunnel" class="ghost-btn sm">Copy</button></div>
        </div>
        <p class="deck-note">Same Wi-Fi? Just scan the QR. First phone connection may need Windows Firewall to allow <b>node.exe</b> (one-time).</p>
      </div>

      <details class="deck-adv">
        <summary>Advanced — Deck folder</summary>
        <div class="deck-copy">
          <input id="deckFolder" type="text" spellcheck="false" placeholder="C:\Users\you\trapline-deck" />
          <button id="deckSaveFolder" class="ghost-btn sm">Save</button>
        </div>
      </details>

      <div class="modal-actions">
        <span class="spacer"></span>
        <button id="closeDeckBtn" class="ghost-btn">Close</button>
      </div>
    </div>
  </div>
```

- [ ] **Step 3: Add styles**

Append to `src/style.css`:

```css
/* ── Deck launcher ─────────────────────────────────────────── */
.deck-dot { display:inline-block; width:8px; height:8px; border-radius:50%;
  background:#666; margin:0 6px 0 10px; vertical-align:middle; }
.deck-dot.running { background:#39d353; box-shadow:0 0 8px #39d353; }
.deck-dot.starting { background:#e3b341; box-shadow:0 0 8px #e3b341; }
.deck-controls { display:flex; gap:10px; margin:4px 0 14px; }
.deck-running { display:flex; flex-direction:column; gap:12px; }
.deck-qr { align-self:center; width:220px; height:220px; padding:10px;
  background:#fff; border-radius:10px; }
.deck-qr svg { width:100%; height:100%; display:block; }
.deck-row { display:flex; flex-direction:column; gap:4px; }
.deck-row > label { font-size:12px; color:var(--muted, #8a8f98); }
.deck-copy { display:flex; gap:8px; align-items:center; }
.deck-copy code, .deck-copy input { flex:1; padding:8px 10px; border-radius:8px;
  background:rgba(255,255,255,0.05); border:1px solid rgba(255,255,255,0.08);
  font-size:12px; overflow:auto; white-space:nowrap; }
.deck-note { font-size:11px; color:var(--muted, #8a8f98); margin:2px 0 0; }
.deck-adv { margin-top:14px; font-size:13px; }
.deck-adv summary { cursor:pointer; color:var(--muted, #8a8f98); }
.deck-adv .deck-copy { margin-top:8px; }
```

- [ ] **Step 4: Verify it renders (dev)**

Run: `npm run tauri dev`
Expected: app builds; the 📡 button appears in the header; clicking it does nothing yet (wired in Task 5). No console errors about missing elements. Close the app.

- [ ] **Step 5: Commit**

```bash
git add index.html src/style.css
git commit -m "feat(deck): header button + deck modal markup and styles"
```

---

### Task 5: Frontend — wire the modal to the commands

**Files:**
- Modify: `src/main.js` (add a Deck section + bottom wiring)

**Interfaces:**
- Consumes (from Task 3): `invoke('deck_start')` → `{running, port, url, lanUrl, token, qrSvg, message}`, `invoke('deck_stop')`, `invoke('deck_status')`, `invoke('deck_set_folder', { path })`, and existing `invoke('open_url', { url })`.
- Consumes (from Task 4): the `#deck*` DOM ids.
- Reuses in-scope helpers: `$()`, `invoke()`, `toast()`, `errMsg()`.

- [ ] **Step 1: Add the Deck UI logic**

In `src/main.js`, near the settings section (search for `function openSettings`), add:

```js
// ── Deck launcher ───────────────────────────────────────────────────────────
function deckDot(state) {
  const dot = $("#deckStatusDot");
  const txt = $("#deckStatusText");
  dot.classList.remove("running", "starting");
  if (state === "running") { dot.classList.add("running"); txt.textContent = "running"; }
  else if (state === "starting") { dot.classList.add("starting"); txt.textContent = "starting…"; }
  else { txt.textContent = "stopped"; }
}

function renderDeck(s) {
  const running = !!(s && s.running);
  deckDot(running ? "running" : "stopped");
  $("#deckStartBtn").classList.toggle("hidden", running);
  $("#deckStopBtn").classList.toggle("hidden", !running);
  $("#deckOpenBtn").classList.toggle("hidden", !running);
  $("#deckRunning").classList.toggle("hidden", !running);
  if (running) {
    $("#deckQr").innerHTML = s.qrSvg || "";
    const hasLan = !!s.lanUrl;
    $("#deckLanRow").classList.toggle("hidden", !hasLan);
    $("#deckLanUrl").textContent = s.lanUrl || "";
    $("#deckToken").textContent = s.token || "";
    $("#deckTunnelCmd").textContent = `cloudflared tunnel --url http://localhost:${s.port}`;
    $("#deckOpenBtn").dataset.url = s.url || "";
    if (!hasLan) $("#deckQr").innerHTML =
      '<p style="color:#333;font-size:12px;padding:20px">Couldn\'t detect a LAN IP — use the tunnel command below.</p>';
  }
}

async function openDeck() {
  $("#deck").classList.remove("hidden");
  try { renderDeck(await invoke("deck_status")); }
  catch (e) { toast("Deck: " + errMsg(e), "err"); }
}
function closeDeck() { $("#deck").classList.add("hidden"); }

async function deckStart() {
  deckDot("starting");
  $("#deckStartBtn").disabled = true;
  try {
    const s = await invoke("deck_start");
    renderDeck(s);
    toast("Deck is running ✓", "ok");
  } catch (e) {
    deckDot("stopped");
    toast(errMsg(e), "err");
  } finally {
    $("#deckStartBtn").disabled = false;
  }
}

async function deckStop() {
  try { await invoke("deck_stop"); renderDeck({ running: false }); toast("Deck stopped", "ok"); }
  catch (e) { toast("Deck: " + errMsg(e), "err"); }
}

async function deckSaveFolder() {
  const path = $("#deckFolder").value.trim();
  try { await invoke("deck_set_folder", { path }); toast("Deck folder saved ✓", "ok"); }
  catch (e) { toast(errMsg(e), "err"); }
}

async function copyText(text, okMsg) {
  try { await navigator.clipboard.writeText(text); toast(okMsg, "ok"); }
  catch { toast("Copy failed", "err"); }
}
```

- [ ] **Step 2: Add the bottom wiring**

In `src/main.js`, find the block near `$("#settingsBtn").onclick = openSettings;` and add:

```js
$("#deckBtn").onclick = openDeck;
$("#closeDeck").onclick = closeDeck;
$("#closeDeckBtn").onclick = closeDeck;
$("#deckStartBtn").onclick = deckStart;
$("#deckStopBtn").onclick = deckStop;
$("#deckOpenBtn").onclick = () => {
  const url = $("#deckOpenBtn").dataset.url;
  if (url) invoke("open_url", { url }).catch((e) => toast(errMsg(e), "err"));
};
$("#deckSaveFolder").onclick = deckSaveFolder;
$("#deckCopyLan").onclick = () => copyText($("#deckLanUrl").textContent, "LAN URL copied ✓");
$("#deckCopyToken").onclick = () => copyText($("#deckToken").textContent, "Token copied ✓");
$("#deckCopyTunnel").onclick = () => copyText($("#deckTunnelCmd").textContent, "Tunnel command copied ✓");
```

- [ ] **Step 3: Verify wiring compiles (dev)**

Run: `npm run tauri dev`
Expected: app builds with no console errors; the 📡 button opens the Deck modal showing "stopped". (Full behavior verified in Task 6.)

- [ ] **Step 4: Commit**

```bash
git add src/main.js
git commit -m "feat(deck): wire deck modal to start/stop/status/open/copy"
```

---

### Task 6: End-to-end verification + build smoke

**Files:** none (verification only).

- [ ] **Step 1: Happy path (dev)**

Run: `npm run tauri dev`. Ensure `~/trapline-deck` exists and `node` is on PATH.
- Click 📡 → **Start Deck ▸**.
- Expected: dot goes green "running"; QR renders; LAN URL like `http://192.168.x.x:8787`; a 32-hex token; tunnel command shows port 8787.
- Click **Open on this PC** → browser opens Deck; paste token → Deck loads.

- [ ] **Step 2: QR handoff (real phone, same Wi-Fi)**

Scan the QR with a phone on the same Wi-Fi. Expected: Deck opens **already authenticated** (no token prompt). If the phone can't connect, allow `node.exe` through Windows Firewall once and retry.

- [ ] **Step 3: Stop + no orphan**

Click **Stop**. Expected: dot → "stopped", running panel hides. In Task Manager / `Get-Process node`, confirm the Deck `node` process is gone.

- [ ] **Step 4: Idempotent Start**

Start Deck, then click 📡 → **Start Deck ▸** again (re-open modal first if needed). Expected: still exactly one `node` process (no second spawn); status returns running.

- [ ] **Step 5: Error paths**

- Wrong folder: open **Advanced**, set `deckFolder` to a bad path, Save, then Start → expect the clear "Deck not found at …" toast, no crash. Restore the correct path via Save.
- Port busy: with Deck running, note that a second independent `node runner.mjs` on 8787 would fail; not required to force, but confirm the health-timeout message wording exists in `deck.rs`.

- [ ] **Step 6: Exit cleanup**

Start Deck, then quit the Trapline window. Run `Get-Process node -ErrorAction SilentlyContinue`. Expected: the Deck process is gone (killed by the exit handler).

- [ ] **Step 7: Backward-compat + release build**

- Confirm your existing `%APPDATA%\Trapline\config.json` still loaded fine (app opened normally in Step 1).
- Run: `npm run tauri build`. Expected: NSIS + MSI build succeed. Launch the built exe once and repeat Step 1's happy path.

- [ ] **Step 8: Final commit / branch ready**

```bash
git add -A
git commit -m "test(deck): manual verification notes" --allow-empty
```
Branch `deck-launcher` is ready for review/merge.

---

## Self-Review

**Spec coverage:**
- Header button + modal → Tasks 4/5. ✓
- Shell-out `node runner.mjs`, env token, null stdio, CREATE_NO_WINDOW → Task 3 Step 4. ✓
- Preflight errors (node/folder), port-busy via health timeout → Task 3. ✓
- Token owned by Trapline, generated + persisted → Task 1 (field) + Task 3 (gen/persist). ✓
- Health poll via reqwest → Task 3 `health_ok`. ✓
- LAN IP detection + QR SVG with `?token=` → Task 2 + Task 3 assembly. ✓
- `deck_set_folder` self-service → Task 3 + Task 5. ✓
- App-exit cleanup → Task 3 Step 5. ✓
- Config backward-compat → Task 1 tests. ✓
- Modal fields (QR, LAN row, token, tunnel cmd, open, firewall note, advanced folder) → Task 4. ✓
- Non-goals (no SMS, no Node bundling, no Rust port, no auto-tunnel) → respected. ✓

**Placeholder scan:** No TBD/TODO; all steps carry real code. ✓

**Type consistency:** `DeckStatus` fields (`running/port/url/lan_url/qr_svg/token/message`) match between Task 3 Rust and Task 5 JS (`lanUrl`/`qrSvg` via camelCase). Helper names (`resolve_deck_dir`, `runner_entry`, `phone_url`, `lan_ip`, `qr_svg`, `gen_token`) are consistent across Tasks 2/3. Command names (`deck_start/deck_stop/deck_status/deck_set_folder`) consistent across Tasks 3/5 and DOM ids consistent across Tasks 4/5. ✓
