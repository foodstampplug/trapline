# Trapline "Deck ▸" Launcher — Design

**Date:** 2026-08-14
**Status:** Approved for planning
**Repo:** `trapline-v2` (Tauri v2 + Rust backend + vanilla JS/CSS frontend)

## Summary

Add a **Deck ▸** button to Trapline's header that starts/stops the local **Trapline Deck** server (the phone-first recon remote at `~/trapline-deck`) from inside the desktop app, and hands the user a **QR code** that opens Deck on their phone — pre-authenticated — over the local network.

Deck is a Node app (`server/runner.mjs`). Trapline launches it by **shelling out to Node** (decision made during brainstorming: quick path, works on this machine today; not a self-contained Rust port). If Node or the Deck folder is missing, the button fails with a clear, actionable message rather than silently.

## Goals

- One click in Trapline starts Deck and confirms it's actually serving (health-checked), then opens it.
- A **scannable QR** in the modal opens Deck on the phone and auto-authenticates, with no manual token entry, on the same Wi-Fi — no tunnel required.
- Stop Deck from the same modal; never leave an orphan `node` server behind (clean up on app exit too).
- Trapline owns the Deck auth token (its own, not the shared demo `testtoken123`).

## Non-goals (YAGNI — explicitly out of scope for v1)

- No bundling of Node or the Deck source into the Trapline exe.
- No Rust re-implementation of Deck's server.
- No SMS/Twilio handoff (considered and declined).
- No auto-launching or output-parsing of `cloudflared` (the tunnel command is shown for copy only).
- No tunnel-URL QR (LAN-URL QR only in v1; a "paste tunnel URL → regenerate QR" field is a noted later add).
- No multi-instance / multi-port management.

## Architecture

```
Trapline (Tauri webview)                     Deck (Node, spawned child)
  Header: [ Deck ▸ ]                            server/runner.mjs
     │ click                                      listens :PORT on 0.0.0.0
     ▼                                            token = env DECK_TOKEN
  #deck modal  ── invoke('deck_start') ──▶  deck.rs
     │                                        1. resolve deck folder
     │                                        2. verify node + runner.mjs
     │                                        3. get/generate token (Trapline config)
     │                                        4. spawn `node runner.mjs`
     │                                           (cwd=server, env DECK_TOKEN+PORT,
     │                                            CREATE_NO_WINDOW, stdout/stderr→null)
     │                                        5. health-poll 127.0.0.1:PORT/api/health
     │                                        6. detect LAN IP, render QR svg
     ◀── DeckStatus{running,url,lanUrl,token,port,qrSvg} ──┘
     │ render QR + copy fields
     ▼
  phone camera scans QR → http://<LAN-IP>:PORT/?token=<token> → Deck opens, authed
```

## Backend — new module `src-tauri/src/deck.rs`

State (process-lifetime static, mirrors the existing `commands::GLOBAL_PIDS` pattern):

```
struct DeckProc { pid: u32, port: u16, token: String }
static DECK: Lazy<Mutex<Option<DeckProc>>>
```

Returned status struct (serde camelCase, matches JS expectations):

```
DeckStatus {
  running: bool,
  port: u16,
  url: String,       // http://localhost:<port>  (for the PC-local "Open" button)
  lanUrl: String,    // http://<lan-ip>:<port>   ("" if LAN IP undetectable)
  token: String,
  qrSvg: String,     // inline SVG string of lanUrl+?token  ("" if no lanUrl)
  message: String,   // human status / last error detail
}
```

### Commands (registered in `lib.rs` `invoke_handler`)

1. **`deck_start(app, state) -> Result<DeckStatus, String>`** (async)
   - If `DECK` already holds a live PID and health passes → return current status (idempotent; no double-spawn).
   - Resolve Deck folder: `config.deckPath` if non-empty, else default `~/trapline-deck` (via `dirs::home_dir()`). The server dir is `<deckPath>/server`, entry `<deckPath>/server/runner.mjs`.
   - Preflight, each with a distinct error string:
     - `node` not on PATH → `"Node.js not found on PATH. Install Node, or check your PATH."`
     - `runner.mjs` missing → `"Deck not found at <path>. Set the Deck folder in this panel."`
   - Token: read `config.deckToken`; if empty, generate 32 hex chars, persist via `config::save` + update in-memory `AppState.config`.
   - Port: `config.deckPort` or default `8787`.
   - Spawn: `Command::new("node").arg("runner.mjs").current_dir(server_dir).env("DECK_TOKEN", token).env("PORT", port).stdout(null).stderr(null)` + `CREATE_NO_WINDOW` on Windows. **stdout/stderr MUST be null** — the server is long-lived; an unread piped stdout buffer would eventually block it.
   - Store `DeckProc` in `DECK`.
   - Health poll: up to 25 tries × 200ms, `GET http://127.0.0.1:<port>/api/health` with `Authorization: Bearer <token>` via `reqwest` (async; already a dependency). First 200 → proceed. If it never comes up → kill the PID, clear `DECK`, return `"Deck did not come up on port <port> (is the port in use?)"`.
   - Compute `lanUrl` + `qrSvg` (see below). Return `DeckStatus{running:true, …}`.

2. **`deck_stop() -> Result<(), String>`**
   - If `DECK` holds a PID, `taskkill /F /T /PID` (Windows) / kill process group (unix) — reuse the same `kill_process` approach as `commands::cancel_command`. Clear `DECK`.
   - No-op success if nothing is running.

3. **`deck_status() -> DeckStatus`**
   - If `DECK` is set, return the cached status with `running:true` (best-effort; does not re-health-check to keep it cheap). Otherwise `running:false` with default fields.

4. **`deck_set_folder(path, state) -> Result<(), String>`**
   - Persist `config.deckPath` (validates the path exists and contains `server/runner.mjs`; returns a clear error otherwise). Lets the user fix a wrong path from the modal.

### LAN IP detection (zero-dependency)

```
fn lan_ip() -> Option<Ipv4Addr> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;   // sets default route dest; sends nothing
    match sock.local_addr().ok()?.ip() { IpAddr::V4(v4) if !v4.is_loopback() => Some(v4), _ => None }
}
```
No packets are actually transmitted by a UDP `connect`; it only makes the OS pick the source interface. If it returns `None`, `lanUrl`/`qrSvg` are empty and the modal shows only the localhost + tunnel paths.

### QR generation

- Add the **`qrcode`** crate. Render `http://<lan-ip>:<port>/?token=<token>` to an **SVG string** (`qrcode`'s string/SVG renderer — no `image` feature, no raster deps).
- Token is embedded as `?token=` because Deck's server already authenticates via `url.searchParams.get('token')`. Scanning therefore both opens and authenticates.
- The SVG string is returned in `DeckStatus.qrSvg` and injected into the modal (`innerHTML`) — it is app-generated, not remote content.

### App-exit cleanup (`lib.rs`)

- In `run()`, handle `tauri::RunEvent::ExitRequested` (or a window `CloseRequested`) to best-effort `deck_stop()` the tracked PID, so quitting Trapline doesn't leave a `node` server running.

## Config changes — `src-tauri/src/config.rs`

Add three fields to `Config`, all `#[serde(default)]` (camelCase) so existing `config.json` files keep loading unchanged:

- `deck_path: String` (default `""` → resolved to `~/trapline-deck` at runtime)
- `deck_port: u16` (default `8787`; a `default_deck_port` fn since serde default for non-empty needs it)
- `deck_token: String` (default `""` → generated on first start)

## Frontend — `index.html`, `src/main.js`, `src/style.css`

- **Header button:** add `<button id="deckBtn">Deck ▸</button>` next to `#settingsBtn`. `deckBtn.onclick` opens the modal and calls `deck_status` to render current state.
- **`#deck` modal:** new element mirroring the existing `#settings` modal (same overlay/panel classes + `.hidden` toggle), containing:
  - Status row: a dot + text — `stopped` / `starting…` / `running`.
  - **Start** and **Stop** buttons (Start disabled while starting; Stop shown only when running).
  - When running:
    - **QR** block (injects `qrSvg`); caption = `lanUrl` with a **copy** button. If `lanUrl` is empty, show "Couldn't detect a LAN IP — use the tunnel command below."
    - **Token** field (read-only) + **copy**.
    - **Tunnel** command `cloudflared tunnel --url http://localhost:<port>` + **copy**, with the note "same Wi-Fi? just scan the QR."
    - **Open** button → `invoke('open_url', { url })` (localhost, for testing on the PC).
    - One-line caveat: "First phone connection may need Windows Firewall to allow node.exe (one-time)."
  - **Advanced:** a **Deck folder** text input (prefilled with resolved default) + **Save** → `invoke('deck_set_folder', {path})`; used to fix a "not found" error.
- **Interactions:** `invoke('deck_start')` / `deck_stop` / `deck_status`; copy via `navigator.clipboard.writeText`; errors surface through the existing `toast()` helper. Reuse existing button/modal styles; add only minimal CSS (QR container sizing, monospace token/URL rows).

## Error handling (explicit, user-facing)

| Condition | Behavior |
|---|---|
| `node` not on PATH | Start fails; toast + modal message: install Node / fix PATH. |
| Deck folder / `runner.mjs` missing | Start fails; message points at the folder field. |
| Port already in use | Health poll fails → kill + clear → "did not come up (port in use?)". |
| LAN IP undetectable | Modal hides QR, shows tunnel path; not an error. |
| Already running | `deck_start` is idempotent — returns existing status. |
| Trapline quits while running | Exit handler kills the child. |

## Testing

- `npm run tauri dev` (dev), then a real `npm run tauri build` smoke.
- Manual matrix:
  1. **Happy path:** Deck ▸ → Start → health passes, QR + LAN URL + token render, **Open** loads Deck locally, **Stop** kills it (verify no `node` in Task Manager).
  2. **QR handoff:** scan with a same-Wi-Fi phone → Deck opens **already authenticated** (no token prompt). (Allow node.exe through Firewall once if prompted.)
  3. **Node missing:** temporarily shadow PATH → Start shows the Node error, no crash.
  4. **Wrong folder:** set a bad `deckPath` → clear "not found" error; fix via the folder field → Start works.
  5. **Idempotent Start:** click Start twice → single server, no second process.
  6. **Exit cleanup:** Start Deck, quit Trapline → server process is gone.
  7. **Backward-compat:** load a pre-existing `config.json` with none of the new fields → app starts, defaults applied.

## Risks / caveats

- **Node + folder dependency** (accepted): a shipped exe on a buyer's machine without Node or the Deck folder can't launch Deck — handled by explicit errors, not silent failure. Self-contained Rust port remains the future option if Deck goes to buyers.
- **Windows Firewall** may prompt to allow `node.exe` inbound on the first LAN connection — surfaced as a one-line note in the modal.
- **Token in QR URL** (`?token=`): acceptable for a LAN URL to your own tool on your own network; Deck already supports query-token auth. Not used for the tunnel path (tunnel is copy-only).
- **`qrcode` crate** adds one small build dependency (SVG string rendering only; no raster/image deps).
