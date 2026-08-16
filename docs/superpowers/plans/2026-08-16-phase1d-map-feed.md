# Phase 1d — Surface Map + Activity Feed views — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the two static center-pane placeholders (Surface Map, Activity Feed) with live, data-driven views — completing Phase 1's center pane. The Surface Map renders a node graph of hosts discovered from your own recon output; the Activity Feed is a reverse-chronological stream of runs and findings.

**Architecture:** Builds on 1a/1b/1c (branch `redesign/free-cockpit`). No backend changes. Both views derive from EXISTING stores. The **Surface Map** derives host nodes from the `runs` store's output lines + flag hits (grouped by registrable domain), flagging nodes that appear in findings; a node → the finding editor prefilled. The **Activity Feed** merges the `runs` and `findings` stores into a time-sorted event list. Rich enrichment of the map (Shodan ports/CVEs, Watch "new" nodes) is explicitly Phases 2–3.

**Tech Stack:** SvelteKit 2 + Svelte 5 runes, TypeScript, Vitest + `@testing-library/svelte`.

**Spec:** `docs/superpowers/specs/2026-08-15-free-cockpit-rebuild-design.md`

## Global Constraints

- **Backend read-only** — no `src-tauri/` edits.
- **Reuse existing infra:** `runs` (`$lib/stores/runs` — `Run{id,cmdline,lines:{text,stream,spans}[],status,code,ms,findings}`), `findings` (`$lib/stores/findings` — `Finding[]` with `createdAt`/`severity`/`endpoint`/`title`), `loot` (`FlagHit{cat,sev,name,value}`), `FindingEditor`/`newFinding` (1c). Style through `var(--…)` tokens (established `#221a06` on-amber + raw-rgba glass are fine; no NEW orphan hex). Reproduce the map/feed look from the approved mock `C:\Users\shane\AppData\Local\Temp\claude\C--Users-shane\0daa4ed0-dbad-498d-8212-810789c216dd\scratchpad\trapline-cockpit-final.html` (`.v-map` and `.v-feed` sections).
- **No `innerHTML`/`{@html}`** on any host/finding/output text (untrusted) — Svelte text bindings only. (SVG connector lines are authored markup, not injected content — fine.)
- **No external resources** (CSP stays valid). Layout math (node positions) is computed in JS; use inline `style` for positions only.
- **Commit after every task.** Conventional Commits; repo-local noreply email already configured.

---

### Task 1: Surface store — derive a host graph from recon runs

**Files:**
- Create: `src/lib/stores/surface.ts`, `src/lib/stores/surface.test.ts`

**Interfaces:**
- Produces: `extractHosts(text: string): string[]`, `registrableDomain(host: string): string`, `buildSurface(runs: Run[], findings: Finding[]): Scope[]`, and a `surface = derived([runs, findings], …)` store. Types: `interface SurfaceNode { host: string; flagged: boolean; severity?: string }`, `interface Scope { domain: string; nodes: SurfaceNode[] }`.

- [ ] **Step 1: Write the failing tests**

`src/lib/stores/surface.test.ts`:
```ts
import { describe, it, expect } from 'vitest';
import { extractHosts, registrableDomain, buildSurface } from './surface';

describe('surface derivation', () => {
  it('extracts hostnames from an output line, ignoring non-hosts', () => {
    expect(extractHosts('admin.app.acme.com   [401] Admin — auth')).toEqual(['admin.app.acme.com']);
    expect(extractHosts('https://api.app.acme.com/v2 ok')).toContain('api.app.acme.com');
    expect(extractHosts('just some words, no host here')).toEqual([]);
  });
  it('registrableDomain takes the last two labels', () => {
    expect(registrableDomain('admin.app.acme.com')).toBe('acme.com');
    expect(registrableDomain('acme.com')).toBe('acme.com');
  });
  it('groups discovered hosts under their registrable domain and flags finding-hosts', () => {
    const runs = [{ id:'r1', cmdline:'subfinder -d acme.com', status:'done', lines:[
      { text:'api.acme.com [200]', stream:'out', spans:[] },
      { text:'admin.acme.com [401]', stream:'out', spans:[] },
    ], findings:[] }] as any;
    const findings = [{ id:'f', endpoint:'https://admin.acme.com/x', severity:'high', title:'Admin' }] as any;
    const scopes = buildSurface(runs, findings);
    const acme = scopes.find((s) => s.domain === 'acme.com')!;
    expect(acme.nodes.map((n) => n.host).sort()).toEqual(['admin.acme.com','api.acme.com']);
    const admin = acme.nodes.find((n) => n.host === 'admin.acme.com')!;
    expect(admin.flagged).toBe(true);
    expect(admin.severity).toBe('high');
  });
});
```

- [ ] **Step 2: Run tests → FAIL.** `npm test src/lib/stores/surface.test.ts`

- [ ] **Step 3: Implement**

`src/lib/stores/surface.ts`:
- `extractHosts(text)`: match with `/(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z]{2,}/gi`, lowercase, dedup, drop obvious non-hosts (e.g. anything with no dot — the regex already requires one). Strip a leading scheme by matching inside URLs too (the regex handles `api.acme.com` inside `https://api.acme.com/v2`).
- `registrableDomain(host)`: `host.split('.').slice(-2).join('.')` (documented heuristic — does not handle multi-label TLDs like `co.uk`; acceptable for v1, note it in a comment).
- `buildSurface(runs, findings)`: gather all hosts from every run's `lines[].text` (via `extractHosts`); build a lowercase set of "flagged" host substrings from findings (`endpoint` hostnames + any finding `title`/loot value that is a host) with their severity; group hosts by `registrableDomain`; each `Scope` = `{ domain, nodes:[{host, flagged, severity}] }`, sorted so flagged/most-recent first. Skip the registrable-domain host itself from `nodes` if you render it as the center separately (your choice — keep it simple: nodes = all discovered hosts, the domain is the center label).
- `surface = derived([runs, findings], ([$r, $f]) => buildSurface($r, $f))`.

- [ ] **Step 4: Run tests → PASS**, then `npm test` (full) + `npm run check`.

- [ ] **Step 5: Commit** `feat(ui): surface store — derive host graph from recon output`

---

### Task 2: Surface Map view

**Files:**
- Create: `src/lib/components/views/SurfaceMap.svelte`, `src/lib/components/views/SurfaceMap.test.ts`
- Modify: `src/lib/components/shell/Cockpit.svelte` (render `<SurfaceMap>` as the `map` view instead of the placeholder; wire a node→finding action to the existing FindingEditor)

**Interfaces:**
- Consumes: `surface` store, `Scope`/`SurfaceNode`, `newFinding` (1c) + the Cockpit's existing FindingEditor host.
- Produces: `<SurfaceMap>` rendering the active scope as a node graph + an inspector; emits `onCreateFinding(host: string)` (callback prop) when the inspector's "→ Finding" is clicked.

- [ ] **Step 1: Write the failing render test**

`src/lib/components/views/SurfaceMap.test.ts`: mock `$lib/stores/surface` with a `surface` writable seeded with one scope (`{domain:'acme.com', nodes:[{host:'admin.acme.com', flagged:true, severity:'high'}, {host:'api.acme.com', flagged:false}]}`); render `<SurfaceMap>`; assert the center domain `acme.com` and a node `admin.acme.com` both render; click the `admin.acme.com` node; assert an inspector shows the host and a "→ Finding" control.

- [ ] **Step 2: Run it → FAIL.**

- [ ] **Step 3: Implement**

`SurfaceMap.svelte`: pick the active scope (`$derived` — the first/most-recent scope, with a small scope selector if there are several). Compute a radial layout: center node at 50%/50%, children evenly on a circle (`angle = i/N * 2π`, radius ~34%), positions as inline `left`/`top` percentages. Render an absolutely-positioned SVG behind the nodes with `<line>` from center to each child (percentage coords). Nodes: `.node` boxes (center = domain, children = hosts); flagged nodes get a severity-colored border/dot (`--crit`/`--high`/`--med`). Clicking a node opens an inspector (docked panel or overlay) showing the host + flagged severity + a "→ Finding" button calling `onCreateFinding(host)`. Empty state ("Run recon to map the surface") when `surface` is empty. Match the mock's `.v-map`/`.node` styling via tokens.
Modify `Cockpit.svelte`: render `<SurfaceMap onCreateFinding={openFindingForHost}>` for the `map` view (replace the placeholder). `openFindingForHost(host)` opens the existing FindingEditor with `{ ...newFinding(), endpoint: 'https://' + host }` (reuse the FindingsPanel/editor host + the `{#key}` pattern). Do NOT disturb Terminal/Activity/launcher/command-bar/rail/dock/status/panels wiring.

- [ ] **Step 4: Run test → PASS**, then `npm test` + `npm run check`.

- [ ] **Step 5: Commit** `feat(ui): Surface Map view — node graph of discovered hosts + node→finding`

---

### Task 3: Activity feed aggregation

**Files:**
- Create: `src/lib/stores/activity.ts`, `src/lib/stores/activity.test.ts`
- Modify: `src/lib/stores/runs.ts` (stamp `startedAt: number` on each run in `startRun`)

**Interfaces:**
- Consumes: `runs`, `findings`.
- Produces: `interface ActivityEvent { kind: 'recon' | 'finding'; ts: number; title: string; sub?: string; severity?: string; status?: string }`, `buildActivity(runs, findings): ActivityEvent[]` (newest-first), `activity = derived([runs, findings], …)`.

- [ ] **Step 1: Stamp runs with a timestamp**

In `src/lib/stores/runs.ts`, add `startedAt: number` to the `Run` interface and set `startedAt: Date.now()` in `startRun` (additive; existing tests unaffected — but if any construct a `Run` literal in a test, they may need the field — check and update `runs.test.ts` only if the compiler/tests require it).

- [ ] **Step 2: Write the failing test**

`src/lib/stores/activity.test.ts`:
```ts
import { describe, it, expect } from 'vitest';
import { buildActivity } from './activity';
describe('activity feed', () => {
  it('merges runs + findings newest-first', () => {
    const runs = [{ id:'r', cmdline:'subfinder -d acme.com', status:'done', startedAt: 1000, findings:[{},{}] }] as any;
    const findings = [{ id:'f', title:'Admin panel', severity:'high', createdAt: new Date(2000).toISOString() }] as any;
    const ev = buildActivity(runs, findings);
    expect(ev[0].kind).toBe('finding');   // ts 2000 newer than 1000
    expect(ev[1].kind).toBe('recon');
    expect(ev[0].severity).toBe('high');
    expect(ev[1].sub).toContain('2');       // flag/finding count summary
  });
});
```

- [ ] **Step 3: Run it → FAIL.**

- [ ] **Step 4: Implement**

`activity.ts`: `buildActivity(runs, findings)` maps each run → `{ kind:'recon', ts: run.startedAt ?? 0, title: run.cmdline, sub: `${run.findings.length} flag(s) · ${run.status}`, status: run.status }` and each finding → `{ kind:'finding', ts: Date.parse(finding.createdAt) || 0, title: finding.title || '(untitled)', sub: finding.endpoint || finding.programName, severity: finding.severity }`; concat and sort by `ts` descending. `activity = derived([runs, findings], ([$r,$f]) => buildActivity($r,$f))`.

- [ ] **Step 5: Run test → PASS**, then `npm test` + `npm run check`.

- [ ] **Step 6: Commit** `feat(ui): activity feed store — merge runs + findings into a timeline`

---

### Task 4: Activity Feed view + live check

**Files:**
- Create: `src/lib/components/views/ActivityFeed.svelte`, `src/lib/components/views/ActivityFeed.test.ts`
- Modify: `src/lib/components/shell/Cockpit.svelte` (render `<ActivityFeed>` as the `feed` view instead of the placeholder)

**Interfaces:**
- Consumes: `activity` store, `ActivityEvent`.
- Produces: `<ActivityFeed>` — a reverse-chronological timeline with a type filter.

- [ ] **Step 1: Write the failing render test**

`src/lib/components/views/ActivityFeed.test.ts`: mock `$lib/stores/activity` with an `activity` writable seeded with one `recon` + one `finding` event; render `<ActivityFeed>`; assert both event titles render; assert filter controls exist (e.g. All / Recon / Findings); clicking `Findings` hides the recon event.

- [ ] **Step 2: Run it → FAIL.**

- [ ] **Step 3: Implement**

`ActivityFeed.svelte`: a vertical timeline (rail line + dots, per the mock `.v-feed .tl .ev`), each event a card with a type marker (recon ▚ / finding 🐛), title, `sub`, a severity chip when present, and a relative time (`Nm`/`Nh` ago — a small pure helper is fine). A filter row (`$state`) with All / Recon / Findings chips filtering `$activity`. Empty state ("No activity yet — run a command"). Tokens for all colors (event dot: recon `--accent2`, finding `--accent`; severity chips reuse `--crit/--high/--med`).
Modify `Cockpit.svelte`: render `<ActivityFeed>` for the `feed` view (replace placeholder). Do NOT disturb any other wiring.

- [ ] **Step 4: Run test → PASS**, then `npm test` (full) + `npm run check`.

- [ ] **Step 5: Live check**

Free port 1420, `npm run tauri dev` (background); confirm Vite ready + cargo `Finished` + `trapline.exe` alive. Note the human should: run a command (⌘K → `crt.sh`, `{{domain}}`=`example.com`, Run) → then click **Surface Map** (nodes should appear for discovered hosts) and **Activity** (the run event should appear). Stop the process after.

- [ ] **Step 6: Commit** `feat(ui): Activity Feed view — timeline of runs + findings with filters`

---

## Self-Review

- **Spec coverage (1d slice):** Surface Map view derived from recon output (T1 store + T2 view), Activity Feed from runs+findings (T3 store + T4 view). Both center-pane placeholders replaced. Shodan/Watch enrichment of the map is explicitly Phases 2–3 — out of scope. This completes Phase 1's center pane.
- **Placeholder scan:** the `registrableDomain` "last two labels" heuristic and the host regex are documented, bounded choices with tests, not placeholders; every code step has real code.
- **Type consistency:** `Scope`/`SurfaceNode` defined in T1 and consumed by T2; `ActivityEvent`/`buildActivity` defined in T3 and consumed by T4; `runs.startedAt` added in T3 and read by T3's `buildActivity`; `newFinding`/FindingEditor reused from 1c in T2.

## Execution Handoff

See the options presented after this plan is saved.
