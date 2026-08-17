# Phase 2 — Watch Merge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Absorb the standalone `trapline-watch` change-detection engine into the Trapline app as a built-in, in-process, monitor-while-open feature — wired to the new cockpit UI, writing findings directly into the app's findings store, with no license and no separate binary.

**Architecture:** Port the seven pure engine modules (`fetch/normalize/parse/score/store/sourcemap/engine`) into `src-tauri/src/watch/` with their logic **unchanged**. Rewire only three edges: (1) config now comes from the app `Config` (JSON at `%APPDATA%\Trapline\config.json`); (2) Discord alerts route through `discord.rs`; (3) findings write **directly** into the in-process `findings.rs` store — eliminating the old cross-process `findings.json` sync and its last-writer-wins race. A background scheduler thread (the `deck.rs` `AtomicBool` + `Mutex` pattern) runs cycles on its own thread with blocking `reqwest`, controlled by `watch_*` commands, auto-resuming on launch if it was on. The frontend lights up the already-present Watch surfaces (right dock, app rail, Settings) off a `watch` store fed by `watch:status` / `watch:new-finding` events.

**Tech Stack:** Rust (Tauri v2, rusqlite bundled, blocking reqwest, scraper, sha2, url, anyhow), SvelteKit 2 + Svelte 5 runes, TypeScript, Vitest.

**Spec:** `docs/superpowers/specs/2026-08-15-free-cockpit-rebuild-design.md` (§4.2, §4.3, §5, §6)

## Global Constraints

- **Engine logic unchanged.** The seven ported modules keep their behavior byte-for-byte. The ONLY permitted edits are (a) `use` paths (`crate::x` → `super::x`) and (b) the three rewired edges in `engine.rs`'s `run_target` tail. Do not "improve" the diff/extract/score/sourcemap logic.
- **Keep the safety gates.** The per-target `in_scope` host gate and the global `max_requests_per_min` rate limit are preserved exactly.
- **Findings severity is lowercase.** The frontend keys severity as `critical` / `high` / `medium` / `low` / `info` (see `src/lib/data/cvss.ts`, `FindingsPanel.svelte` `SEV_ORDER`). Watch findings MUST store `severity` lowercase — map `score::severity(top)` (`"Critical"`…) via `.to_lowercase()`.
- **No new secrets this phase.** Shodan/LeakCheck API keys are Phase 3. Watch adds no credential fields.
- **CSP unchanged, no new frontend deps, no CDN.** The strict CSP shipped in PR #6 stays valid: same-origin only, no external fetch/script/style/font.
- **No `{@html}` on untrusted content.** Watch finding text (target JS-derived values) flows through Svelte text bindings only. The engine's `parse::add` already strips `<`/`>` from artifact values — keep that guard.
- **Watch-while-open only.** No tray, no headless, no autostart. Auto-resume means: on app launch, if `watchEnabled` is true, start the scheduler.
- **Commit after every task.** Conventional Commits; repo-local `noreply` email. Backend tests: `cd src-tauri && cargo test`. Frontend tests: `npm test` + `npm run check`.

---

### Task 1: App Config — watch fields + `WatchTarget` + runtime-field preservation

**Files:**
- Modify: `src-tauri/src/config.rs`

**Interfaces:**
- Produces: `config::WatchTarget { name: String, pages: Vec<String>, js: Vec<String>, in_scope: Vec<String>, auto_enrich: bool }` (serde camelCase, all `#[serde(default)]`); new `Config` fields `watch_targets: Vec<WatchTarget>`, `watch_interval_secs: u64`, `watch_alert_threshold: i64`, `watch_max_rpm: u32`, `watch_enabled: bool` (camelCase JSON). `preserve_deck_fields` additionally carries `watch_enabled` from `current` (it is runtime-only state, not a Settings-form field, so a Settings save must not clobber it).

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)] mod tests` block in `src-tauri/src/config.rs`:

```rust
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
```

- [ ] **Step 2: Run them → FAIL**

Run: `cd src-tauri && cargo test config::tests`
Expected: FAIL (no `WatchTarget`, no `watch_*` fields).

- [ ] **Step 3: Implement**

In `src-tauri/src/config.rs`:

Add the struct above `Config` (or below it):
```rust
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
```

Add these fields to `Config` (after `deck_token`):
```rust
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
```

Add the default fns (next to `default_deck_port`):
```rust
fn default_watch_interval() -> u64 { 1800 }
fn default_watch_threshold() -> i64 { 50 }
fn default_watch_rpm() -> u32 { 30 }
```

Update `impl Default for Config` to add:
```rust
            watch_targets: Vec::new(),
            watch_interval_secs: 1800,
            watch_alert_threshold: 50,
            watch_max_rpm: 30,
            watch_enabled: false,
```

Extend `preserve_deck_fields` — add one line before `incoming` is returned, and update its doc comment to mention watch_enabled:
```rust
    incoming.watch_enabled = current.watch_enabled;
```

- [ ] **Step 4: Run tests → PASS**

Run: `cd src-tauri && cargo test config::tests`
Expected: PASS (all config tests, incl. the three existing deck tests, green).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/config.rs
git commit -m "feat(watch): add watch config fields + WatchTarget + preserve watch_enabled"
```

---

### Task 2: Port the pure engine modules

**Files:**
- Create: `src-tauri/src/watch/mod.rs`, `src-tauri/src/watch/config.rs`, `src-tauri/src/watch/fetch.rs`, `src-tauri/src/watch/normalize.rs`, `src-tauri/src/watch/parse.rs`, `src-tauri/src/watch/score.rs`, `src-tauri/src/watch/store.rs`, `src-tauri/src/watch/sourcemap.rs`
- Modify: `src-tauri/Cargo.toml` (deps), `src-tauri/src/lib.rs` (`mod watch;`)

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `watch::config::Config { interval_secs: u64, alert_threshold: i64, discord_webhook: String, discord_username: String, database: String, user_agent: String, max_requests_per_min: u64, probe_source_maps: bool, targets: Vec<Target> }` and `watch::config::Target { name: String, pages: Vec<String>, js: Vec<String>, in_scope: Vec<String> }` (both `#[derive(Clone)]`); plus `watch::fetch::{Fetched, Fetcher}`, `watch::normalize::logical_url`, `watch::parse::{extract, Artifact, Kind}`, `watch::score::{score, severity, color}`, `watch::store::Store`, `watch::sourcemap::*`. These are consumed by Tasks 3–5.

- [ ] **Step 1: Add dependencies**

In `src-tauri/Cargo.toml`, under `[dependencies]`, add `"blocking"` to the reqwest feature list and add the watch deps:
```toml
reqwest = { version = "0.12", default-features = false, features = ["multipart", "json", "rustls-tls", "blocking"] }
rusqlite = { version = "0.31", features = ["bundled"] }
sha2 = "0.10"
anyhow = "1"
url = "2"
scraper = "0.19"
base64 = "0.22"
```
(Leave every existing dependency line untouched; only the reqwest line changes, the rest are additions.)

- [ ] **Step 2: Copy the six logic-only modules verbatim**

Copy these files **byte-for-byte** from the standalone repo into `src-tauri/src/watch/`, with exactly one edit total (noted for `score.rs`):

- `C:\Users\shane\trapline-watch\src\fetch.rs`      → `src-tauri/src/watch/fetch.rs` (verbatim)
- `C:\Users\shane\trapline-watch\src\normalize.rs`  → `src-tauri/src/watch/normalize.rs` (verbatim — keeps its `#[cfg(test)]` tests)
- `C:\Users\shane\trapline-watch\src\parse.rs`      → `src-tauri/src/watch/parse.rs` (verbatim — keeps its `#[cfg(test)]` tests)
- `C:\Users\shane\trapline-watch\src\store.rs`      → `src-tauri/src/watch/store.rs` (verbatim)
- `C:\Users\shane\trapline-watch\src\sourcemap.rs`  → `src-tauri/src/watch/sourcemap.rs` (verbatim — keeps its `#[cfg(test)]` tests)
- `C:\Users\shane\trapline-watch\src\score.rs`      → `src-tauri/src/watch/score.rs`, changing ONLY its first line `use crate::parse::{Artifact, Kind};` to `use super::parse::{Artifact, Kind};`

- [ ] **Step 3: Write `watch/config.rs` (the engine's internal config view)**

This is a trimmed in-memory struct (built by the scheduler in Task 5 from the app `Config`); it is NOT loaded from disk, so it has no serde/TOML code. Create `src-tauri/src/watch/config.rs`:
```rust
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
```

- [ ] **Step 4: Write `watch/mod.rs`**

Create `src-tauri/src/watch/mod.rs` (engine/sink/scheduler submodules are added in later tasks — declare only what exists now, plus placeholders added as tasks land):
```rust
//! Built-in Watch: continuous attack-surface change detection, ported in-process
//! from the standalone `trapline-watch` crate. Engine logic is unchanged; the
//! config/alert/findings edges are rewired to the app.

pub mod config;
pub mod fetch;
pub mod normalize;
pub mod parse;
pub mod score;
pub mod sourcemap;
pub mod store;
```

- [ ] **Step 5: Register the module**

In `src-tauri/src/lib.rs`, add `mod watch;` to the module list at the top (alphabetical-ish, e.g. after `mod session;` or wherever fits the existing order).

- [ ] **Step 6: Build + run the carried tests**

Run: `cd src-tauri && cargo test watch::`
Expected: PASS — `watch::parse::tests` (2), `watch::normalize::tests` (3), `watch::sourcemap::tests` (3) all green; crate compiles. (Unused-code warnings for not-yet-consumed modules are acceptable; there must be no errors.)

- [ ] **Step 7: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/lib.rs src-tauri/src/watch/
git commit -m "feat(watch): port pure engine modules (fetch/normalize/parse/score/store/sourcemap) in-process"
```

---

### Task 3: Edge sinks — Discord blocking alert + Watch→finding mapping

**Files:**
- Modify: `src-tauri/src/discord.rs` (add a blocking rich-embed sender)
- Create: `src-tauri/src/watch/sink.rs`
- Modify: `src-tauri/src/watch/mod.rs` (add `pub mod sink;`)

**Interfaces:**
- Consumes: `watch::parse::{Artifact, Kind}` (Task 2), `crate::findings::save` (existing), `crate::discord` (existing).
- Produces:
  - `discord::send_watch_alert(webhook: &str, username: &str, content: &str, embeds: serde_json::Value) -> Result<(), String>` — blocking; no-op when `webhook` is empty; posts `{ "username": username, "content": content, "embeds": embeds }`.
  - `watch::sink::build_watch_finding_json(id: &str, target: &str, title: &str, severity_lc: &str, top: i64, kept: &[(i64, Artifact)], source_url: &str) -> String` — pure; returns a camelCase `HunterFinding` JSON string.
  - `watch::sink::record_watch_finding(id, target, title, severity_lc, top, kept, source_url) -> Result<(), String>` — calls `build_watch_finding_json` then `crate::findings::save`.

- [ ] **Step 1: Write the failing tests**

Append a test module to `src-tauri/src/discord.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_watch_alert_noop_on_empty_webhook() {
        // Empty webhook must short-circuit to Ok without any network call.
        let r = send_watch_alert("", "Trapline", "hi", serde_json::json!([]));
        assert!(r.is_ok());
    }
}
```

Create `src-tauri/src/watch/sink.rs` with its test module:
```rust
use crate::watch::parse::{Artifact, Kind};

/// Build a HunterFinding JSON (camelCase, mirrors src-tauri/src/findings.rs) from
/// a watch alert. Severity is passed already-lowercased so it matches the UI.
pub fn build_watch_finding_json(
    id: &str,
    target: &str,
    title: &str,
    severity_lc: &str,
    top: i64,
    kept: &[(i64, Artifact)],
    source_url: &str,
) -> String {
    // De-duplicate by value within each bucket while preserving score order.
    let mut secrets = Vec::new();
    let mut endpoints = Vec::new();
    let mut routes = Vec::new();
    let mut flags = Vec::new();
    for (s, a) in kept {
        let line = format!("[{s}] {}", a.value);
        match a.kind {
            Kind::Secret => secrets.push(a.value.clone()),
            Kind::Endpoint => endpoints.push(line),
            Kind::Route => routes.push(line),
            Kind::Flag => flags.push(line),
        }
    }
    let mut evidence = format!("Trapline Watch — new attack surface on {target}\nSource: {source_url}\n");
    if !secrets.is_empty() {
        evidence.push_str(&format!("\nNew secrets ({}):\n{}\n", secrets.len(), secrets.join("\n")));
    }
    if !endpoints.is_empty() {
        evidence.push_str(&format!("\nNew endpoints ({}):\n{}\n", endpoints.len(), endpoints.join("\n")));
    }
    if !routes.is_empty() {
        evidence.push_str(&format!("\nNew routes ({}):\n{}\n", routes.len(), routes.join("\n")));
    }
    if !flags.is_empty() {
        evidence.push_str(&format!("\nNew flags ({}):\n{}\n", flags.len(), flags.join("\n")));
    }

    let finding = serde_json::json!({
        "id": id,
        "programName": target,
        "platform": "Watch",
        "title": title,
        "severity": severity_lc,
        "status": "draft",
        "endpoint": source_url,
        "summary": format!("Watch detected {} new artifact(s) on {} (top score {}).", kept.len(), target, top),
        "description": "",
        "steps": "",
        "evidence": evidence,
        "impact": "",
        "remediation": "",
        "cvss": "",
        "cvssScore": "",
        "notes": "Auto-generated by built-in Trapline Watch.",
        "cmdline": "trapline-watch",
    });
    serde_json::to_string(&finding).unwrap()
}

/// Persist a watch alert as a finding in the in-process store.
pub fn record_watch_finding(
    id: &str,
    target: &str,
    title: &str,
    severity_lc: &str,
    top: i64,
    kept: &[(i64, Artifact)],
    source_url: &str,
) -> Result<(), String> {
    let json = build_watch_finding_json(id, target, title, severity_lc, top, kept, source_url);
    crate::findings::save(&json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watch::parse::{Artifact, Kind};

    #[test]
    fn maps_kept_artifacts_to_lowercase_severity_finding() {
        let kept = vec![
            (100i64, Artifact { kind: Kind::Secret, value: "sk_live_xxx".into() }),
            (85i64, Artifact { kind: Kind::Endpoint, value: "/api/v2/admin".into() }),
        ];
        let json = build_watch_finding_json(
            "watch-1", "acme", "New attack surface on acme", "critical", 100, &kept, "https://acme.com/app.js",
        );
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["severity"], "critical");
        assert_eq!(v["programName"], "acme");
        assert_eq!(v["platform"], "Watch");
        assert_eq!(v["endpoint"], "https://acme.com/app.js");
        assert!(v["evidence"].as_str().unwrap().contains("/api/v2/admin"));
        assert!(v["evidence"].as_str().unwrap().contains("sk_live_xxx"));
        assert_eq!(v["cmdline"], "trapline-watch");
    }
}
```

- [ ] **Step 2: Run them → FAIL**

Run: `cd src-tauri && cargo test`
Expected: FAIL — `send_watch_alert` not defined; `watch::sink` not declared.

- [ ] **Step 3: Implement `discord::send_watch_alert` + register the sink module**

In `src-tauri/src/discord.rs`, add the blocking sender (place after `send_embed`):
```rust
/// Post a watch alert (content + rich embed array) to a Discord webhook using a
/// BLOCKING client — called from the Watch scheduler thread, not the async
/// runtime. No-op when the webhook is empty.
pub fn send_watch_alert(
    webhook_url: &str,
    username: &str,
    content: &str,
    embeds: serde_json::Value,
) -> Result<(), String> {
    if webhook_url.trim().is_empty() {
        return Ok(());
    }
    let payload = serde_json::json!({
        "username": username,
        "content": content,
        "embeds": embeds,
    });
    let client = reqwest::blocking::Client::builder()
        .timeout(TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(webhook_url)
        .json(&payload)
        .send()
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        Err(format!("Discord returned {}: {}", status, body))
    }
}
```

In `src-tauri/src/watch/mod.rs`, add:
```rust
pub mod sink;
```

- [ ] **Step 4: Run tests → PASS**

Run: `cd src-tauri && cargo test`
Expected: PASS (the two new tests + all prior).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/discord.rs src-tauri/src/watch/sink.rs src-tauri/src/watch/mod.rs
git commit -m "feat(watch): edge sinks — blocking Discord alert + Watch→findings mapping"
```

---

### Task 4: Port `engine.rs` with rewired edges

**Files:**
- Create: `src-tauri/src/watch/engine.rs`
- Modify: `src-tauri/src/watch/mod.rs` (add `pub mod engine;`)

**Interfaces:**
- Consumes: `watch::config::{Config, Target}`, `watch::fetch`, `watch::parse`, `watch::store::Store`, `watch::{normalize, score, sourcemap}` (Task 2); `watch::sink::record_watch_finding`, `discord::send_watch_alert` (Task 3).
- Produces: `watch::engine::run_once(cfg: &Config, store: &Store, fetcher: &Fetcher) -> anyhow::Result<usize>` (returns total new-artifact count; consumed by Task 5).

- [ ] **Step 1: Copy `engine.rs` and rewire the `use` block**

Copy `C:\Users\shane\trapline-watch\src\engine.rs` → `src-tauri/src/watch/engine.rs`. Replace the top `use` block (lines 1–12 of the source) with:
```rust
use anyhow::Result;
use chrono::Utc;
use scraper::{Html, Selector};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use url::Url;

use super::config::{Config, Target};
use super::fetch::{Fetched, Fetcher};
use super::parse::{self, Artifact, Kind};
use super::store::Store;
use super::{normalize, score, sourcemap};
```
(The removed `use crate::{alert, finding, ...trapline}` line is intentionally gone — those edges are handled below.)

- [ ] **Step 2: Rewire the `run_target` tail (the alert + finding block)**

In the copied `engine.rs`, find the block that currently starts at `let embed = build_embed(...)` and runs through `store.insert_finding(...)?;` (the standalone's lines ~196–217). Replace that block with:
```rust
    let embed = build_embed(&target.name, &kept, sev, top, &source_url);
    let content = format!("\u{1FAA4} **Trapline Watch** — {title}");
    if let Err(e) = crate::discord::send_watch_alert(&cfg.discord_webhook, &cfg.discord_username, &content, embed) {
        eprintln!("[{}] discord alert failed: {e}", target.name);
    }

    // One id shared across the alert + the in-process finding.
    let sh = sha256_hex(&source_url);
    let id = format!("watch-{}-{}", Utc::now().format("%Y%m%d-%H%M%S"), &sh[..6]);

    // Findings go DIRECTLY into the in-process store (no cross-process sync).
    if let Err(e) = super::sink::record_watch_finding(
        &id, &target.name, &title, &sev.to_lowercase(), top, &kept, &source_url,
    ) {
        eprintln!("[{}] finding write failed: {e}", target.name);
    }
    // SQLite audit row (Watch's own history table) — unchanged.
    store.insert_finding(&target.name, &title, sev, top)?;

    println!("[{}] {count} new artifact(s) — top {top} ({sev})", target.name);
    Ok(count)
```
Everything ELSE in `engine.rs` — `sha256_hex`, `host_in_scope`, `discover_js`, `run_once`, the body of `run_target` above this tail, `analysis_units`, `source_files`, `fetch_map`, `truncate_list`, `build_embed` — is copied **verbatim**. Do not touch it.

- [ ] **Step 3: Register the module**

In `src-tauri/src/watch/mod.rs`, add:
```rust
pub mod engine;
```

- [ ] **Step 4: Build**

Run: `cd src-tauri && cargo test`
Expected: PASS — compiles clean; all existing tests still green (`engine.rs` has no unit tests of its own; its coverage is the module tests from Task 2). There must be no errors.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/watch/engine.rs src-tauri/src/watch/mod.rs
git commit -m "feat(watch): port engine with rewired alert/findings edges (in-process, no cross-process sync)"
```

---

### Task 5: Scheduler + `watch_*` commands + registration + auto-resume

**Files:**
- Create: `src-tauri/src/watch/scheduler.rs`
- Modify: `src-tauri/src/watch/mod.rs` (add `pub mod scheduler;`), `src-tauri/src/lib.rs` (register commands + `.setup()` auto-resume + exit-handler stop)

**Interfaces:**
- Consumes: `crate::config` (Task 1), `watch::config`, `watch::store::Store`, `watch::fetch::Fetcher`, `watch::engine::run_once` (Tasks 2/4), `crate::AppState`.
- Produces: commands `watch_start`, `watch_stop`, `watch_status`, `watch_run_once`; the `watch:status` and `watch:new-finding` events; pure mapper `watch::scheduler::to_engine_config(&crate::config::Config, database: String) -> watch::config::Config`; `watch::scheduler::watch_db_path() -> String`; `watch::scheduler::stop_flag()` (for the exit handler); `WatchStatus` (serde camelCase).

- [ ] **Step 1: Write the failing tests (pure pieces)**

Create `src-tauri/src/watch/scheduler.rs` starting with the testable pure helpers + their tests (the thread/command glue is added in Step 3, but these tests pin the mapping and status shape which don't need an `AppHandle`):
```rust
use crate::config::Config as AppConfig;
use super::config::{Config as EngineConfig, Target as EngineTarget, default_user_agent};

/// Map the app-level Config into the engine's in-memory Config for one cycle.
pub fn to_engine_config(app: &AppConfig, database: String) -> EngineConfig {
    EngineConfig {
        interval_secs: app.watch_interval_secs,
        alert_threshold: app.watch_alert_threshold,
        discord_webhook: app.webhook_url.clone(),
        discord_username: app.username.clone(),
        database,
        user_agent: default_user_agent(),
        max_requests_per_min: app.watch_max_rpm as u64,
        probe_source_maps: true,
        targets: app
            .watch_targets
            .iter()
            .map(|t| EngineTarget {
                name: t.name.clone(),
                pages: t.pages.clone(),
                js: t.js.clone(),
                in_scope: t.in_scope.clone(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config as AppConfig, WatchTarget};

    #[test]
    fn maps_app_config_to_engine_config() {
        let mut app = AppConfig::default();
        app.webhook_url = "https://discord/wh".into();
        app.username = "Trapline".into();
        app.watch_interval_secs = 900;
        app.watch_alert_threshold = 70;
        app.watch_max_rpm = 20;
        app.watch_targets = vec![WatchTarget {
            name: "acme".into(),
            pages: vec!["https://app.acme.com".into()],
            js: vec![],
            in_scope: vec!["acme.com".into()],
            auto_enrich: false,
        }];
        let ec = to_engine_config(&app, "C:/x/watch.db".into());
        assert_eq!(ec.interval_secs, 900);
        assert_eq!(ec.alert_threshold, 70);
        assert_eq!(ec.max_requests_per_min, 20);
        assert_eq!(ec.discord_webhook, "https://discord/wh");
        assert_eq!(ec.discord_username, "Trapline");
        assert_eq!(ec.database, "C:/x/watch.db");
        assert_eq!(ec.targets.len(), 1);
        assert_eq!(ec.targets[0].name, "acme");
        assert_eq!(ec.targets[0].in_scope, vec!["acme.com".to_string()]);
    }

    #[test]
    fn watch_status_serializes_camelcase() {
        let s = WatchStatus {
            running: true,
            targets: 2,
            interval_secs: 1800,
            last_run_ms: 123,
            last_assets: 18,
            last_new: 3,
        };
        let j = serde_json::to_string(&s).unwrap();
        assert!(j.contains("\"intervalSecs\""));
        assert!(j.contains("\"lastRunMs\""));
        assert!(j.contains("\"lastAssets\""));
        assert!(!j.contains("interval_secs"));
    }
}
```

- [ ] **Step 2: Run them → FAIL**

Run: `cd src-tauri && cargo test watch::scheduler`
Expected: FAIL (`WatchStatus` not defined; `scheduler` not declared in mod).

- [ ] **Step 3: Implement the scheduler + commands**

Add to the TOP of `src-tauri/src/watch/scheduler.rs` (above the `to_engine_config` fn), the state, status type, runtime, cycle, and commands:
```rust
use crate::AppState;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

use super::engine;
use super::fetch::Fetcher;
use super::store::Store;

/// The scheduler is a single background thread; this flag is its run gate.
static WATCH_RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Default, Clone)]
struct Runtime {
    last_run_ms: i64,
    last_assets: i64,
    last_new: i64,
}
static RUNTIME: Lazy<Mutex<Runtime>> = Lazy::new(|| Mutex::new(Runtime::default()));

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WatchStatus {
    pub running: bool,
    pub targets: usize,
    pub interval_secs: u64,
    pub last_run_ms: i64,
    pub last_assets: i64,
    pub last_new: i64,
}

/// `%APPDATA%\Trapline\watch.db` — colocated with config.json/findings.json.
pub fn watch_db_path() -> String {
    let mut p = crate::config::config_path();
    p.pop(); // drop config.json → the Trapline dir
    p.push("watch.db");
    p.to_string_lossy().to_string()
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Snapshot the current app config into an engine config for one cycle.
fn snapshot(app: &AppHandle) -> crate::config::Config {
    app.state::<AppState>().config.lock().unwrap().clone()
}

/// Build the current status from the flag, runtime, and live config.
pub fn status(app: &AppHandle) -> WatchStatus {
    let cfg = snapshot(app);
    let rt = RUNTIME.lock().unwrap().clone();
    WatchStatus {
        running: WATCH_RUNNING.load(Ordering::SeqCst),
        targets: cfg.watch_targets.len(),
        interval_secs: cfg.watch_interval_secs,
        last_run_ms: rt.last_run_ms,
        last_assets: rt.last_assets,
        last_new: rt.last_new,
    }
}

fn emit_status(app: &AppHandle) {
    let _ = app.emit("watch:status", status(app));
}

/// Run exactly one detection cycle across all targets.
fn run_cycle(app: &AppHandle) {
    let app_cfg = snapshot(app);
    let cfg = to_engine_config(&app_cfg, watch_db_path());
    if cfg.targets.is_empty() {
        emit_status(app);
        return;
    }
    let store = match Store::open(&cfg.database) {
        Ok(s) => s,
        Err(e) => { eprintln!("[watch] open db failed: {e}"); return; }
    };
    let fetcher = match Fetcher::new(&cfg.user_agent, cfg.max_requests_per_min) {
        Ok(f) => f,
        Err(e) => { eprintln!("[watch] fetcher init failed: {e}"); return; }
    };
    match engine::run_once(&cfg, &store, &fetcher) {
        Ok(n) => {
            let assets: i64 = cfg
                .targets
                .iter()
                .map(|t| store.asset_count(&t.name).unwrap_or(0))
                .sum();
            {
                let mut rt = RUNTIME.lock().unwrap();
                rt.last_run_ms = now_ms();
                rt.last_assets = assets;
                rt.last_new = n as i64;
            }
            emit_status(app);
            if n > 0 {
                let _ = app.emit("watch:new-finding", serde_json::json!({ "count": n, "ts": now_ms() }));
            }
        }
        Err(e) => eprintln!("[watch] cycle error: {e}"),
    }
}

/// Set watch_enabled in the persisted config (runtime on/off is durable).
fn set_enabled(app: &AppHandle, on: bool) {
    let st = app.state::<AppState>();
    let mut c = st.config.lock().unwrap();
    c.watch_enabled = on;
    crate::config::save(&c);
}

/// Start the scheduler thread if not already running (idempotent).
pub fn start(app: AppHandle) {
    if WATCH_RUNNING.swap(true, Ordering::SeqCst) {
        return; // already running
    }
    set_enabled(&app, true);
    std::thread::spawn(move || {
        while WATCH_RUNNING.load(Ordering::SeqCst) {
            run_cycle(&app);
            let interval = snapshot(&app).watch_interval_secs.max(1);
            let mut slept = 0u64;
            // Sleep in 1s steps so a stop is responsive.
            while slept < interval && WATCH_RUNNING.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_secs(1));
                slept += 1;
            }
        }
        emit_status(&app);
    });
}

/// Stop the scheduler (idempotent) and persist the off state.
pub fn stop(app: &AppHandle) {
    WATCH_RUNNING.store(false, Ordering::SeqCst);
    set_enabled(app, false);
    emit_status(app);
}

/// Flag-only stop for the app exit handler (no config write / no emit needed).
pub fn stop_flag() {
    WATCH_RUNNING.store(false, Ordering::SeqCst);
}

/// Run a single cycle immediately, off the scheduler cadence (manual "Run once").
pub fn run_once_now(app: AppHandle) {
    std::thread::spawn(move || run_cycle(&app));
}

// ── Tauri commands ───────────────────────────────────────────────────────────
#[tauri::command]
pub fn watch_start(app: AppHandle) {
    start(app);
}

#[tauri::command]
pub fn watch_stop(app: AppHandle) {
    stop(&app);
}

#[tauri::command]
pub fn watch_status(app: AppHandle) -> WatchStatus {
    status(&app)
}

#[tauri::command]
pub fn watch_run_once(app: AppHandle) {
    run_once_now(app);
}
```

In `src-tauri/src/watch/mod.rs`, add:
```rust
pub mod scheduler;
```

- [ ] **Step 4: Register commands + auto-resume + exit stop in `lib.rs`**

In `src-tauri/src/lib.rs`:

Add the four commands to the `tauri::generate_handler![...]` list (after the `deck::*` entries):
```rust
            watch::scheduler::watch_start,
            watch::scheduler::watch_stop,
            watch::scheduler::watch_status,
            watch::scheduler::watch_run_once,
```

Add a `.setup(...)` call to the builder chain (before `.build(...)`), so Watch auto-resumes if it was on:
```rust
        .setup(|app| {
            use tauri::Manager;
            let enabled = app.state::<AppState>().config.lock().unwrap().watch_enabled;
            if enabled {
                watch::scheduler::start(app.handle().clone());
            }
            Ok(())
        })
```

In the `.run(|_app, event| { ... })` closure, add the watch flag-stop alongside the existing deck stop:
```rust
            if let tauri::RunEvent::ExitRequested { .. } = event {
                watch::scheduler::stop_flag();
                let _ = deck::deck_stop();
            }
```

- [ ] **Step 5: Run tests + build → PASS**

Run: `cd src-tauri && cargo test`
Expected: PASS — the two scheduler unit tests + all prior; crate builds with the four new commands registered.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/watch/scheduler.rs src-tauri/src/watch/mod.rs src-tauri/src/lib.rs
git commit -m "feat(watch): background scheduler + watch_* commands + auto-resume on launch"
```

---

### Task 6: Frontend bridge + events + `watch` store

**Files:**
- Modify: `src/lib/types.ts`, `src/lib/bridge.ts`, `src/lib/events.ts`
- Create: `src/lib/stores/watch.ts`, `src/lib/stores/watch.test.ts`

**Interfaces:**
- Consumes: the `watch_*` commands + `watch:status` / `watch:new-finding` events (Task 5), `findings` store's `loadFindings` (existing).
- Produces: TS `WatchTarget`, `WatchStatus`; extended `Config`; bridge `watchStart/watchStop/watchStatus/watchRunOnce`; `onWatchStatus/onWatchNewFinding`; the `watch` writable + `refreshWatch/startWatch/stopWatch/runWatchOnce/initWatch`.

- [ ] **Step 1: Extend types**

In `src/lib/types.ts`, extend `Config` and add the two new interfaces:
```ts
export interface WatchTarget {
  name: string;
  pages: string[];
  js: string[];
  inScope: string[];
  autoEnrich: boolean;
}

// src-tauri/src/watch/scheduler.rs (WatchStatus) — #[serde(rename_all = "camelCase")]
export interface WatchStatus {
  running: boolean;
  targets: number;
  intervalSecs: number;
  lastRunMs: number;
  lastAssets: number;
  lastNew: number;
}
```
Add to the `Config` interface:
```ts
  watchTargets: WatchTarget[];
  watchIntervalSecs: number;
  watchAlertThreshold: number;
  watchMaxRpm: number;
  watchEnabled: boolean;
```

- [ ] **Step 2: Add bridge wrappers**

In `src/lib/bridge.ts`, import `WatchStatus`, then add (after the Deck block):
```ts
// ── Watch (src-tauri/src/watch/scheduler.rs) ────────────────────────────────
export const watchStart = () => invoke<void>('watch_start');
export const watchStop = () => invoke<void>('watch_stop');
export const watchStatus = () => invoke<WatchStatus>('watch_status');
export const watchRunOnce = () => invoke<void>('watch_run_once');
```

- [ ] **Step 3: Add event listeners**

In `src/lib/events.ts`, add (mirroring `onQEvent`):
```ts
import type { WatchStatus } from './types';

export function onWatchStatus(handler: (s: WatchStatus) => void): Promise<() => void> {
  return listen<WatchStatus>('watch:status', (ev) => handler(ev.payload));
}

export interface WatchNewFinding { count: number; ts: number }
export function onWatchNewFinding(handler: (e: WatchNewFinding) => void): Promise<() => void> {
  return listen<WatchNewFinding>('watch:new-finding', (ev) => handler(ev.payload));
}
```

- [ ] **Step 4: Write the failing store test**

Create `src/lib/stores/watch.test.ts` (mock `$lib/bridge`, `$lib/events`, and `$lib/stores/findings` so the module loads without Tauri — the established store-test isolation rule):
```ts
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';

vi.mock('$lib/bridge', () => ({
  watchStart: vi.fn(() => Promise.resolve()),
  watchStop: vi.fn(() => Promise.resolve()),
  watchRunOnce: vi.fn(() => Promise.resolve()),
  watchStatus: vi.fn(() =>
    Promise.resolve({ running: false, targets: 0, intervalSecs: 1800, lastRunMs: 0, lastAssets: 0, lastNew: 0 }),
  ),
}));
vi.mock('$lib/events', () => ({
  onWatchStatus: vi.fn(() => Promise.resolve(() => {})),
  onWatchNewFinding: vi.fn(() => Promise.resolve(() => {})),
}));
vi.mock('$lib/stores/findings', () => ({ loadFindings: vi.fn(() => Promise.resolve()) }));

import { watch, refreshWatch, startWatch } from './watch';
import * as bridge from '$lib/bridge';

describe('watch store', () => {
  beforeEach(() => vi.clearAllMocks());

  it('refreshWatch pulls status from the bridge into the store', async () => {
    await refreshWatch();
    expect(bridge.watchStatus).toHaveBeenCalled();
    expect(get(watch).intervalSecs).toBe(1800);
  });

  it('startWatch calls the bridge then refreshes', async () => {
    await startWatch();
    expect(bridge.watchStart).toHaveBeenCalled();
    expect(bridge.watchStatus).toHaveBeenCalled();
  });
});
```

- [ ] **Step 5: Run it → FAIL**

Run: `npm test src/lib/stores/watch.test.ts`
Expected: FAIL (no `./watch` module).

- [ ] **Step 6: Implement the store**

Create `src/lib/stores/watch.ts` (follow the config/loot store pattern):
```ts
// Watch store — live status of the built-in change-detection scheduler.
// Fed by watch:status / watch:new-finding events plus an initial watchStatus()
// pull. A new finding triggers a findings reload so every findings surface
// (RightDock, FindingsPanel, Activity feed) updates.
import { writable } from 'svelte/store';
import type { WatchStatus } from '$lib/types';
import { watchStart, watchStop, watchStatus, watchRunOnce } from '$lib/bridge';
import { onWatchStatus, onWatchNewFinding } from '$lib/events';
import { loadFindings } from '$lib/stores/findings';

const EMPTY: WatchStatus = {
  running: false, targets: 0, intervalSecs: 1800, lastRunMs: 0, lastAssets: 0, lastNew: 0,
};

export const watch = writable<WatchStatus>(EMPTY);

export async function refreshWatch(): Promise<void> {
  try {
    watch.set(await watchStatus());
  } catch {
    /* backend not ready — keep last known */
  }
}

export async function startWatch(): Promise<void> {
  await watchStart();
  await refreshWatch();
}

export async function stopWatch(): Promise<void> {
  await watchStop();
  await refreshWatch();
}

export async function runWatchOnce(): Promise<void> {
  await watchRunOnce();
}

/** Wire the live event listeners once (call from the shell's onMount). */
export async function initWatch(): Promise<void> {
  await onWatchStatus((s) => watch.set(s));
  await onWatchNewFinding(() => {
    void refreshWatch();
    void loadFindings();
  });
  await refreshWatch();
}
```

- [ ] **Step 7: Run tests + check → PASS**

Run: `npm test src/lib/stores/watch.test.ts && npm run check`
Expected: PASS (store test green; `svelte-check` 0 errors).

- [ ] **Step 8: Commit**

```bash
git add src/lib/types.ts src/lib/bridge.ts src/lib/events.ts src/lib/stores/watch.ts src/lib/stores/watch.test.ts
git commit -m "feat(watch): frontend bridge + events + live watch store"
```

---

### Task 7: Wire RightDock + AppRail to the live watch + findings

**Files:**
- Modify: `src/lib/components/shell/RightDock.svelte`, `src/lib/components/shell/AppRail.svelte`, `src/lib/components/shell/Cockpit.svelte` (call `initWatch()` on mount)
- Create/Modify: `src/lib/components/shell/RightDock.test.ts`

**Interfaces:**
- Consumes: `watch` store (Task 6), `findings` store (existing).
- Produces: a live right dock (Watch box + findings list) and a live Watch indicator on the rail. No new exported interface.

- [ ] **Step 1: Read the current shell wiring**

Read `src/lib/components/shell/Cockpit.svelte` and `AppRail.svelte` to see (a) where `onMount` runs and what's already loaded there, (b) how the rail renders its Watch button and any existing badges (Task 3 of the polish sweep added findings/loot count badges — reuse that pattern), and (c) how a finding is opened into the editor (the FindingsPanel path) so the dock's finding rows can reuse it. Do NOT invent a second editor host.

- [ ] **Step 2: Write the failing component test**

Create `src/lib/components/shell/RightDock.test.ts` — mock `$lib/stores/watch` and `$lib/stores/findings` with seeded writables, render `RightDock`, assert live values appear:
```ts
import { describe, it, expect, vi } from 'vitest';
import { render } from '@testing-library/svelte';
import { writable } from 'svelte/store';

vi.mock('$lib/stores/watch', () => ({
  watch: writable({ running: true, targets: 2, intervalSecs: 1800, lastRunMs: Date.now(), lastAssets: 18, lastNew: 3 }),
}));
vi.mock('$lib/stores/findings', () => ({
  findings: writable([
    { id: 'a', severity: 'critical', title: 'Creds in breach dump', programName: 'acme', endpoint: 'x', createdAt: '2026-08-16T00:00:00Z' },
    { id: 'b', severity: 'high', title: 'Admin panel exposed', programName: 'acme', endpoint: 'y', createdAt: '2026-08-16T00:00:00Z' },
  ]),
}));

import RightDock from './RightDock.svelte';

describe('RightDock', () => {
  it('renders live watch counts and findings from the stores', () => {
    const { getByText, container } = render(RightDock);
    expect(getByText('18')).toBeTruthy();       // lastAssets
    expect(getByText('3')).toBeTruthy();        // lastNew
    expect(getByText('Creds in breach dump')).toBeTruthy();
    expect(getByText('Admin panel exposed')).toBeTruthy();
    // findings count in the header reflects the store length (2), not the mock "4"
    expect(container.textContent).toContain('2');
  });
});
```

- [ ] **Step 3: Run it → FAIL**

Run: `npm test src/lib/components/shell/RightDock.test.ts`
Expected: FAIL (RightDock is currently static markup with hardcoded values `18`/`3`/mock findings — the assertions on store-driven values may pass by coincidence on the numbers but the findings titles and count won't match; make the test meaningful by first confirming it fails on `'2'`/store titles).

- [ ] **Step 4: Implement — make the dock live**

In `RightDock.svelte`, add a `<script>` importing the stores and a `timeAgo` helper, and replace the hardcoded values with store-driven ones. Keep ALL existing class names + CSS (visual continuity):
```svelte
<script lang="ts">
  import { watch } from '$lib/stores/watch';
  import { findings } from '$lib/stores/findings';

  const SEV_ORDER: Record<string, number> = { critical: 0, high: 1, medium: 2, low: 3, info: 4 };
  const SEV_ABBR: Record<string, string> = { critical: 'C', high: 'H', medium: 'M', low: 'L', info: 'I' };

  function timeAgo(ms: number): string {
    if (!ms) return '—';
    const s = Math.max(0, Math.floor((Date.now() - ms) / 1000));
    if (s < 60) return `${s}s`;
    if (s < 3600) return `${Math.floor(s / 60)}m`;
    return `${Math.floor(s / 3600)}h`;
  }

  // Worst-first, capped for the dock.
  $: sortedFindings = [...$findings].sort(
    (a, b) => (SEV_ORDER[a.severity] ?? 5) - (SEV_ORDER[b.severity] ?? 5) || b.createdAt.localeCompare(a.createdAt),
  );
</script>
```
Then in the markup:
- The `.wlive` "live" indicator: show it only when `$watch.running`; otherwise render a muted "off" state. e.g. wrap the live dot in `{#if $watch.running}…live{:else}…idle{/if}`.
- The `.wl` "new route" callout: drive from `$watch.lastNew` — if `$watch.lastNew > 0` show "{$watch.lastNew} new artifacts since last cycle", else a muted "no changes" line. (Keep the box; just swap the text. Do not fabricate a specific route — Watch's per-artifact detail lives in the finding, surfaced in the findings list below.)
- The `.wm` metrics row: `Last` → `{timeAgo($watch.lastRunMs)}`, `Assets` → `{$watch.lastAssets}`, `New` → `{$watch.lastNew}`, `Every` → `{Math.round($watch.intervalSecs / 60)}m`.
- The `.findbox` header count `.c` → `{$findings.length}`.
- The `.fl` findings list: replace the three hardcoded `.fi` rows with `{#each sortedFindings.slice(0, 6) as f (f.id)}` rendering `<span class="sev {f.severity}">{SEV_ABBR[f.severity] ?? '•'}</span>` + `<div class="t">{f.title}<span>{f.programName || f.endpoint}</span></div>` + `<span class="go">›</span>`. Use TEXT bindings (never `{@html}`). If the FindingsPanel open-editor mechanism read in Step 1 is a simple store/prop call, add an `on:click` that opens `f` in the editor; if it requires threading a host callback that isn't readily available, leave the row non-interactive for this task and note it as a deferred follow-up (the FindingsPanel view remains the full editor entry).

Add the `medium`/`low`/`info` severity colors to the `.sev.*` CSS if not already present (mirror `.sev.high`/`.sev.crit`, using `--med`/`--low`/`--info` tokens if they exist; otherwise reuse the closest existing token — do not invent hex).

In `AppRail.svelte`, make the Watch button reflect `$watch.running` (e.g. an active/live class or the small pulse dot when running). Import the `watch` store; keep the existing button markup + the badge pattern from the polish sweep.

- [ ] **Step 5: Call `initWatch()` on mount**

In `Cockpit.svelte`'s existing `onMount`, add `void initWatch();` (import from `$lib/stores/watch`). Ensure findings are loaded too (if `loadFindings()` isn't already called on mount, add `void loadFindings();`). If `Cockpit.test.ts` breaks because it now imports the watch store (which imports bridge/events), add the same `$lib/stores/watch` mock used in Task 6 to `Cockpit.test.ts`.

- [ ] **Step 6: Run tests + check → PASS**

Run: `npm test && npm run check`
Expected: PASS — RightDock test green, `Cockpit.test.ts` still green (mock added if needed), `svelte-check` 0 errors.

- [ ] **Step 7: Commit**

```bash
git add src/lib/components/shell/RightDock.svelte src/lib/components/shell/AppRail.svelte src/lib/components/shell/Cockpit.svelte src/lib/components/shell/RightDock.test.ts
git commit -m "feat(watch): live right-dock watch box + findings list + rail indicator"
```

---

### Task 8: Settings — Watch section (targets + params + controls)

**Files:**
- Modify: `src/lib/components/Settings.svelte` (+ `src/lib/components/Settings.test.ts` if it exists; else create it)

**Interfaces:**
- Consumes: the `config` store + its save path (existing), `watch` store + `startWatch/stopWatch/runWatchOnce` (Task 6).
- Produces: a Watch settings section. No new exported interface.

- [ ] **Step 1: Read the current Settings save flow**

Read `src/lib/components/Settings.svelte` to see how it binds to the `config` store and how it saves (`saveConfig`/`setConfig` path). The new watch fields (`watchTargets`, `watchIntervalSecs`, `watchAlertThreshold`, `watchMaxRpm`) must round-trip through that SAME save (they're part of the `Config` type now). `watchEnabled` is NOT saved from the form — it's owned by `startWatch/stopWatch` and preserved server-side (Task 1).

- [ ] **Step 2: Write the failing component test**

Add to (or create) `src/lib/components/Settings.test.ts` — mock `$lib/bridge`, `$lib/events`, `$lib/stores/watch`, and the config store as needed (match the isolation pattern already used by other component tests in the repo). Assert:
```ts
// pseudocode shape — align imports/mocks with the repo's existing Settings/component tests
it('renders the Watch section with interval + a target editor', () => {
  // render Settings with a config store seeded with one watchTarget
  // expect the interval input to show 1800 (or the seeded value)
  // expect the seeded target name to appear in an editable field
});
it('the enable toggle calls startWatch when turned on', async () => {
  // click the enable toggle → expect startWatch (from $lib/stores/watch mock) called
});
```
(Write real assertions matching the repo's testing-library patterns; the two behaviors above are the contract.)

- [ ] **Step 3: Run it → FAIL**

Run: `npm test src/lib/components/Settings.test.ts`
Expected: FAIL (no Watch section yet).

- [ ] **Step 4: Implement the Watch section**

In `Settings.svelte`, add a "Watch" section (match the existing section styling — reuse the same field/label/section classes the Deck and webhook sections use):
- **Enable toggle** — reflects `$watch.running`; on change calls `startWatch()` / `stopWatch()` from `$lib/stores/watch`. (Do not bind it to `config.watchEnabled` in the form — the toggle drives the scheduler, which persists enabled itself.)
- **Interval (minutes)** — number input bound to `config.watchIntervalSecs` (show/edit in minutes: value = secs/60, write back `*60`), min 1.
- **Alert threshold** — number input bound to `config.watchAlertThreshold` (0–100).
- **Max requests/min** — number input bound to `config.watchMaxRpm` (min 0; 0 = unthrottled).
- **Run once** button — calls `runWatchOnce()`.
- **Targets editor** — for each target in `config.watchTargets`: a `name` text input, three textareas for `pages` / `js` / `inScope` (one URL/host per line — split on newlines on save, join with `\n` for display), and an `autoEnrich` checkbox; a "Remove" button per target and an "Add target" button that pushes a blank `{ name:'', pages:[], js:[], inScope:[], autoEnrich:false }`. Convert textarea⇄array with a helper (`toLines(arr)=>arr.join('\n')`, `fromLines(s)=>s.split('\n').map(t=>t.trim()).filter(Boolean)`).
- The section's edits persist through the existing Settings **Save** (which sends the whole `Config`). Confirm the save serializes the new fields (they're on the bound config object) — no separate save path.

Keep all content in Svelte text/`bind:value` bindings — no `{@html}`.

- [ ] **Step 5: Run tests + check → PASS**

Run: `npm test && npm run check`
Expected: PASS — Settings test green, whole suite green, `svelte-check` 0 errors.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/Settings.svelte src/lib/components/Settings.test.ts
git commit -m "feat(watch): Settings — targets editor, interval/threshold/rpm, enable + run-once controls"
```

---

## Self-Review

**1. Spec coverage (§4.3 checklist):**
- Port `{engine,fetch,normalize,parse,score,store,sourcemap}` as-is → Tasks 2 + 4. ✅
- Config in (targets + interval/threshold/rpm into app Config, reuse Discord webhook, watch.db in `%APPDATA%\Trapline`) → Tasks 1 + 5 (`to_engine_config`, `watch_db_path`). ✅
- Alerts out via `discord.rs` (not `alert.rs`) → Task 3 (`send_watch_alert`), Task 4 (engine calls it). ✅ (`alert.rs` not ported.)
- Findings out directly to in-process `findings.rs` (removes last-writer race) → Task 3 (`record_watch_finding`), Task 4 (engine tail). ✅ (`finding.rs`/`trapline.rs` not ported; cross-process sync gone.)
- Scheduler: background thread, `AtomicBool` + `Mutex`, blocking reqwest on its own thread, start/stop commands, auto-resume on launch → Task 5. ✅
- Drop `license.rs` + `main.rs` → not ported (only the seven engine modules + engine come over). ✅
- Keep `in_scope` gate + global rate limit → preserved verbatim in `engine.rs` (`host_in_scope`) + `fetch.rs` (`min_gap` throttle). ✅
- §5 config additions (`watch_targets`/`watch_interval_secs`/`watch_alert_threshold`/`watch_max_rpm`/`watch_enabled`, all `#[serde(default)]` camelCase) → Task 1. ✅ (`shodan_api_key`/`leakcheck_api_key` are Phase 3, intentionally deferred — noted in Global Constraints.)
- §4.2 events (`watch:status`, `watch:new-finding`) → Task 5 emits, Task 6 listens. ✅ (`enrich:progress` is Phase 3.)
- §4.2 UI (Watch rail + Activity feed + Surface Map new-node) → Task 7 (rail + dock live); Activity feed + Surface Map pick up Watch findings automatically because they derive from the `findings`/runs stores which now receive Watch findings (Phase 1d built them to merge from those stores). ✅
- Commands surface: chose config-managed targets (edited via Settings/`set_config`) over separate `watch_add/remove/list` commands — single source of truth, no duplicate state; the spec listed those as "illustrative." Commands shipped: `watch_start/stop/status/run_once`. ✅ (deviation, justified)

**2. Placeholder scan:** All Rust steps have concrete code. The two ported-verbatim tasks (2, 4) reference exact source paths with the exact single edits enumerated — not placeholders. Frontend Tasks 7–8 have concrete code for the non-obvious parts; Steps that say "read the current file first" (7.1, 8.1) are discovery steps, not deferred implementation — the implementation code follows them. Task 8's test is described by contract (the repo's Settings test harness must be matched) rather than reproduced verbatim, because its mock shape depends on the existing Settings test setup discovered in 8.1 — acceptable and flagged.

**3. Type consistency:**
- `WatchTarget` (Rust, camelCase `inScope`/`autoEnrich`) ↔ TS `WatchTarget` (`inScope`/`autoEnrich`). ✅
- `WatchStatus` fields `running/targets/intervalSecs/lastRunMs/lastAssets/lastNew` identical Rust↔TS. ✅
- `to_engine_config` maps `watch_max_rpm: u32` → engine `max_requests_per_min: u64` (`as u64`) — pinned in the Task 5 test. ✅
- `record_watch_finding` emits lowercase `severity` — pinned in the Task 3 test; engine passes `&sev.to_lowercase()`. ✅
- Engine `run_once` signature `(&Config, &Store, &Fetcher) -> Result<usize>` unchanged, consumed by scheduler `run_cycle`. ✅
- `send_watch_alert(webhook, username, content, embeds)` — engine passes `(&cfg.discord_webhook, &cfg.discord_username, &content, embed)`. ✅

## Execution Handoff

Subagent-driven (per REQUIRED SUB-SKILL). Backend chain (Tasks 1→5) then frontend (6→7→8), strictly sequential — each task's `cargo test` / `npm test` gate must pass before the next.
