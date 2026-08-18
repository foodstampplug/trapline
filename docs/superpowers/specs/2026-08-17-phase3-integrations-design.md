# Phase 3 — Shodan + LeakCheck Integrations (Design Addendum)

- **Date:** 2026-08-17
- **Status:** Approved design, pre-plan
- **Repo:** `C:\Users\shane\trapline-v2` (Tauri v2 app)
- **Parent spec:** `2026-08-15-free-cockpit-rebuild-design.md` §4.4 (this addendum resolves its open wiring decisions; everything in §4.4/§5/§6 still binds)
- **Related:** [[project-trapline-tool]], [[project-trapline-watch]]

## 1. Summary

Phase 3 of the free-cockpit initiative adds **Shodan** and **LeakCheck** as first-class
recon integrations. Chosen scope (confirmed): **full hybrid** — on-demand lookups **and**
per-target auto-enrich in this phase. Shodan enriches the attack surface (ports / services /
CVEs on Surface-Map nodes + recon cards); LeakCheck surfaces credential exposure as
**Findings**. Keys live masked in Settings, backend→API only. Both are passive third-party
lookups (they never touch the target), so they sit **outside** the `in_scope` fetch gate.

Builds on Phase 2 (Watch merged in-process, `main`@`3cfc2e5`). Phase 4 (go free) is separate.

## 2. Goals / Non-goals

**Goals**
- G1. Backend `integrations/` module: Rust `reqwest` clients for Shodan + LeakCheck, exposed as
  Tauri commands returning structured JSON.
- G2. On-demand surfaces: ⌘K/playbook command entries (result cards), a Surface-Map node
  **Enrich** action, and LeakCheck → Findings.
- G3. Per-target **auto-enrich** hooked into the Watch cycle: new hosts → Shodan, new emails →
  LeakCheck, deduped + cached so each value is queried once.
- G4. Masked key management in Settings; keys never logged, never in chat, never committed.

**Non-goals (this phase)**
- Phase 4 (landing rewrite / license removal / Gumroad retirement).
- Other data sources (Censys, HIBP, etc.).
- Auto-enrich on user-run one-off recon commands — auto-enrich is driven ONLY by the Watch
  cycle's per-target `autoEnrich` toggle (confirmed decision D1). One-off recon uses the manual
  Enrich button.
- Historical/bulk backfill of already-discovered hosts when `autoEnrich` is first toggled on
  (auto-enrich acts on values newly discovered on subsequent cycles; existing values can be
  enriched manually via the node Enrich button).

## 3. Confirmed decisions

- **D1 — auto-enrich scope:** tied to the **Watch cycle** for targets with `autoEnrich=on`, not
  to user-run recon commands (those use the manual node Enrich button).
- **D2 — LeakCheck API:** built to the **documented v2 API** (`GET https://leakcheck.io/api/v2/query/{query}?type=email|domain`, `X-API-Key` header),
  corrected against the owner's enterprise key at test time.
- **D3 — quota guard:** every host/email is enriched **once** — deduped + cached in a new
  `enrichment` table in `watch.db`; auto-enrich only queries values not already present.

## 4. Backend

### 4.1 New module `src-tauri/src/integrations/`
- `mod.rs` — declares `shodan`, `leakcheck`; shared `EnrichError` → `String` mapping; a shared
  `reqwest::Client` builder (async, rustls, timeout) reused by both clients.
- `shodan.rs` — async client to `https://api.shodan.io`:
  - `host(ip) → GET /shodan/host/{ip}?key=KEY` → `ShodanHost { ip, org, hostnames, ports, services: Vec<{port, product, version}>, cves: Vec<String>, tags }` (parsed from the `data[]` array + `vulns`).
  - `domain(domain) → GET /dns/domain/{domain}?key=KEY` → `ShodanDomain { domain, subdomains: Vec<String>, records: Vec<{type, value}> }`.
  - `search(query) → GET /shodan/host/search?key=KEY&query=Q` → `ShodanSearch { total, matches: Vec<{ip, port, org, product, cves}> }` (capped to the first N matches for the card).
- `leakcheck.rs` — async client to LeakCheck v2 (`X-API-Key`):
  - `query(value, type) → GET /api/v2/query/{value}?type={email|domain}` → `LeakResult { found: u64, sources: Vec<{name, date}>, results: Vec<{email, username, password_present: bool, source}> }` (v2 returns `result[]`; map defensively — fields optional).
- Key absent/blank → `Err("Shodan API key not set — add it in Settings")` (resp. LeakCheck), never a panic; command returns this string so the UI shows a toast.
- All parsing is **defensive** (serde with `#[serde(default)]` / `Option`), because third-party
  JSON shapes drift; a shape we don't recognize yields an empty-but-Ok result, not a hard error.

### 4.2 Config additions (§5)
Add to `Config` (`#[serde(default)]`, camelCase, backward-compat): `shodan_api_key: String`,
`leakcheck_api_key: String`. These ARE Settings-form fields (round-trip through `saveConfig`
like `watchTargets`), so no `preserve_*` change is needed — but they are sensitive: never logged
(no `eprintln!` of config), masked in the UI. `Config`'s `Debug` derive already exists; do **not**
add any code that prints the whole config.

### 4.3 Commands (Tauri, structured JSON — not shell)
Registered in `lib.rs`: `shodan_host(ip)`, `shodan_domain(domain)`, `shodan_search(query)`,
`leakcheck_domain(domain)`, `leakcheck_email(email)`. Each reads the key from `AppState.config`,
calls the client, and returns the typed result (serde camelCase) or the key-absent error string.
`leakcheck_*` ALSO writes any hits into the findings store (§4.5) before returning.

### 4.4 Enrichment store (`watch.db`) — dedup + cache
New table (created alongside the existing `assets`/`artifacts`/`findings` tables in
`watch/store.rs`):
```sql
CREATE TABLE IF NOT EXISTS enrichment (
  target      TEXT NOT NULL,
  kind        TEXT NOT NULL,          -- 'host' | 'email'
  value       TEXT NOT NULL,
  result_json TEXT NOT NULL DEFAULT '',
  first_seen  TEXT NOT NULL,
  PRIMARY KEY (target, kind, value)
);
```
`Store` gains `enrichment_seen(target, kind, value) -> bool` (INSERT OR IGNORE returns
new/seen, mirroring `record_artifact`) and `save_enrichment(target, kind, value, result_json)`.
Adding a table + two methods to `store.rs` is the ONE allowed deviation from Phase 2's
"verbatim port" of that file (documented here so the plan/reviewer expect it); the existing
methods stay byte-identical.

### 4.5 LeakCheck → Findings mapping
A `leakcheck` hit becomes a `HunterFinding` via a mapper (mirrors `watch::sink`): `programName`
= target (or the queried domain/email when run on-demand with no target context), `platform` =
`"LeakCheck"`, `title` = `"Credential exposure: {value} ({found} records)"`, `severity` =
`critical` if any `password_present` (plaintext) else `high` if `found>0` else it writes no
finding; `evidence` = a redacted summary (sources + counts + which fields were present — **never
the plaintext password value itself**, even if the API returns it); `cmdline` = `"leakcheck"`.
Written through `findings::save` (now `SAVE_LOCK`-guarded, Phase 2 — so concurrent Watch +
LeakCheck writes are safe). Severity keys stay lowercase.

### 4.6 Auto-enrich hook (Watch cycle)

**Who has what:** hosts come from the cycle's new endpoint/route artifacts, and emails must be
harvested from the fetched unit **content** — but the engine downloads and then discards that
content inside `run_target`. So the *harvesting* (of raw host + email candidates) happens **inside
the engine** where both content and new artifacts are in scope; the *enrichment* (network calls,
dedup, events, findings) happens in the **scheduler**, keeping all third-party I/O out of the
ported engine logic.

**Engine change (scoped, documented):** add an optional harvest collector to the cycle. When the
scheduler passes one (only for `autoEnrich` targets), `run_target` records, per target:
- **hosts:** `url::Url::parse(v).host_str()` over each **new** endpoint/route artifact value `v`
  it surfaces this cycle;
- **emails:** a regex (`[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}`) over each fetched unit's
  content string (the content it already has in hand before discarding).
The collector is an opt-in sink (e.g. `run_once` gains a variant/param taking `&mut HarvestSink`
or returns `Vec<TargetHarvest { target, hosts, emails }>` when asked). The diff/extract/score
logic is **unchanged**; this only taps values the cycle already computed/downloaded. When no
collector is passed (scheduler off, or target `autoEnrich=off`), behavior is byte-identical to
Phase 2.

**Scheduler (`watch/scheduler.rs::run_cycle`):** after the cycle, for each target's harvest:
- dedup each host via `enrichment_seen(target,'host',h)` and each email via
  `enrichment_seen(target,'email',e)` — skip anything already enriched (persisted forever in
  `watch.db`);
- for each **new** host → `shodan::host`/`domain`; cache result in `enrichment`, emit
  `enrich:host` `{target, host, ports, cves, org}` (Surface Map paints it);
- for each **new** email → `leakcheck::email`; hits → Findings (§4.5);
- throttle between calls (reuse the Watch rate-limit gap / a short sleep) so a target with many
  new hosts doesn't burst the API;
- run the async clients on the scheduler thread via `tauri::async_runtime::block_on` (least-change
  reuse of the shared async client).
Bounded: only NEW (deduped) values per cycle are queried; already-enriched values are skipped
forever.

### 4.7 Events
- `enrich:host` `{ target, host, ports: number[], cves: string[], org }` — auto-enrich Shodan
  result for a host (Surface Map node paint).
- `enrich:progress` `{ target, phase, done, total }` — optional coarse progress for the auto-
  enrich pass (so the UI can show "enriching 3/8"); MAY be folded into `watch:status` if simpler.
- LeakCheck auto-enrich findings surface through the existing `watch:new-finding` reload path
  (they're written to the findings store), so no new finding event is required.

## 5. Frontend

- **Types/bridge/events:** TS `ShodanHost/ShodanDomain/ShodanSearch/LeakResult` + extend `Config`
  with `shodanApiKey`/`leakcheckApiKey`; bridge wrappers for the 5 commands; `onEnrichHost`
  listener.
- **`enrichment` store** (`src/lib/stores/enrichment.ts`): `Map<host, { ports, services, cves,
  org, lastEnriched }>`, updated by `onEnrichHost` + by on-demand `shodan_*` results.
- **⌘K / playbook:** five integration entries. Selecting one prompts for its arg (ip/domain/
  email/query) then invokes the command and renders a **result card** (a new lightweight card
  view — Shodan: org + ports + services + CVE chips, or search hits; LeakCheck: found count +
  sources + redacted field summary). Integration entries are visually distinct from shell
  commands (they don't stream; they resolve to a card).
- **Surface-Map node Enrich:** each node gains an **Enrich** action → `shodan_domain`/`shodan_host`
  on the node's host → `enrichment` store → node renders port + CVE badges (severity-tinted for
  CVEs). Re-enrich allowed (manual override of the once-only cache).
- **Findings:** LeakCheck findings appear in the existing findings surfaces automatically
  (they're in the store). No dedicated panel.
- **Settings:** an **Integrations** section with two **masked** key inputs (`type="password"` +
  a reveal toggle), saved through the existing `saveConfig` path alongside the other fields; a
  short "keys are stored locally and sent only to Shodan/LeakCheck" note. No key value is ever
  echoed to logs/toasts.

## 6. Security & safety

- Keys: local config only, masked in UI, backend→API only; never logged, never in a toast/error
  string, never in a test fixture, never committed. Key-absent → a benign "set it in Settings"
  message.
- LeakCheck PII: treat results as sensitive — store/display a **redacted** summary; never persist
  or render the plaintext password value even when the API returns it.
- Passive lookups sit **outside** the `in_scope` gate (they don't touch the target), but
  auto-enrich only fires for targets the user explicitly set `autoEnrich=on`, and results are
  tagged by target.
- Strict CSP (PR #6) retained: all calls are backend `reqwest` (not webview `fetch`), so
  `connect-src 'self'` is unaffected; no new frontend deps / CDNs.
- Findings data-safety: LeakCheck findings go through the `SAVE_LOCK`-guarded `findings::save`.

## 7. Testing

- **Backend:** client request-building (URL/params/headers correct; key placement) via a
  URL-builder unit seam (no live network in tests); result→card/finding mapping (incl. LeakCheck
  severity: plaintext→critical, found→high, none→no finding; redaction preserved); key-absent
  handling; `enrichment` table dedup (`enrichment_seen` returns new once then seen); the host
  extractor (`url::Url::host_str`) + email regex harvest. TDD. **No key or real PII in any test.**
- **Frontend:** enrichment store updates on `enrich:host`; node Enrich action calls the bridge +
  updates the store; integration command cards render from a mocked result; Settings key inputs
  are masked + round-trip through save (mocked bridge; never a real key).

## 8. Out of scope (restated)

Phase 4 (landing rewrite, license removal, Gumroad retirement); non-Shodan/LeakCheck sources;
auto-enrich on one-off recon commands; historical backfill on first `autoEnrich` toggle.
