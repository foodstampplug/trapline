# Phase 3 — Shodan + LeakCheck Integrations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Shodan + LeakCheck as first-class recon integrations — on-demand lookups (⌘K commands + Surface-Map node Enrich) **and** per-target auto-enrich driven by the Watch cycle — with masked keys in Settings, Shodan enriching the surface map and LeakCheck surfacing credential exposure as Findings.

**Architecture:** A new backend `integrations/` module holds async `reqwest` clients for Shodan (`api.shodan.io`, `?key=`) and LeakCheck v2 (`X-API-Key`), exposed as Tauri commands. Keys live in the app `Config` (masked in Settings). On-demand: commands render result cards; a Surface-Map node **Enrich** action paints Shodan ports/CVEs onto nodes; LeakCheck writes Findings. Auto-enrich: the Watch **engine** harvests raw host+email candidates during a cycle (it has the fetched content), and the **scheduler** enriches them (Shodan→enrichment store/event, LeakCheck→Findings), deduped+cached in a new `enrichment` table in `watch.db` so each value is queried once.

**Tech Stack:** Rust (Tauri v2, reqwest async rustls, rusqlite, url, regex, serde), SvelteKit 2 + Svelte 5 runes, TypeScript, Vitest.

**Spec:** `docs/superpowers/specs/2026-08-17-phase3-integrations-design.md` (resolves parent spec `2026-08-15-free-cockpit-rebuild-design.md` §4.4; §5/§6 still bind)

## Global Constraints

- **Keys are secret.** `shodanApiKey`/`leakcheckApiKey` live in local config only, masked in the UI, sent **only** backend→API. NEVER log them (no `eprintln!`/`println!`/`dbg!` of a key or the whole `Config`), never put one in an error string, a toast, a test fixture, or a commit. Key absent/blank → a benign `Err("… key not set — add it in Settings")`, never a panic.
- **LeakCheck PII is sensitive.** Store/render only a **redacted** summary (sources, counts, which fields were present). NEVER persist or render a plaintext password value even when the API returns it.
- **Passive lookups sit outside the `in_scope` gate** (they don't touch the target). Auto-enrich only fires for targets whose `autoEnrich` is explicitly on.
- **Ported Watch engine logic (diff/extract/score/sourcemap) stays unchanged.** The only allowed Watch-side edits are the two documented in the spec: the `enrichment` table + 2 methods in `store.rs`, and an **opt-in** harvest collector in `engine.rs`/`scheduler.rs` (when no collector is passed, behavior is byte-identical to Phase 2).
- **Defensive parsing.** Third-party JSON shapes drift: use `#[serde(default)]`/`Option`; an unrecognized shape yields an empty-but-Ok result, not a hard error.
- **Findings severity lowercase** (`critical/high/medium/low/info`); LeakCheck findings go through the `SAVE_LOCK`-guarded `findings::save`.
- **Strict CSP unchanged, no new frontend deps, no CDN.** All API calls are backend `reqwest` (not webview `fetch`), so `connect-src 'self'` is unaffected.
- **Commit after every task.** Conventional Commits; repo-local `noreply` email. Backend: `cd src-tauri && cargo test`. Frontend: `npm test` + `npm run check`.

---

### Task 1: Config — Shodan + LeakCheck key fields

**Files:**
- Modify: `src-tauri/src/config.rs`

**Interfaces:**
- Produces: `Config.shodan_api_key: String`, `Config.leakcheck_api_key: String` (camelCase JSON `shodanApiKey`/`leakcheckApiKey`, `#[serde(default)]`). These are Settings-form fields (round-trip through `set_config`/`saveConfig` like `watch_targets`) — no `preserve_*` change needed.

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `config.rs`:
```rust
    #[test]
    fn old_config_json_loads_without_integration_keys() {
        let old = r#"{"webhookUrl":"","username":"Trapline"}"#;
        let cfg: Config = serde_json::from_str(old).expect("old config must still parse");
        assert_eq!(cfg.shodan_api_key, "");
        assert_eq!(cfg.leakcheck_api_key, "");
    }

    #[test]
    fn integration_keys_round_trip_camelcase() {
        let mut c = Config::default();
        c.shodan_api_key = "SKEY".into();
        c.leakcheck_api_key = "LKEY".into();
        let j = serde_json::to_string(&c).unwrap();
        assert!(j.contains("\"shodanApiKey\""), "expected camelCase, got {j}");
        assert!(j.contains("\"leakcheckApiKey\""));
        let back: Config = serde_json::from_str(&j).unwrap();
        assert_eq!(back.shodan_api_key, "SKEY");
        assert_eq!(back.leakcheck_api_key, "LKEY");
    }
```

- [ ] **Step 2: Run → FAIL** — `cd src-tauri && cargo test config::tests` (fields don't exist).

- [ ] **Step 3: Implement**

Add to the `Config` struct (after the watch fields):
```rust
    /// Shodan API key (Settings, masked). Sent only backend→api.shodan.io.
    #[serde(default)]
    pub shodan_api_key: String,
    /// LeakCheck API key (Settings, masked). Sent only backend→leakcheck.io.
    #[serde(default)]
    pub leakcheck_api_key: String,
```
Add to `impl Default for Config`:
```rust
            shodan_api_key: String::new(),
            leakcheck_api_key: String::new(),
```

- [ ] **Step 4: Run → PASS** — `cd src-tauri && cargo test config::tests` (all config tests green, incl. existing deck/watch tests).

- [ ] **Step 5: Commit** — `feat(integrations): add shodan/leakcheck API key config fields`

---

### Task 2: Shodan client

**Files:**
- Create: `src-tauri/src/integrations/mod.rs`, `src-tauri/src/integrations/shodan.rs`
- Modify: `src-tauri/src/lib.rs` (`mod integrations;`)

**Interfaces:**
- Consumes: `reqwest` (async, already a dep with rustls).
- Produces:
  - `integrations::client() -> reqwest::Client` (shared builder: rustls, 20s timeout).
  - `integrations::shodan::{host_url, domain_url, search_url}(key, arg) -> String` (pure URL builders, testable).
  - `integrations::shodan::{host, domain, search}(key: &str, arg: &str) -> Result<ShodanHost|ShodanDomain|ShodanSearch, String>` (async).
  - `ShodanHost { ip, org, hostnames: Vec<String>, ports: Vec<u16>, services: Vec<ShodanService>, cves: Vec<String> }`, `ShodanService { port: u16, product: String, version: String }`, `ShodanDomain { domain, subdomains: Vec<String>, records: Vec<ShodanRecord{ kind, value }> }`, `ShodanSearch { total: u64, matches: Vec<ShodanMatch{ ip, port, org, product, cves: Vec<String> }> }` — all `#[derive(Serialize)]` camelCase, consumed by Tasks 4/7.

- [ ] **Step 1: Write failing tests**

Create `src-tauri/src/integrations/shodan.rs` with a `#[cfg(test)] mod tests` (URL building + defensive parsing from a static fixture — NO network, NO real key/data):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_place_key_and_args() {
        assert_eq!(host_url("K", "1.2.3.4"), "https://api.shodan.io/shodan/host/1.2.3.4?key=K");
        assert_eq!(domain_url("K", "acme.com"), "https://api.shodan.io/dns/domain/acme.com?key=K");
        assert!(search_url("K", "apache port:443").starts_with("https://api.shodan.io/shodan/host/search?key=K&query="));
        assert!(search_url("K", "a b").contains("query=a%20b") || search_url("K", "a b").contains("query=a+b"));
    }

    #[test]
    fn parse_host_pulls_ports_services_cves() {
        let sample = r#"{
          "ip_str":"1.2.3.4","org":"Acme","hostnames":["a.acme.com"],
          "ports":[80,443],
          "vulns":{"CVE-2021-1234":{},"CVE-2020-5678":{}},
          "data":[{"port":443,"product":"nginx","version":"1.20"},{"port":80,"product":"","version":""}]
        }"#;
        let h = parse_host(sample);
        assert_eq!(h.ip, "1.2.3.4");
        assert_eq!(h.org, "Acme");
        assert_eq!(h.ports, vec![80, 443]);
        assert!(h.cves.contains(&"CVE-2021-1234".to_string()));
        assert!(h.services.iter().any(|s| s.port == 443 && s.product == "nginx"));
    }

    #[test]
    fn parse_host_tolerates_missing_fields() {
        // Unknown/empty shape → empty-but-Ok, never a panic.
        let h = parse_host("{}");
        assert_eq!(h.ip, "");
        assert!(h.ports.is_empty());
        assert!(h.cves.is_empty());
    }
}
```

- [ ] **Step 2: Run → FAIL** — `cd src-tauri && cargo test integrations::shodan` (module doesn't exist).

- [ ] **Step 3: Implement**

`integrations/mod.rs`:
```rust
pub mod shodan;

use std::time::Duration;

/// Shared HTTP client for third-party integration lookups (async, rustls).
pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}
```
`integrations/shodan.rs` — URL builders, the typed structs, a pure `parse_host`/`parse_domain`/`parse_search` (serde into a permissive `Raw*` then map), and the async `host`/`domain`/`search`:
```rust
use serde::{Deserialize, Serialize};

const BASE: &str = "https://api.shodan.io";

pub fn host_url(key: &str, ip: &str) -> String { format!("{BASE}/shodan/host/{ip}?key={key}") }
pub fn domain_url(key: &str, domain: &str) -> String { format!("{BASE}/dns/domain/{domain}?key={key}") }
pub fn search_url(key: &str, query: &str) -> String {
    format!("{BASE}/shodan/host/search?key={key}&query={}", urlencoding(query))
}
/// Minimal query-string escaper (avoid pulling a new dep): space + a few reserved chars.
fn urlencoding(s: &str) -> String {
    s.chars().map(|c| match c {
        'a'..='z'|'A'..='Z'|'0'..='9'|'-'|'_'|'.'|'~' => c.to_string(),
        ' ' => "%20".to_string(),
        _ => format!("%{:02X}", c as u32 & 0xFF),
    }).collect()
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ShodanService { pub port: u16, pub product: String, pub version: String }
#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ShodanHost {
    pub ip: String, pub org: String, pub hostnames: Vec<String>,
    pub ports: Vec<u16>, pub services: Vec<ShodanService>, pub cves: Vec<String>,
}
// ... ShodanRecord/ShodanDomain, ShodanMatch/ShodanSearch analogously ...

#[derive(Deserialize, Default)]
struct RawHost {
    #[serde(default)] ip_str: String,
    #[serde(default)] org: String,
    #[serde(default)] hostnames: Vec<String>,
    #[serde(default)] ports: Vec<u16>,
    #[serde(default)] vulns: std::collections::HashMap<String, serde_json::Value>,
    #[serde(default)] data: Vec<RawService>,
}
#[derive(Deserialize, Default)]
struct RawService { #[serde(default)] port: u16, #[serde(default)] product: String, #[serde(default)] version: String }

pub fn parse_host(body: &str) -> ShodanHost {
    let raw: RawHost = serde_json::from_str(body).unwrap_or_default();
    let mut cves: Vec<String> = raw.vulns.keys().cloned().collect();
    cves.sort();
    ShodanHost {
        ip: raw.ip_str, org: raw.org, hostnames: raw.hostnames, ports: raw.ports,
        services: raw.data.into_iter().map(|d| ShodanService { port: d.port, product: d.product, version: d.version }).collect(),
        cves,
    }
}
// parse_domain / parse_search similar (permissive Raw structs).

pub async fn host(key: &str, ip: &str) -> Result<ShodanHost, String> {
    if key.trim().is_empty() { return Err("Shodan API key not set — add it in Settings".into()); }
    let body = super::client().get(host_url(key, ip)).send().await
        .map_err(|e| e.to_string())?.text().await.map_err(|e| e.to_string())?;
    Ok(parse_host(&body))
}
// domain / search analogous.
```
(Write the full `ShodanDomain`/`ShodanSearch` structs + their `parse_*` + async fns — same defensive pattern. Add a small test for `parse_search` total+matches and `parse_domain` subdomains if quick.)

Register `mod integrations;` in `lib.rs` (after `mod findings;` alphabetically-ish).

- [ ] **Step 4: Run → PASS** — `cd src-tauri && cargo test integrations::shodan` (URL + parse tests green; crate compiles — unused-until-Task-4 warnings OK).

- [ ] **Step 5: Commit** — `feat(integrations): shodan client (host/domain/search, defensive parsing)`

---

### Task 3: LeakCheck client + finding mapper

**Files:**
- Create: `src-tauri/src/integrations/leakcheck.rs`
- Modify: `src-tauri/src/integrations/mod.rs` (`pub mod leakcheck;`)

**Interfaces:**
- Consumes: `integrations::client()`, `crate::findings::save` (existing), `crate::watch::sink` pattern (for the finding JSON shape — mirror it).
- Produces:
  - `leakcheck::query_url(value) -> String`, `leakcheck::query(key, value, kind: &str) -> Result<LeakResult, String>` (async; `kind` = "email"|"domain").
  - `LeakResult { found: u64, sources: Vec<LeakSource{ name, date }>, results: Vec<LeakRow{ email, username_present: bool, password_present: bool, source: String }> }` (Serialize camelCase; **no plaintext password field**).
  - `leckcheck::build_finding_json(target: &str, value: &str, r: &LeakResult) -> Option<String>` — pure; `None` when `found==0`; severity `critical` if any `password_present` else `high`; evidence a **redacted** summary; camelCase HunterFinding.
  - `leakcheck::record_findings(target: &str, value: &str, r: &LeakResult) -> Result<(), String>` — calls `build_finding_json` then `crate::findings::save` when `Some`.

- [ ] **Step 1: Write failing tests**

`#[cfg(test)] mod tests` in `leakcheck.rs` (URL/parse + mapping + redaction; NO key, NO real PII — use obviously-fake values):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_targets_v2_query() {
        assert_eq!(query_url("a@b.test"), "https://leakcheck.io/api/v2/query/a@b.test");
    }

    #[test]
    fn parse_maps_found_and_flags_without_plaintext() {
        let sample = r#"{"success":true,"found":2,"result":[
          {"email":"a@b.test","password":"hunter2","source":{"name":"BreachX","breach_date":"2020-01"}},
          {"email":"c@b.test","username":"cc","source":{"name":"BreachY"}}
        ]}"#;
        let r = parse(sample);
        assert_eq!(r.found, 2);
        assert!(r.results.iter().any(|x| x.password_present));   // flagged...
        // ...but the plaintext value never appears anywhere in the serialized result:
        let j = serde_json::to_string(&r).unwrap();
        assert!(!j.contains("hunter2"), "plaintext password must never be serialized");
        assert!(r.sources.iter().any(|s| s.name == "BreachX"));
    }

    #[test]
    fn finding_severity_critical_when_plaintext_else_high_else_none() {
        let none = LeakResult { found: 0, ..Default::default() };
        assert!(build_finding_json("acme", "acme.com", &none).is_none());

        let plain = LeakResult { found: 1, results: vec![LeakRow { email: "a@b.test".into(), username_present: false, password_present: true, source: "BreachX".into() }], ..Default::default() };
        let j = build_finding_json("acme", "a@b.test", &plain).unwrap();
        let v: serde_json::Value = serde_json::from_str(&j).unwrap();
        assert_eq!(v["severity"], "critical");
        assert_eq!(v["platform"], "LeakCheck");
        assert!(!j.contains("hunter2"));

        let found_only = LeakResult { found: 3, results: vec![], ..Default::default() };
        let j2 = build_finding_json("acme", "acme.com", &found_only).unwrap();
        let v2: serde_json::Value = serde_json::from_str(&j2).unwrap();
        assert_eq!(v2["severity"], "high");
    }
}
```
(Add `#[derive(Default)]` to `LeakResult`/`LeakRow`/`LeakSource` so the tests can build them.)

- [ ] **Step 2: Run → FAIL** — `cd src-tauri && cargo test integrations::leakcheck`.

- [ ] **Step 3: Implement**

`leakcheck.rs`: `query_url`, permissive `Raw` parse (`success`, `found`, `result[]` each with optional `email`/`username`/`password`/`source`), mapping to `LeakResult` **dropping any password value** (only a `password_present` bool), `build_finding_json` (mirror `watch::sink::build_watch_finding_json` field set: id `leak-{ts}-{hash6}` computed by the caller OR inside — here compute inside via `chrono` + a short hash of value; `programName`=target, `platform`="LeakCheck", `title`=`format!("Credential exposure: {value} ({} records)", r.found)`, `severity` per rule, `status`="draft", `endpoint`=value, `evidence`=redacted summary [sources + counts + "N rows with password present"], `cmdline`="leakcheck", cvss/notes empty), and `query`/`record_findings`. `query` returns the key-absent error when `key` blank. Add `pub mod leakcheck;` to `integrations/mod.rs`.

Do NOT `println!`/`eprintln!` the key or any password. The evidence text must never include a plaintext password.

- [ ] **Step 4: Run → PASS** — `cd src-tauri && cargo test integrations::leakcheck`.

- [ ] **Step 5: Commit** — `feat(integrations): leakcheck v2 client + redacted credential-exposure finding mapper`

---

### Task 4: Integration commands + registration

**Files:**
- Create: `src-tauri/src/integrations/commands.rs`
- Modify: `src-tauri/src/integrations/mod.rs` (`pub mod commands;`), `src-tauri/src/lib.rs` (register 5 commands)

**Interfaces:**
- Consumes: `integrations::{shodan, leakcheck}` (T2/T3), `crate::AppState` (key from config).
- Produces: commands `shodan_host(ip)`, `shodan_domain(domain)`, `shodan_search(query)`, `leakcheck_domain(domain)`, `leakcheck_email(email)` — each async, reads the relevant key from `AppState.config`, returns the typed result (or the key-absent error). `leakcheck_*` calls `record_findings` (writes to the findings store) before returning the `LeakResult` for the card.

- [ ] **Step 1: Write the failing test**

`#[cfg(test)] mod tests` in `commands.rs` — the command glue needs a live `AppState`/`AppHandle` so full command tests aren't hermetic; instead pin the one pure guard that lives here (key-absent → the exact error), which the commands rely on:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_key_message_is_actionable() {
        assert_eq!(key_or_err("", "Shodan"), Err("Shodan API key not set — add it in Settings".to_string()));
        assert_eq!(key_or_err("  ", "LeakCheck"), Err("LeakCheck API key not set — add it in Settings".to_string()));
        assert_eq!(key_or_err("K", "Shodan"), Ok("K".to_string()));
    }
}
```

- [ ] **Step 2: Run → FAIL** — `cd src-tauri && cargo test integrations::commands`.

- [ ] **Step 3: Implement**

`commands.rs`:
```rust
use crate::AppState;
use crate::integrations::{leakcheck, shodan};
use tauri::State;

/// Trim-and-validate a key, or an actionable error naming the provider.
pub fn key_or_err(key: &str, provider: &str) -> Result<String, String> {
    let k = key.trim();
    if k.is_empty() { Err(format!("{provider} API key not set — add it in Settings")) } else { Ok(k.to_string()) }
}

#[tauri::command]
pub async fn shodan_host(ip: String, state: State<'_, AppState>) -> Result<shodan::ShodanHost, String> {
    let key = key_or_err(&state.config.lock().unwrap().shodan_api_key, "Shodan")?;
    shodan::host(&key, &ip).await
}
// shodan_domain / shodan_search analogous (search takes `query: String`).

#[tauri::command]
pub async fn leakcheck_domain(domain: String, state: State<'_, AppState>) -> Result<leakcheck::LeakResult, String> {
    let key = key_or_err(&state.config.lock().unwrap().leakcheck_api_key, "LeakCheck")?;
    let r = leakcheck::query(&key, &domain, "domain").await?;
    let _ = leakcheck::record_findings(&domain, &domain, &r); // best-effort; card still returns
    Ok(r)
}
// leakcheck_email analogous (kind "email"; target = the email for on-demand context).
```
Note: read the key inside a short lock scope, drop the guard before `.await` (the guard is not `Send` across await). Add `pub mod commands;` to `integrations/mod.rs`; register the 5 commands in `lib.rs`'s `generate_handler!`.

- [ ] **Step 4: Run → PASS + build** — `cd src-tauri && cargo test` (all pass; the 5 commands compile + register; the T2/T3 dead-code warnings clear now that commands call them).

- [ ] **Step 5: Commit** — `feat(integrations): shodan_* / leakcheck_* tauri commands + registration`

---

### Task 5: Enrichment cache table (`watch.db`)

**Files:**
- Modify: `src-tauri/src/watch/store.rs`

**Interfaces:**
- Produces: an `enrichment(target, kind, value, result_json, first_seen)` table (added to `SCHEMA`), and `Store::enrichment_seen(target, kind, value) -> Result<bool>` (INSERT OR IGNORE → true when newly inserted) + `Store::save_enrichment(target, kind, value, result_json) -> Result<()>` (upsert the cached result). Consumed by Task 6.

- [ ] **Step 1: Write the failing test**

Add a `#[cfg(test)] mod tests` to `store.rs` (in-memory SQLite — hermetic):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    fn mem() -> Store { let conn = Connection::open_in_memory().unwrap(); conn.execute_batch(SCHEMA).unwrap(); Store { conn } }

    #[test]
    fn enrichment_seen_is_true_once_then_false() {
        let s = mem();
        assert!(s.enrichment_seen("acme", "host", "a.acme.com").unwrap());  // first time: new
        assert!(!s.enrichment_seen("acme", "host", "a.acme.com").unwrap()); // seen
        assert!(s.enrichment_seen("acme", "email", "a.acme.com").unwrap()); // different kind: new
    }
}
```
(If `Store`'s fields aren't visible to the test, either the test is in the same module — it is — so `Store { conn }` works; keep `conn` private but same-module tests can construct it.)

- [ ] **Step 2: Run → FAIL** — `cd src-tauri && cargo test watch::store`.

- [ ] **Step 3: Implement**

Append to `SCHEMA`:
```sql
CREATE TABLE IF NOT EXISTS enrichment (
  target      TEXT NOT NULL,
  kind        TEXT NOT NULL,
  value       TEXT NOT NULL,
  result_json TEXT NOT NULL DEFAULT '',
  first_seen  TEXT NOT NULL,
  PRIMARY KEY (target, kind, value)
);
```
Add the two methods (mirror `record_artifact`/`upsert_asset`):
```rust
/// Records that (target, kind, value) has been enrichment-queried. True if new.
pub fn enrichment_seen(&self, target: &str, kind: &str, value: &str) -> Result<bool> {
    let n = self.conn.execute(
        "INSERT OR IGNORE INTO enrichment (target, kind, value, first_seen) VALUES (?1, ?2, ?3, ?4)",
        params![target, kind, value, Utc::now().to_rfc3339()],
    )?;
    Ok(n == 1)
}
pub fn save_enrichment(&self, target: &str, kind: &str, value: &str, result_json: &str) -> Result<()> {
    self.conn.execute(
        "UPDATE enrichment SET result_json = ?4 WHERE target = ?1 AND kind = ?2 AND value = ?3",
        params![target, kind, value, result_json],
    )?;
    Ok(())
}
```
Every existing method in `store.rs` stays byte-identical.

- [ ] **Step 4: Run → PASS** — `cd src-tauri && cargo test watch::store`.

- [ ] **Step 5: Commit** — `feat(integrations): enrichment cache table + dedup methods in watch store`

---

### Task 6: Auto-enrich — engine harvest + scheduler enrich

**Files:**
- Modify: `src-tauri/src/watch/engine.rs` (opt-in harvest collector), `src-tauri/src/watch/scheduler.rs` (enrich pass + event)
- Modify: `src-tauri/src/watch/mod.rs` if a shared `TargetHarvest` type needs a home (or define it in `engine.rs` and re-export)

**Interfaces:**
- Consumes: `engine::run_once` (existing), `Store::{enrichment_seen, save_enrichment}` (T5), `integrations::{shodan, leakcheck}` (T2/T3), the app `Config.watch_targets[].auto_enrich`.
- Produces:
  - `engine::TargetHarvest { target: String, hosts: Vec<String>, emails: Vec<String> }` and `engine::run_once_harvest(cfg, store, fetcher, out: &mut Vec<TargetHarvest>) -> Result<usize>` (same as `run_once` but threads harvesting into `run_target`; `run_once` stays a thin wrapper passing no collector → byte-identical behavior).
  - `enrich:host` event `{ target, host, ports: Vec<u16>, cves: Vec<String>, org }`.

- [ ] **Step 1: Write failing tests (pure helpers)**

Add to `engine.rs` a small pure host extractor + email harvester and test them (these are the new logic; the network/thread glue is exercised manually):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extract_host_from_url_artifact() {
        assert_eq!(host_of("https://api.acme.com/v2/x"), Some("api.acme.com".to_string()));
        assert_eq!(host_of("/relative/path"), None);      // routes without a host → None
        assert_eq!(host_of("not a url"), None);
    }
    #[test]
    fn harvest_emails_from_content() {
        let c = "contact support@acme.com or admin@acme.io; noise a@b (not an email)";
        let mut got = harvest_emails(c);
        got.sort();
        assert_eq!(got, vec!["admin@acme.io".to_string(), "support@acme.com".to_string()]);
    }
}
```

- [ ] **Step 2: Run → FAIL** — `cd src-tauri && cargo test watch::engine`.

- [ ] **Step 3: Implement the harvest**

In `engine.rs`:
- `pub fn host_of(v: &str) -> Option<String>` = `url::Url::parse(v).ok().and_then(|u| u.host_str().map(|h| h.to_string()))`.
- `pub fn harvest_emails(content: &str) -> Vec<String>` = a `once_cell` `Regex` `[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}`, deduped.
- `pub struct TargetHarvest { pub target: String, pub hosts: Vec<String>, pub emails: Vec<String> }`.
- Add `harvest: Option<&mut TargetHarvest>` to `run_target`; when `Some`: inside the unit loop, extend emails from `harvest_emails(&content)`; after building `new_artifacts`, add `host_of(&a.value)` for each new `Endpoint`/`Route` artifact. `run_once` passes `None` (one call-site change — behavior identical). Add `run_once_harvest` that builds a `Vec<TargetHarvest>` (one entry per target with a non-empty harvest). Dedup hosts/emails within a harvest.

Keep the diff/extract/score/sourcemap logic untouched — the harvest only reads values the cycle already computed/downloaded.

- [ ] **Step 4: Implement the scheduler enrich pass**

In `scheduler.rs::run_cycle`, after the existing `engine::run_once` result handling:
- Determine if any current app `watch_targets` has `auto_enrich == true`. If none, keep calling `run_once` exactly as today.
- If some do: call `run_once_harvest` (instead of `run_once`) to also collect harvests. For each `TargetHarvest` whose app target has `auto_enrich` on:
  - build the engine key snapshot (Shodan/LeakCheck keys from the app config snapshot);
  - for each host: `if store.enrichment_seen(target, "host", host)? {` → `block_on(shodan::domain_or_host(...))`; on Ok, `store.save_enrichment(...)` + `app.emit("enrich:host", json!({target, host, ports, cves, org}))`; (use `shodan::host` if the value parses as an IP, else `shodan::domain`); `}`
  - for each email: `if store.enrichment_seen(target, "email", email)? {` → `block_on(leakcheck::query(key, email, "email"))`; on Ok, `leakcheck::record_findings(target, email, &r)` (writes findings) + `store.save_enrichment(...)`; `}`
  - throttle: `std::thread::sleep(Duration::from_millis(...))` between calls (reuse a small gap; keys with no value are skipped up-front so a blank key doesn't spin).
- Use `tauri::async_runtime::block_on` around the async client calls (scheduler is a std thread). Emit-then-continue; a failed lookup is logged (NO key/PII in the log) and skipped, never fatal.
- After enrich, still emit `watch:status` as today.

- [ ] **Step 5: Run → PASS + build** — `cd src-tauri && cargo test` (engine harvest tests + all prior green; crate builds). Manual note in the report: with no `auto_enrich` target, the `run_once` path is unchanged.

- [ ] **Step 6: Commit** — `feat(integrations): watch-cycle auto-enrich (engine harvest + scheduler shodan/leakcheck pass)`

---

### Task 7: Frontend bridge + events + types + enrichment store

**Files:**
- Modify: `src/lib/types.ts`, `src/lib/bridge.ts`, `src/lib/events.ts`
- Create: `src/lib/stores/enrichment.ts`, `src/lib/stores/enrichment.test.ts`

**Interfaces:**
- Consumes: the 5 commands + `enrich:host` event (T4/T6).
- Produces: TS `ShodanHost/ShodanService/ShodanDomain/ShodanRecord/ShodanSearch/ShodanMatch`, `LeakResult/LeakSource/LeakRow` (camelCase, mirroring the Rust); extend `Config` with `shodanApiKey`/`leakcheckApiKey`; bridge `shodanHost/shodanDomain/shodanSearch/leakcheckDomain/leakcheckEmail`; `onEnrichHost`; the `enrichment` store (`Map<host, EnrichEntry>`) + `refreshEnrich`/`applyEnrichHost`/`initEnrich`.

- [ ] **Step 1: Extend types**

Add the interfaces to `types.ts` (match the Rust camelCase field-for-field): `ShodanHost { ip, org, hostnames, ports:number[], services:ShodanService[], cves:string[] }`, etc.; `EnrichHost { target, host, ports:number[], cves:string[], org }` (the event payload); extend `Config` with `shodanApiKey: string; leakcheckApiKey: string`.

- [ ] **Step 2: Bridge + events**

`bridge.ts` (after the watch block):
```ts
export const shodanHost = (ip: string) => invoke<ShodanHost>('shodan_host', { ip });
export const shodanDomain = (domain: string) => invoke<ShodanDomain>('shodan_domain', { domain });
export const shodanSearch = (query: string) => invoke<ShodanSearch>('shodan_search', { query });
export const leakcheckDomain = (domain: string) => invoke<LeakResult>('leakcheck_domain', { domain });
export const leakcheckEmail = (email: string) => invoke<LeakResult>('leakcheck_email', { email });
```
`events.ts`:
```ts
import type { EnrichHost } from './types';
export function onEnrichHost(handler: (e: EnrichHost) => void): Promise<() => void> {
  return listen<EnrichHost>('enrich:host', (ev) => handler(ev.payload));
}
```

- [ ] **Step 3: Write the failing store test**

`enrichment.test.ts` (mock `$lib/bridge` + `$lib/events`; assert `applyEnrichHost` inserts a host entry retrievable by host, and merging keeps latest):
```ts
vi.mock('$lib/bridge', () => ({ shodanDomain: vi.fn(), shodanHost: vi.fn() }));
vi.mock('$lib/events', () => ({ onEnrichHost: vi.fn(() => Promise.resolve(() => {})) }));
import { get } from 'svelte/store';
import { enrichment, applyEnrichHost } from './enrichment';
// applyEnrichHost({target:'acme',host:'a.acme.com',ports:[443],cves:['CVE-1'],org:'Acme'})
// → get(enrichment).get('a.acme.com')?.ports === [443]
```

- [ ] **Step 4: Run → FAIL** — `npm test src/lib/stores/enrichment.test.ts`.

- [ ] **Step 5: Implement `enrichment.ts`**

A `writable<Map<string, EnrichEntry>>` where `EnrichEntry { ports:number[]; services?:…; cves:string[]; org:string; lastEnriched:number }`; `applyEnrichHost(e)` upserts by `e.host`; `initEnrich()` wires `onEnrichHost(applyEnrichHost)`; on-demand `shodan_*` results also feed it (a helper `applyShodanHost(host, ShodanHost)`).

- [ ] **Step 6: Run → PASS + check** — `npm test src/lib/stores/enrichment.test.ts && npm run check`.

- [ ] **Step 7: Commit** — `feat(integrations): frontend bridge + events + types + enrichment store`

---

### Task 8: On-demand command cards (⌘K integration entries + result card)

**Files:**
- Modify: the launcher/command surface (discover in Step 1) + a new result-card component
- Create: `src/lib/components/IntegrationCard.svelte` (+ a test)

**Interfaces:**
- Consumes: the bridge wrappers (T7), the `enrichment` store (T7).
- Produces: five integration entries in the ⌘K/playbook surface that prompt for their arg and render an `IntegrationCard` (Shodan host/domain/search or LeakCheck summary) instead of streaming.

- [ ] **Step 1: Discovery**

Read `src/lib/components/Launcher.svelte`, `CommandBar.svelte`, `Playbook.svelte`, and `src/lib/data/templates.ts` to see how a command is chosen, how `{{placeholder}}` args are filled, and how a run is dispatched (so the integration entries reuse the arg-fill flow but call the bridge + open a card rather than `runCommand`). Do NOT invent a parallel launcher — extend the existing one with a distinct "integration" entry kind.

- [ ] **Step 2: Failing component test**

`IntegrationCard.test.ts`: render `IntegrationCard` with a mocked Shodan host result → asserts org, a port, and a CVE chip render; with a LeakCheck result → asserts found-count + a source render and **no plaintext password** appears (there is none in the type, but assert the redacted summary shape).

- [ ] **Step 3: Run → FAIL** — `npm test src/lib/components/IntegrationCard.test.ts`.

- [ ] **Step 4: Implement**

`IntegrationCard.svelte`: a discriminated `{ kind: 'shodanHost'|'shodanDomain'|'shodanSearch'|'leak', data }` card rendering the relevant fields with text bindings (ports/CVE chips, service list, or breach sources + counts). Wire five integration entries into the launcher: each prompts for its single arg (ip/domain/query/email), calls the matching bridge fn, and opens the card (and for Shodan host/domain, feeds `applyShodanHost` so the Surface Map benefits too). On error (e.g. key absent), show the error via the existing toast. No `{@html}`.

- [ ] **Step 5: Run → PASS + check** — `npm test && npm run check`.

- [ ] **Step 6: Commit** — `feat(integrations): ⌘K integration commands + result cards`

---

### Task 9: Surface Map node Enrich + badges

**Files:**
- Modify: `src/lib/components/views/SurfaceMap.svelte`
- Modify: `Cockpit.svelte` if it must call `initEnrich()` on mount (discover)

**Interfaces:**
- Consumes: `enrichment` store (T7), `shodanDomain`/`shodanHost` bridge (T7), `applyShodanHost` (T7).
- Produces: an **Enrich** action in the node inspector + Shodan port/CVE badges on nodes whose host has enrichment data.

- [ ] **Step 1: Failing component test**

Extend/create `SurfaceMap.test.ts`: mock `$lib/stores/surface` (a scope with one node `a.acme.com`) + `$lib/stores/enrichment` (seeded with `a.acme.com → {ports:[443], cves:['CVE-1']}`) → assert the node (or its inspector) shows a `443` port badge and a CVE indicator; clicking **Enrich** (with `shodanDomain` mocked) calls the bridge.

- [ ] **Step 2: Run → FAIL** — `npm test src/lib/components/views/SurfaceMap.test.ts`.

- [ ] **Step 3: Implement**

In `SurfaceMap.svelte`: import `enrichment` + the bridge; in the inspector, add an **Enrich** button next to **→ Finding** that calls `shodanHost`/`shodanDomain` on `inspected.host` (host-vs-IP heuristic) → `applyShodanHost` → store; render, on any node whose host is in `$enrichment`, a small ports/CVE badge (CVE count severity-tinted using the existing `--crit/--high` tokens — no invented hex). Ensure `initEnrich()` runs on mount (add to `Cockpit.svelte`'s `onMount` if not already; if that transitively imports the enrichment store into `Cockpit.test.ts`, add the mock like the Phase-2 watch-store mock). Keep all existing SurfaceMap classes/CSS; additive only. No `{@html}`.

- [ ] **Step 4: Run → PASS + check** — `npm test && npm run check`.

- [ ] **Step 5: Commit** — `feat(integrations): surface-map node enrich action + shodan badges`

---

### Task 10: Settings — Integrations section (masked keys)

**Files:**
- Modify: `src/lib/components/Settings.svelte` (+ its test)

**Interfaces:**
- Consumes: the `config` store + existing save path (T1 added the fields).
- Produces: an Integrations settings section with two **masked** key inputs (Shodan, LeakCheck) that round-trip through the existing `saveConfig`.

- [ ] **Step 1: Failing test**

Add to `Settings.test.ts`: the Integrations section renders two key inputs of `type="password"` bound to `shodanApiKey`/`leakcheckApiKey`; a reveal toggle flips one to `type="text"`; saving includes the key fields in the `saveConfig` payload (mock the bridge; use a fake key value, never a real one).

- [ ] **Step 2: Run → FAIL** — `npm test src/lib/components/Settings.test.ts`.

- [ ] **Step 3: Implement**

Add an **Integrations** section (reuse the Watch/webhook section styling): two inputs `type={reveal ? 'text' : 'password'}` bound to `config.shodanApiKey`/`config.leakcheckApiKey`, each with a small reveal (👁) toggle; a one-line note "Keys are stored locally and sent only to Shodan / LeakCheck." The fields ride the existing Settings **Save** (they're on the bound `config`) — no separate save path. Never echo a key to a toast/log. No `{@html}`.

- [ ] **Step 4: Run → PASS + check** — `npm test && npm run check`.

- [ ] **Step 5: Commit** — `feat(integrations): Settings — masked Shodan/LeakCheck key inputs`

---

## Self-Review

**1. Spec coverage (§4.4):**
- Clients (Shodan `?key=`, LeakCheck v2 `X-API-Key`) → T2/T3. ✅
- Commands (shodan host/domain/search, leakcheck domain/email) + ⌘K → T4 + T8. ✅
- Enrich hybrid — on-demand (commands + node Enrich) → T8/T9; auto per-target → T6 (engine harvest + scheduler, gated by `auto_enrich`). ✅
- Result mapping — Shodan → cards + Surface-Map node enrichment (T8/T9); LeakCheck → Findings (T3 mapper, written by T4 on-demand + T6 auto). ✅
- Keys in Config, masked, backend→API only → T1 + T10 + Global Constraints. ✅
- Safety — passive, outside `in_scope`, PII redacted → T3 (redaction) + Global Constraints. ✅
- Config additions (§5) shodan/leakcheck keys → T1 (watch_* already shipped Phase 2). ✅
- Dedup/quota guard (D3) → T5 (`enrichment` table) + T6 (`enrichment_seen` gate). ✅
- Engine-harvest / scheduler-enrich split (spec §4.6) → T6. ✅

**2. Placeholder scan:** Backend tasks carry concrete code; the two client tasks say "domain/search structs analogous — same defensive pattern" which is a repeat-the-pattern instruction with the host example fully spelled out (acceptable). Frontend UI tasks (T8/T9/T10) use discovery steps (read the launcher/SurfaceMap/Settings first) + explicit behavior contracts + concrete test assertions — matching the Phase-2 approach for shell-integrated UI. No TBD/TODO.

**3. Type consistency:** `ShodanHost`/`LeakResult` Rust structs (T2/T3, Serialize camelCase) ↔ TS interfaces (T7) field-for-field; `enrich:host` payload `{target,host,ports,cves,org}` emitted by T6 ↔ `EnrichHost`/`onEnrichHost` (T7) ↔ consumed T9; `enrichment_seen`/`save_enrichment` (T5) ↔ called T6; the 5 command names (T4) ↔ bridge invoke strings (T7). LeakCheck severity `critical|high|none` consistent T3↔T3 tests↔findings UI (lowercase). ✅

## Execution Handoff

Subagent-driven (per REQUIRED SUB-SKILL). Backend chain T1→T6 (T5 before T6; T2/T3 before T4), then frontend T7→T8, T7→T9, T10 — sequential, each task's `cargo test`/`npm test` gate green before the next.
