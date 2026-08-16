# Phase 1c — Findings, Report Generator, Loot, Settings, Deck — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the cockpit's output/tracking surfaces at parity: the findings tracker + one-click HackerOne/Bugcrowd report generator, the loot panel, settings, and the Deck launcher panel — all wired to the existing backend.

**Architecture:** Builds on the Phase 1a shell + 1b recon core (branch `redesign/free-cockpit`). Backend unchanged. Findings persist via `save_finding {data}` / `load_findings` (returns JSON string) / `delete_finding {id}` — the Rust `HunterFinding` schema. Loot aggregates the flag `findings` the runs store already collects on each run's `done` event. Settings round-trips through the existing config store (the backend `set_config` server-side-merges the `deck_*` fields, so Settings only sends the 4 user fields). The Deck panel drives the existing `deck_start/stop/status/set_folder` bridge commands. All UI recovered/ported from `main:src/main.js` + `main:index.html`.

**Tech Stack:** SvelteKit 2 + Svelte 5 runes, TypeScript, `@tauri-apps/api`, Vitest + `@testing-library/svelte`.

**Spec:** `docs/superpowers/specs/2026-08-15-free-cockpit-rebuild-design.md`

## Global Constraints

- **Parity (spec G5):** the report generator, finding fields, CVSS defaults, OWASP refs, loot markdown, settings, and deck panel must match today's app. Source of truth = `git show main:src/main.js` and `git show main:index.html`.
- **Finding schema (`HunterFinding`, camelCase JSON):** `id, programName, platform, title, severity, status, endpoint, summary, description, steps, evidence, impact, remediation, cvss, cvssScore, notes, cmdline, createdAt, updatedAt`. Persist via the bridge: `saveFinding(finding)` stringifies to `{data}`; `loadFindings()` parses the returned JSON string; `deleteFinding(id)`.
- **Backend read-only** — reference `src-tauri/src/{findings,commands,deck,config}.rs`; do NOT modify anything under `src-tauri/`.
- **Reuse existing infra:** the config store (`$lib/stores/config` — `saveConfig(patch)` server-merges deck fields), the runs store (`$lib/stores/runs` — `runs[].findings`), the bridge, tokens, the cockpit shell + rail affordances. Style through `var(--…)` tokens; prefer `color-mix(in srgb, var(--accent) N%, transparent)` for tints; no new hardcoded palette hex; no external resources (keeps CSP valid).
- **No `innerHTML`/`{@html}` on any finding/loot/report/command text** — untrusted content renders through Svelte text bindings. The generated report markdown is shown in a `<pre>`/textarea (text), never injected as HTML.
- **Clipboard/Discord actions:** copy via `navigator.clipboard.writeText`; Discord via `bridge.sendLoot({markdown})` (loot) — never post to Discord without the user's explicit click.
- **Commit after every task.** Conventional Commits; repo-local noreply email already configured.

---

### Task 1: Findings store + full Finding type + CVSS/OWASP data

**Files:**
- Create: `src/lib/data/cvss.ts`, `src/lib/stores/findings.ts`, `src/lib/stores/findings.test.ts`, `src/lib/data/cvss.test.ts`
- Modify: `src/lib/types.ts` (replace the placeholder `Finding` with the full `HunterFinding` shape)

**Interfaces:**
- Produces: `interface Finding { …all 19 fields… }` (camelCase), `newFinding(): Finding` (blank with a fresh `crypto.randomUUID()` id + empty strings), a `findings` store, `loadFindings()`, `saveFinding(f)`, `deleteFinding(id)` (all via bridge, refreshing the store), and `CVSS_DEFAULTS: Record<Severity, {vector:string; score:string}>` + `OWASP_REFS`.

- [ ] **Step 1: Port the CVSS/OWASP data**

Recover `CVSS_DEFAULTS` (starts ~line 868 in `main:src/main.js`) and `OWASP_REFS` (~line 881) via `git show main:src/main.js`. Put them in `src/lib/data/cvss.ts` as typed exports, verbatim values (vectors + scores per severity: critical/high/medium/low/info).

- [ ] **Step 2: Write the failing tests**

`src/lib/data/cvss.test.ts`:
```ts
import { describe, it, expect } from 'vitest';
import { CVSS_DEFAULTS } from './cvss';
describe('CVSS defaults', () => {
  it('gives critical a 9.x score and an AV:N vector', () => {
    expect(CVSS_DEFAULTS.critical.score.startsWith('9')).toBe(true);
    expect(CVSS_DEFAULTS.critical.vector).toContain('AV:N');
  });
  it('has an entry for every severity', () => {
    for (const s of ['critical','high','medium','low','info'] as const) {
      expect(CVSS_DEFAULTS[s].vector).toMatch(/^CVSS:3\.1/);
    }
  });
});
```
`src/lib/stores/findings.test.ts`:
```ts
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';
vi.mock('$lib/bridge', () => ({
  loadFindings: vi.fn().mockResolvedValue([{ id: 'a', title: 'T', severity: 'high' }]),
  saveFinding: vi.fn().mockResolvedValue(undefined),
  deleteFinding: vi.fn().mockResolvedValue(undefined),
}));
import { findings, loadFindings, saveFinding, deleteFinding, newFinding } from './findings';
import * as bridge from '$lib/bridge';
beforeEach(() => vi.clearAllMocks());
describe('findings store', () => {
  it('loadFindings populates the store from the backend', async () => {
    await loadFindings();
    expect(get(findings)[0].title).toBe('T');
  });
  it('saveFinding persists then reloads', async () => {
    await saveFinding(newFinding());
    expect(bridge.saveFinding).toHaveBeenCalled();
    expect(bridge.loadFindings).toHaveBeenCalled(); // refresh after save
  });
  it('newFinding has a fresh id and empty fields', () => {
    const f = newFinding();
    expect(f.id).toBeTruthy();
    expect(f.title).toBe('');
  });
});
```

- [ ] **Step 3: Run tests → FAIL.** `npm test src/lib/stores/findings.test.ts src/lib/data/cvss.test.ts`

- [ ] **Step 4: Implement**

`src/lib/types.ts`: replace the placeholder `Finding` with the full interface (all 19 `HunterFinding` fields, camelCase, all `string` except none — timestamps are strings). `src/lib/stores/findings.ts`: a `writable<Finding[]>`; `loadFindings()` awaits `bridge.loadFindings()` and sets it; `saveFinding(f)` awaits `bridge.saveFinding(f)` then `loadFindings()`; `deleteFinding(id)` awaits `bridge.deleteFinding(id)` then `loadFindings()`; `newFinding()` returns a blank Finding with `id = crypto.randomUUID()`, `severity:'medium'`, `status:'draft'`, timestamps `''`.

- [ ] **Step 5: Run tests → PASS**, then `npm test` (full) + `npm run check` clean.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat(ui): findings store + full Finding type + CVSS/OWASP data"
```

---

### Task 2: Finding editor

**Files:**
- Create: `src/lib/components/FindingEditor.svelte`, `src/lib/components/FindingEditor.test.ts`

**Interfaces:**
- Consumes: `Finding`, `newFinding`, `saveFinding`, `deleteFinding`, `CVSS_DEFAULTS`.
- Produces: `<FindingEditor>` with props `{ finding: Finding; onSaved?: () => void; onClose?: () => void }` — the full form; changing **Severity** auto-fills `cvss`/`cvssScore` from `CVSS_DEFAULTS` (only when the field is empty or still a default, so manual edits stick); Save calls `saveFinding`.

- [ ] **Step 1: Write the failing test**

`src/lib/components/FindingEditor.test.ts`: mock `$lib/stores/findings` (`saveFinding: vi.fn()`, `newFinding` real-ish, `CVSS_DEFAULTS` real via `$lib/data/cvss`); render `<FindingEditor finding={blank}>`; assert a Title input renders; change Severity to `critical`; assert the CVSS field now shows the critical vector; fill Title; click Save; assert `saveFinding` called with a finding whose `severity==='critical'`.

- [ ] **Step 2: Run it → FAIL.**

- [ ] **Step 3: Implement**

Port the editor field set from `main:index.html` (`#findingEditor`: ffTitle, ffSeverity, ffStatus, ffPlatform, ffProgram, ffEndpoint, ffSummary, ffSteps, ffEvidence, ffImpact, ffRemediation, ffCVSS, ffCVSSScore, ffNotes) into a Svelte form bound to a local `$state` copy of the `finding` prop. Severity is a select (critical/high/medium/low/info) with statuses (draft/ready/submitted/triaged/resolved/na/duplicate) and platforms (h1/bc/synack/intigriti/yeswehack/other) from the old markup. On severity change, if `cvss` is empty or equals another severity's default vector, set `cvss`/`cvssScore` from `CVSS_DEFAULTS[sev]`. Save → `saveFinding(local)` then `onSaved?.()`. Delete button (when editing an existing finding) → `deleteFinding(id)`. Style via tokens (the old `.finding-card` look).

- [ ] **Step 4: Run test → PASS**, then `npm test` + `npm run check`.

- [ ] **Step 5: Commit** `feat(ui): finding editor with severity→CVSS auto-fill`

---

### Task 3: Findings panel + report generator

**Files:**
- Create: `src/lib/reports/generate.ts`, `src/lib/reports/generate.test.ts`, `src/lib/components/FindingsPanel.svelte`, `src/lib/components/ReportModal.svelte`, `src/lib/components/FindingsPanel.test.ts`
- Modify: `src/lib/components/shell/Cockpit.svelte` (open FindingsPanel from the rail 🐛 + host the editor/report modals), `src/lib/components/shell/AppRail.svelte` (Findings affordance if not already present)

**Interfaces:**
- Consumes: `findings` store, `Finding`, `CVSS_DEFAULTS`, `OWASP_REFS`, `saveFinding`/`deleteFinding`, `<FindingEditor>`.
- Produces: `generateReport(f: Finding): string` (the markdown report), `<FindingsPanel>` (list + New + open editor + Generate Report), `<ReportModal>` (shows the markdown in a `<pre>` + Copy).

- [ ] **Step 1: Write the failing report test**

`src/lib/reports/generate.test.ts`:
```ts
import { describe, it, expect } from 'vitest';
import { generateReport } from './generate';
const f = { id:'1', title:'IDOR in /api/users', severity:'high', endpoint:'https://x/api/users/1',
  summary:'S', steps:'1. do', evidence:'curl ...', impact:'bad', remediation:'fix',
  cvss:'CVSS:3.1/AV:N', cvssScore:'8.1', programName:'Acme', platform:'h1', status:'draft',
  description:'', notes:'', cmdline:'', createdAt:'', updatedAt:'' } as any;
describe('generateReport', () => {
  it('includes title, severity, CVSS, and the impact section', () => {
    const md = generateReport(f);
    expect(md).toContain('IDOR in /api/users');
    expect(md).toMatch(/SEVERITY|Severity/);
    expect(md).toContain('8.1');
    expect(md).toContain('bad'); // impact
  });
});
```

- [ ] **Step 2: Run it → FAIL.**

- [ ] **Step 3: Implement the report generator**

Port the report-building logic from `main:src/main.js` (the `#genReportBtn` handler / report-text builder near the finding editor, ~line 1006+). `generateReport(f)` returns the markdown following the app's template — TITLE, SEVERITY, CVSS (vector + score), SUMMARY, VULNERABILITY DESCRIPTION, STEPS TO REPRODUCE, PROOF OF CONCEPT (evidence), IMPACT, REMEDIATION, REFERENCES (from `OWASP_REFS`). Match the old output's section order/labels. Leave `[brackets]` placeholders where the old builder did.

- [ ] **Step 4: Run report test → PASS.**

- [ ] **Step 5: Build the panel + modal + wire the shell**

`FindingsPanel.svelte`: list `$findings` (severity dot + title + program/endpoint + status), a `+ New finding` button (opens `<FindingEditor finding={newFinding()}>`), clicking a row opens the editor for it, a `Generate Report →` action opens `<ReportModal>` with `generateReport(f)`. `ReportModal.svelte`: the markdown in a `<pre class="report">` (text, not HTML) + a Copy-to-Clipboard button + Close. Wire into `Cockpit.svelte`: rail 🐛 opens FindingsPanel; host the editor + report modals; call `loadFindings()` on mount. Do NOT disturb T1b's Terminal/launcher/playbook wiring.

- [ ] **Step 6: Test + check + commit**

`FindingsPanel.test.ts`: mock `$lib/stores/findings` with a seeded finding; render; assert the finding title renders and a `+ New finding` control exists. Then `npm test` + `npm run check`. Commit `feat(ui): findings panel + one-click report generator`.

---

### Task 4: Loot panel

**Files:**
- Create: `src/lib/stores/loot.ts`, `src/lib/stores/loot.test.ts`, `src/lib/components/Loot.svelte`, `src/lib/components/Loot.test.ts`
- Modify: `src/lib/stores/runs.ts` (feed finalized-run findings into loot), `src/lib/components/shell/Cockpit.svelte` (open Loot from the rail 💰)

**Interfaces:**
- Consumes: run `findings` (flag hits `{cat, sev, name, value}` from `flags::Finding`), `bridge.sendLoot`.
- Produces: a `loot` store (deduped `{cat, sev, name, value, cmd}[]`), `addLoot(findings, cmd)` (dedup by `sev|name|value`), `lootMarkdown(): string`, `clearLoot()`; `<Loot>` panel (grouped by severity, counts, Copy Markdown, Send to Discord, Clear).

- [ ] **Step 1: Write the failing store test**

`src/lib/stores/loot.test.ts`:
```ts
import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { loot, addLoot, lootMarkdown, clearLoot } from './loot';
beforeEach(() => clearLoot());
describe('loot store', () => {
  it('dedupes identical flag hits across runs', () => {
    addLoot([{ cat:'secret', sev:'high', name:'AWS key', value:'AKIA…' }], 'cmd1');
    addLoot([{ cat:'secret', sev:'high', name:'AWS key', value:'AKIA…' }], 'cmd2');
    expect(get(loot).length).toBe(1);
  });
  it('lootMarkdown lists collected findings', () => {
    addLoot([{ cat:'recon', sev:'medium', name:'admin', value:'admin.x.com' }], 'c');
    expect(lootMarkdown()).toContain('admin.x.com');
  });
});
```

- [ ] **Step 2: Run it → FAIL.**

- [ ] **Step 3: Implement loot** (port `addLoot`/`lootMarkdown`/`sevRank` from `main:src/main.js` ~lines 642-684). Dedup set keyed `sev|name|value`. `lootMarkdown()` builds the session markdown (title + count + severity-sorted list). Then in `runs.ts`, when a `done` event carries `findings`, call `addLoot(findings, run.cmdline)` (import loot's `addLoot`).

- [ ] **Step 4: Run test → PASS.**

- [ ] **Step 5: Build `Loot.svelte`** — grouped-by-severity list, total count, Copy Markdown (`navigator.clipboard.writeText(lootMarkdown())`), Send to Discord (`bridge.sendLoot({ markdown: lootMarkdown() })` — only on click), Clear. Wire rail 💰 → open Loot in `Cockpit.svelte`. `Loot.test.ts`: seed loot, render, assert an item + the Copy control. Then `npm test` + `npm run check`.

- [ ] **Step 6: Commit** `feat(ui): loot panel — dedup, markdown, copy/send to Discord`

---

### Task 5: Settings

**Files:**
- Create: `src/lib/components/Settings.svelte`, `src/lib/components/Settings.test.ts`
- Modify: `src/lib/components/shell/Cockpit.svelte` (open Settings from the rail ⚙)

**Interfaces:**
- Consumes: `config` store + `saveConfig` (Phase 1a), `bridge.testWebhook`.
- Produces: `<Settings>` — inputs for `webhookUrl`, `username`, `communityDiscord`, and a `shell` select; Save → `saveConfig({webhookUrl, username, communityDiscord, shell})` (backend merges deck fields); a `Send test →` button → `bridge.testWebhook(webhookUrl)`.

- [ ] **Step 1: Write the failing test**

`src/lib/components/Settings.test.ts`: mock `$lib/stores/config` (`config` writable seeded, `saveConfig: vi.fn()`) and `$lib/bridge` (`testWebhook: vi.fn()`); render `<Settings>`; assert the webhook input renders with the seeded value; change it; click Save; assert `saveConfig` called with the four fields incl. the new webhook; click `Send test`; assert `testWebhook` called with the webhook URL.

- [ ] **Step 2: Run it → FAIL.**

- [ ] **Step 3: Implement** (port field set from `main:index.html` `#settings`: setWebhook/setUsername/setCommunity/setShell; shell options: ""/powershell/cmd/bash/sh). Local `$state` seeded from the `config` store; Save → `saveConfig({...})`; `Send test →` → `testWebhook(webhook)`. Style via tokens.

- [ ] **Step 4: Run test → PASS**, then `npm test` + `npm run check`.

- [ ] **Step 5: Commit** `feat(ui): settings panel (webhook/username/community/shell)`

---

### Task 6: Deck launcher panel

**Files:**
- Create: `src/lib/components/Deck.svelte`, `src/lib/components/Deck.test.ts`
- Modify: `src/lib/components/shell/Cockpit.svelte` (open Deck from the rail 📡)

**Interfaces:**
- Consumes: `bridge.deckStart/deckStop/deckStatus/deckSetFolder`, `DeckStatus`.
- Produces: `<Deck>` — Start/Stop controls, a status dot + text, and when running: the QR (the `qrSvg` string), LAN URL, token, tunnel command (each with Copy), and an advanced folder input (Save).

- [ ] **Step 1: Write the failing test**

`src/lib/components/Deck.test.ts`: mock `$lib/bridge` (`deckStatus: vi.fn().mockResolvedValue({running:false})`, `deckStart: vi.fn().mockResolvedValue({running:true, lanUrl:'http://192.168.1.5:8787', token:'abc', qrSvg:'<svg></svg>', url:'http://localhost:8787'})`, `deckStop`, `deckSetFolder`); render `<Deck>`; assert a Start control; click Start; assert `deckStart` called and the LAN URL `192.168.1.5` appears.

- [ ] **Step 2: Run it → FAIL.**

- [ ] **Step 3: Implement** (port the `#deck` modal from `main:index.html` + the deck functions `renderDeck/deckStart/deckStop/deckSaveFolder/deckDot` from `main:src/main.js` ~lines 706-772). The `qrSvg` from `deckStatus`/`deckStart` is TRUSTED (generated by our Rust `qrcode` crate) — it is the ONLY place `{@html}` is acceptable, and ONLY for `qrSvg` (add a code comment saying so). Status dot color via tokens (`--accent` running / `--muted` stopped). On mount, `deckStatus()` to reflect current state. Copy buttons use `navigator.clipboard`.

- [ ] **Step 4: Run test → PASS**, then `npm test` + `npm run check`.

- [ ] **Step 5: Live check** — free port 1420, `npm run tauri dev` (background), confirm Vite ready + cargo Finished + `trapline.exe` alive; note the human should open each rail panel (🐛 Findings → New → Generate Report; 💰 Loot; ⚙ Settings; 📡 Deck → Start → QR). Stop the process after.

- [ ] **Step 6: Commit** `feat(ui): deck launcher panel (start/stop/QR/token/folder)`

---

## Self-Review

- **Spec coverage (1c slice):** findings store + full schema + CVSS/OWASP (T1), finding editor w/ CVSS auto-fill (T2), findings panel + report generator (T3), loot (T4), settings (T5), deck panel (T6). Surface Map / Activity Feed real data = 1d; Watch/Shodan/LeakCheck = Phases 2/3 — out of scope here.
- **Placeholder scan:** the "port from `main:src/main.js`/`index.html`" instructions are recover-verbatim directives with the exact source locations, not code placeholders; every logic step has real code or a precise port target. The single `{@html}` allowance (T6, `qrSvg` only, trusted Rust-generated SVG) is explicitly scoped.
- **Type consistency:** `Finding` (full `HunterFinding`) defined in T1 (`types.ts`) and consumed by T2/T3; `CVSS_DEFAULTS`/`OWASP_REFS` (T1) used by T2/T3; `generateReport` (T3) consumes the T1 `Finding`; `addLoot`/`lootMarkdown` (T4) consume run findings + are called from `runs.ts`; `saveConfig`/`config` (1a) reused by T5; deck bridge (1a) reused by T6.

## Execution Handoff

See the options presented after this plan is saved.
