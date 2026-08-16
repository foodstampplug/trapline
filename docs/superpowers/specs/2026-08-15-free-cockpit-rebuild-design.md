# Trapline — Free Cockpit Rebuild (design spec)

- **Date:** 2026-08-15
- **Status:** Approved design, pre-plan
- **Repo:** `C:\Users\shane\trapline-v2` (Tauri v2 app)
- **Related:** [[project-trapline-tool]], [[project-trapline-watch]], [[project-trapline-deck]]

## 1. Summary

Trapline becomes **one free product** with a ground-up new front end. A SvelteKit
**cockpit** replaces the current vanilla-JS UI; **Watch** (today a separate paid
product) is absorbed as a built-in feature; **Shodan** and **LeakCheck** are added
as first-class recon integrations; and the **paid tier is retired** — the license
gate is removed and the Gumroad listings are taken down.

The Rust backend is **preserved and extended**, not rewritten. Only the front end is
rebuilt. All existing capability is retained at parity (see §4.1) and new capability
is added on top.

## 2. Goals / Non-goals

**Goals**
- G1. Ground-up SvelteKit front end in the approved "cockpit" structure and the
  "Obsidian-amber" aesthetic.
- G2. Absorb Watch's change-detection engine into the app as a built-in, in-process
  feature (monitor-while-open MVP).
- G3. Add Shodan + LeakCheck integrations (hybrid on-demand / opt-in auto-enrich).
- G4. Make everything free: remove the license gate, retire Gumroad, rewrite the
  landing page.
- G5. **Zero feature regression** vs. today's shipping app.

**Non-goals (this initiative)**
- Tray icon, run-at-Windows-startup, background/headless monitoring (Watch runs only
  while the app is open).
- Watch engine roadmap items P1–P3 (AST diff, scope radar, Ollama triage) — the
  ported engine keeps exactly what it has today.
- macOS/Linux builds and code signing (tracked separately).
- Merging any *other* Trapline-family product.

## 3. The moves at a glance

| Move | Surface | Effort |
|---|---|---|
| Rebuild front end (SvelteKit cockpit) | `src/` → new SvelteKit app | Large |
| Merge Watch engine in-process | `src-tauri/src/watch/` (port) | Medium |
| Shodan + LeakCheck clients | `src-tauri/src/integrations/` | Medium |
| Go free | `trapline-site/`, remove gate, Gumroad | Small |

## 4. Target architecture

### 4.1 Front end — the cockpit (SvelteKit)

- **Stack:** SvelteKit + `adapter-static` (SPA mode) for Tauri, matching the Forge /
  Spotify builds. Vite bundler (already in place). All assets same-origin (keeps the
  strict CSP shipped in PR #6 valid — see §6).
- **Shell (Mission Control):** persistent frame — top bar, left app-rail, targets
  panel, center pane, right dock (Watch + Findings), bottom status strip.
- **Launcher (⌘K):** a command palette that fuzzy-searches **all 215 playbook
  commands** plus the new integration commands; also the primary "run a command"
  surface. Rides the top bar and opens as an overlay.
- **Center pane — three switchable views:**
  - **Terminal** — live command output as cards, with flag highlighting (the current
    card/stream model).
  - **Surface Map** — spatial node graph of the attack surface (target → subdomains →
    endpoints), Shodan ports/CVEs shown on nodes, new/flagged nodes highlighted, a
    node → finding action.
  - **Activity Feed** — chronological stream of recon runs, Watch changes, and
    integration hits; Watch-first, filterable.
- **Right dock:** Watch live status + the Findings list (the report-generator entry).
- **Design system:** "Obsidian-amber" tokens — cool near-black ground (`#0a0b0e`),
  frosted-glass panels, hairline light borders, amber accent (`#ffbf47`) with a cool
  blue complement (`#7aa2ff`), semantic severity colors. Fonts: a clean UI sans + a
  mono for data (real faces bundled: JetBrains Mono + a UI face). Tokens defined once;
  every component styled through them.

**Parity requirements (hard — G5).** The rebuild MUST preserve, with no loss:
- **All 215 playbook commands / 30 categories.** These live as the `TEMPLATES` data
  array in `src/main.js` (`[{cat, items:[{name, desc, tool, cmd}]}]`, `{{placeholder}}`
  variables). Ported **verbatim** as a data module the new front end imports.
- **Playbook UX:** browse-by-category + fill-in-the-`{{blanks}}` (today's drawer),
  reachable from both the ⌘K launcher and a Playbook view.
- **Findings tracker → one-click HackerOne/Bugcrowd report generator** (auto CVSS,
  OWASP ref, impact statement).
- **Loot panel, tool checker (22 tools), Discord webhook, the Deck launcher (📡).**
- **94 flag-detection rules** in `src-tauri/src/flags.rs` (backend, untouched).

### 4.2 Back end (Rust) — preserved + extended

- **Unchanged:** every current Tauri command (run/stream, send_card/send_loot/
  test_webhook → `discord.rs`, config, findings.rs, flags.rs, deck.rs). The front-end
  rebuild re-wires `invoke()` calls to the **same** command surface.
- **New modules:**
  - `watch/` — the ported Watch engine (§4.3).
  - `integrations/shodan.rs`, `integrations/leakcheck.rs` — API clients (§4.4).
- **New commands (illustrative):** `watch_*` (start/stop/status/add/remove/list/
  run_once), `shodan_host/shodan_domain/shodan_search`, `leakcheck_domain/
  leakcheck_email`, plus an `enrich_target` orchestrator.
- **Events:** `watch:status`, `watch:new-finding`, `enrich:progress` emitted to the
  front end for live updates.

### 4.3 Watch merge (in-process)

Port `trapline-watch/src/{engine,fetch,normalize,parse,score,store,sourcemap}.rs`
into `src-tauri/src/watch/` **as-is** (logic unchanged). Rewire the three edges:
- **Config in:** watch targets + `interval_secs`, `alert_threshold`, `max_requests_per_min`
  move into the app `Config` (JSON, `%APPDATA%\Trapline\config.json`), reusing the
  existing Discord `webhook_url`. SQLite baseline lives at `%APPDATA%\Trapline\watch.db`.
- **Alerts out:** reuse `discord.rs` (`send_embed`) instead of `alert.rs`.
- **Findings out:** write **directly** to the in-process findings store (`findings.rs`)
  instead of the cross-process `findings.json` sync — this removes the known
  last-writer-wins race in Watch v0.
- **Scheduler:** a background thread (guarded by `AtomicBool` + `Mutex`, the `deck.rs`
  pattern) sleeps `interval_secs` and calls `engine::run_once`, using **blocking
  reqwest on its own thread** (least-change port). Start/stop via commands; state
  persists and **auto-resumes on app launch** if it was on.
- **Drop:** `license.rs` (and the CLI `main.rs`) — Watch is free and GUI-driven.
- **Keep:** the per-target `in_scope` safety gate and the global rate limit.

### 4.4 Shodan + LeakCheck

- **Clients:** Rust `reqwest` calls straight from the backend to
  `api.shodan.io` and `leakcheck.io` (v2, `X-API-Key`).
- **Commands:** `shodan host <ip>`, `shodan domain <domain>`, `shodan search <q>`;
  `leakcheck domain <domain>`, `leakcheck email <addr>` — also in the playbook/⌘K.
- **Enrich mode — hybrid:** on-demand by default (run a command, or one-click
  "Enrich" on a node/finding). A **per-target `auto_enrich` toggle**: when on, every
  newly discovered host is queried against Shodan (ports/CVEs onto the Surface Map)
  and every discovered email against LeakCheck, on each recon pass.
- **Result mapping:** Shodan → recon cards + Surface Map node enrichment (ports,
  services, CVEs). LeakCheck → **Findings** (credential exposure), tagged by target.
- **Keys:** stored in the app `Config` (Settings UI, masked), sent only backend→API,
  **never hardcoded or committed**. No key ever passes through chat or the repo.
- **Safety:** both are **passive third-party lookups** (they do not touch the target),
  so they sit outside the `in_scope` fetch gate, but results are tagged by target and
  LeakCheck's PII is handled as sensitive.

### 4.5 Going free

- **Landing page** (`trapline-site/index.html`): remove all Pro/Gumroad/pricing refs
  (~27), reframe as one **free** download; Watch becomes a built-in feature bullet, not
  a paid add-on. Keep the GitHub free-download CTA.
- **License:** removed from the shipped product (the Watch gate is dropped by the port;
  the main app has no in-app gate today).
- **Standalone `trapline-watch` repo:** left in place, marked **deprecated → superseded
  by built-in**. Not deleted.
- **Gumroad:** the *Trapline Pro* and *Trapline Watch* listings are **unpublished by the
  owner** (cannot be done from here — a hand-off checklist is provided).

## 5. Config schema additions

Add to `Config` (all `#[serde(default)]`, camelCase JSON), preserving backward-compat:
- `shodan_api_key: String`, `leakcheck_api_key: String`
- `watch_targets: Vec<WatchTarget>` (`{name, pages, js, in_scope, auto_enrich}`)
- `watch_interval_secs: u64` (default 1800), `watch_alert_threshold: i64`,
  `watch_max_rpm: u32`, `watch_enabled: bool`
- Existing deck/webhook/username fields unchanged.

## 6. Security & safety

- **Keys** live in local config only, masked in UI, backend→API only (§4.4).
- **Strict CSP** (PR #6) is retained; the SvelteKit output must stay same-origin (no
  external fonts/CDNs — bundle faces as assets) so `script-src 'self'` still holds.
- **in_scope** gate + rate limits preserved for active recon; Shodan/LeakCheck are
  passive.
- **Findings data-safety** (atomic writes, fail-loud on corrupt store — PR #3) is
  reused for the in-process Watch writes.

## 7. Phased implementation roadmap

This spec is **too large for one implementation plan**. It ships as sequenced phases,
each its own plan (`writing-plans`) and its own PR(s):

- **Phase 1 — Front-end cockpit at parity.** Scaffold SvelteKit + adapter-static; build
  the cockpit shell, ⌘K launcher, three center views, Playbook (215 templates ported),
  Findings/report/loot/tools/webhook/Deck — all wired to the **existing** backend
  commands. Watch/Shodan/LeakCheck UI surfaces are present but call backend commands
  delivered in later phases (feature-flagged/stubbed until then). **Definition of done:
  full parity with today's app on the new UI.**
- **Phase 2 — Watch merge.** Port the engine, scheduler, config, in-process findings;
  light up the Watch rail + Activity feed + Surface Map "new node" behavior.
- **Phase 3 — Shodan + LeakCheck.** Clients, commands, hybrid enrich, Settings keys,
  Surface Map/Findings mapping.
- **Phase 4 — Go free.** Landing-page rewrite, deprecate standalone repo, Gumroad
  hand-off checklist; final release refresh.

Phase 1 is the first `writing-plans` target immediately after this spec is approved.

## 8. Testing

- **Backend:** unit tests for new modules (Watch scheduler start/stop, config
  round-trip incl. new fields, integration client request-building + result→finding
  mapping, key-absent handling). Reuse Watch's existing engine tests. TDD.
- **Front end:** component tests for the launcher search over `TEMPLATES`, view
  switching, and the `{{placeholder}}` fill/resolve; a parity checklist verifying all
  215 commands render and resolve.
- **Build gates:** `cargo build`/`cargo test`, `npm run build` clean, CSP-safe
  (same-origin) output, app launches and renders (live check).
- Same subagent-driven-development + review flow as the Deck launcher.

## 9. Risks / open questions

- **Front-end rebuild is the biggest risk to parity.** Mitigation: Phase 1 is
  explicitly "parity on the new shell," with a 215-command checklist as a gate before
  any new feature work.
- **Bundled fonts vs. CSP:** faces must be bundled as assets (no CDN) to keep
  `script-src/style-src` strict. Confirmed approach.
- **Watch-while-open only:** accepted MVP limit; tray/headless deferred (§2).
- **API quotas:** hybrid enrich bounds Shodan/LeakCheck spend; document per-run counts.

## 10. Out of scope (restated)

Tray/autostart/headless; Watch engine P1–P3; macOS/Linux; code signing; other
Trapline-family products.
