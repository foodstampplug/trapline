# Phase 1b — Recon Core (Playbook, Launcher, Terminal) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring the cockpit to functional recon parity: port all 215 playbook commands, build the ⌘K launcher and command bar with `{{blank}}` fill-in, and make the Terminal view actually run commands with live streaming + flag highlighting + findings — all against the existing Rust backend.

**Architecture:** Builds on the Phase 1a shell (branch `redesign/free-cockpit`). The backend is unchanged: `run_command` streams `q_event` Tauri events (`{id, type:'line'|'done', text, stream, spans, code, ms, findings}`), where `spans` are flag highlights from `flags::scan_line` and `findings` aggregate on done. The front end ports the old `TEMPLATES` data verbatim, adds a small template engine + a typed event helper, and replaces the placeholder Terminal/launcher with live components.

**Tech Stack:** SvelteKit 2 + Svelte 5 (runes), TypeScript, `@tauri-apps/api` (`core.invoke`, `event.listen`), Vitest + `@testing-library/svelte`.

**Spec:** `docs/superpowers/specs/2026-08-15-free-cockpit-rebuild-design.md`

## Global Constraints

- **Parity is the point (spec G5):** all 215 commands / 30 categories must be present and runnable; no command dropped or reworded. Source of truth = the `TEMPLATES` array at lines 25–364 of `main:src/main.js` (recover via `git show main:src/main.js`).
- **Backend is read-only** — reference `src-tauri/src/{runner,flags,commands}.rs` for shapes; do NOT modify anything under `src-tauri/`.
- **Reuse Phase 1a infra:** the typed bridge (`src/lib/bridge.ts` — `runCommand({id, cmdline})`, `cancelCommand(id)`, `toolCheck()`), the design tokens, the cockpit shell. Style everything through the existing `var(--…)` tokens; no new hardcoded palette hex, no external resources (keeps the strict CSP valid).
- **Streaming contract (verbatim):** listen to event name `"q_event"`. Line event: `{id, type:'line', text, stream:'out'|'err', spans}`. Done event: `{id, type:'done', code, ms, findings}`. `Span` fields as serialized by `flags.rs` (verify: the old front end read `sp.s, sp.e, sp.cat, sp.sev, sp.label`).
- **Template syntax:** fill-ins are `{{name}}` where name matches `\w+`. `resolveCmd` replaces each with its var value, leaving `{{name}}` literally when the value is empty/absent.
- **No `innerHTML` for command output** — build segment arrays and render through Svelte text bindings (auto-escaped). The backend command strings themselves are trusted templates, but streamed output is untrusted and must never be injected as HTML.
- **Commit after every task.** Conventional Commits; repo-local noreply email is already configured.

---

### Task 1: Port the 215-command TEMPLATES data

**Files:**
- Create: `src/lib/data/templates.ts`, `src/lib/data/templates.test.ts`

**Interfaces:**
- Produces: `interface Template { name: string; desc?: string; tool?: string; cmd: string }`, `interface TemplateCategory { cat: string; items: Template[] }`, `export const TEMPLATES: TemplateCategory[]`, and `export function flattenTemplates(): (Template & { cat: string })[]`.

- [ ] **Step 1: Recover the data block**

Run: `git show main:src/main.js > /tmp/oldmain.js` (or read via `git show`). Copy the literal array assigned to `const TEMPLATES = [ … ];` (starts line 25 `const TEMPLATES = [`, ends line 364 `];`). This is pure data — do not edit any `name`/`desc`/`tool`/`cmd` string.

- [ ] **Step 2: Write the failing test**

`src/lib/data/templates.test.ts`:
```ts
import { describe, it, expect } from 'vitest';
import { TEMPLATES, flattenTemplates } from './templates';

describe('TEMPLATES data', () => {
  it('has all 30 categories', () => {
    expect(TEMPLATES.length).toBe(30);
  });
  it('preserves a known Quickfire command verbatim', () => {
    const all = flattenTemplates();
    const kong = all.find((t) => t.name === 'Kong portal UUID leak');
    expect(kong).toBeDefined();
    expect(kong!.cmd).toContain('/api/v3/portal');
    expect(kong!.cat).toBe('Quickfire');
  });
  it('flatten count equals the sum of category items', () => {
    const sum = TEMPLATES.reduce((n, c) => n + c.items.length, 0);
    expect(flattenTemplates().length).toBe(sum);
    expect(sum).toBeGreaterThanOrEqual(215);
  });
});
```

> The implementer MUST confirm the real category count from the recovered data and set the `toBe(30)` assertion to the actual number if it differs (the spec says "30 categories" — verify against the data and make the test assert the true count). Same for the `>= 215` floor.

- [ ] **Step 3: Run test to verify it fails**

Run: `npm test src/lib/data/templates.test.ts` → FAIL (`./templates` missing).

- [ ] **Step 4: Create the typed module**

`src/lib/data/templates.ts`:
```ts
export interface Template { name: string; desc?: string; tool?: string; cmd: string }
export interface TemplateCategory { cat: string; items: Template[] }

export const TEMPLATES: TemplateCategory[] = [
  /* … paste the recovered array literal here, verbatim … */
];

export function flattenTemplates(): (Template & { cat: string })[] {
  return TEMPLATES.flatMap((c) => c.items.map((it) => ({ ...it, cat: c.cat })));
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `npm test src/lib/data/templates.test.ts` → PASS. Then `npm run check` (svelte-check) must be clean (the literal is valid TS).

- [ ] **Step 6: Commit**

```bash
git add src/lib/data/templates.ts src/lib/data/templates.test.ts
git commit -m "feat(ui): port all 215 playbook commands as typed TEMPLATES data"
```

---

### Task 2: Template engine (extract vars + resolve)

**Files:**
- Create: `src/lib/templates/engine.ts`, `src/lib/templates/engine.test.ts`

**Interfaces:**
- Produces: `export function extractVars(cmd: string): string[]` (unique `{{\w+}}` names, first-seen order), `export function resolveCmd(cmd: string, vars: Record<string, string>): string` (replace each `{{name}}` with `vars[name]` when non-empty, else leave `{{name}}`).

- [ ] **Step 1: Write the failing test**

`src/lib/templates/engine.test.ts`:
```ts
import { describe, it, expect } from 'vitest';
import { extractVars, resolveCmd } from './engine';

describe('template engine', () => {
  it('extracts unique vars in first-seen order', () => {
    expect(extractVars('curl {{url}}/{{path}} then {{url}}')).toEqual(['url', 'path']);
  });
  it('returns [] when there are no vars', () => {
    expect(extractVars('subfinder -d example.com')).toEqual([]);
  });
  it('resolves provided vars and leaves the rest as literal placeholders', () => {
    expect(resolveCmd('curl {{url}}/{{path}}', { url: 'https://x' })).toBe('curl https://x/{{path}}');
  });
  it('treats an empty-string value as unfilled', () => {
    expect(resolveCmd('curl {{url}}', { url: '' })).toBe('curl {{url}}');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test src/lib/templates/engine.test.ts` → FAIL.

- [ ] **Step 3: Implement**

`src/lib/templates/engine.ts`:
```ts
const VAR_RE = /\{\{(\w+)\}\}/g;

export function extractVars(cmd: string): string[] {
  const seen = new Set<string>();
  for (const m of cmd.matchAll(VAR_RE)) seen.add(m[1]);
  return [...seen];
}

export function resolveCmd(cmd: string, vars: Record<string, string>): string {
  return cmd.replace(VAR_RE, (m, name) => (vars[name] ? vars[name] : m));
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test src/lib/templates/engine.test.ts` → PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/templates/engine.ts src/lib/templates/engine.test.ts
git commit -m "feat(ui): template var-extract + resolve engine"
```

---

### Task 3: Typed q_event stream helper

**Files:**
- Create: `src/lib/events.ts`, `src/lib/events.test.ts`

**Interfaces:**
- Consumes: `listen` from `@tauri-apps/api/event`.
- Produces: `export interface Span { s: number; e: number; cat: string; sev: string; label: string }`, `export interface QEvent { id: string; type: 'line' | 'done'; text?: string; stream?: 'out' | 'err'; spans?: Span[]; code?: number; ms?: number; findings?: unknown[] }`, `export function onQEvent(handler: (e: QEvent) => void): Promise<() => void>`.

- [ ] **Step 1: Confirm the Span shape**

Read `src-tauri/src/flags.rs` and confirm the serialized field names of `Span` (the old front end used `s, e, cat, sev, label`). If serde renames differ, set the `Span` interface to the real JSON field names and note it in the report.

- [ ] **Step 2: Write the failing test**

`src/lib/events.test.ts`:
```ts
import { describe, it, expect, vi } from 'vitest';
const listen = vi.fn().mockResolvedValue(() => {});
vi.mock('@tauri-apps/api/event', () => ({ listen: (...a: unknown[]) => listen(...a) }));
import { onQEvent } from './events';

describe('onQEvent', () => {
  it('subscribes to "q_event" and forwards the payload to the handler', async () => {
    const seen: unknown[] = [];
    await onQEvent((e) => seen.push(e));
    expect(listen).toHaveBeenCalledWith('q_event', expect.any(Function));
    // simulate a backend emit
    const cb = listen.mock.calls[0][1] as (ev: { payload: unknown }) => void;
    cb({ payload: { id: 'j1', type: 'line', text: 'hi', spans: [] } });
    expect(seen).toEqual([{ id: 'j1', type: 'line', text: 'hi', spans: [] }]);
  });
});
```

- [ ] **Step 3: Run test to verify it fails**

Run: `npm test src/lib/events.test.ts` → FAIL.

- [ ] **Step 4: Implement**

`src/lib/events.ts`:
```ts
import { listen } from '@tauri-apps/api/event';

export interface Span { s: number; e: number; cat: string; sev: string; label: string }
export interface QEvent {
  id: string; type: 'line' | 'done';
  text?: string; stream?: 'out' | 'err'; spans?: Span[];
  code?: number; ms?: number; findings?: unknown[];
}

export function onQEvent(handler: (e: QEvent) => void): Promise<() => void> {
  return listen<QEvent>('q_event', (ev) => handler(ev.payload));
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `npm test src/lib/events.test.ts` → PASS.

- [ ] **Step 6: Commit**

```bash
git add src/lib/events.ts src/lib/events.test.ts
git commit -m "feat(ui): typed q_event stream helper"
```

---

### Task 4: Terminal view — run, stream, flag-highlight, findings

**Files:**
- Create: `src/lib/stores/runs.ts`, `src/lib/stores/runs.test.ts`, `src/lib/components/views/Terminal.svelte`, `src/lib/components/views/OutputLine.svelte`, `src/lib/components/views/Terminal.test.ts`
- Modify: `src/lib/components/shell/Cockpit.svelte` (mount `<Terminal />` as the Terminal view instead of the placeholder)

**Interfaces:**
- Consumes: `bridge.runCommand`, `bridge.cancelCommand`, `onQEvent`, `Span`/`QEvent`, `extractVars`/`resolveCmd` (for the caller in Task 5).
- Produces: a `runs` store of `Run` objects (`{ id, cmdline, lines: {text, stream, spans}[], status:'running'|'done', code?, ms?, findings: unknown[] }`), `startRun(cmdline: string): string` (creates a run, calls `runCommand`, returns id), and the `<Terminal>` view rendering the runs newest-first.

- [ ] **Step 1: Write the failing store test**

`src/lib/stores/runs.test.ts`:
```ts
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';
vi.mock('$lib/bridge', () => ({ runCommand: vi.fn().mockResolvedValue(undefined), cancelCommand: vi.fn() }));
import { runs, startRun, applyEvent } from './runs';

beforeEach(() => runs.set([]));

describe('runs store', () => {
  it('startRun creates a running run and invokes the backend', async () => {
    const id = startRun('whoami');
    const r = get(runs).find((x) => x.id === id)!;
    expect(r.cmdline).toBe('whoami');
    expect(r.status).toBe('running');
  });
  it('a line event appends a line to the matching run', () => {
    const id = startRun('whoami');
    applyEvent({ id, type: 'line', text: 'shane', stream: 'out', spans: [] });
    expect(get(runs).find((x) => x.id === id)!.lines[0].text).toBe('shane');
  });
  it('a done event finalizes status + findings', () => {
    const id = startRun('whoami');
    applyEvent({ id, type: 'done', code: 0, ms: 12, findings: [{ name: 'x' }] });
    const r = get(runs).find((x) => x.id === id)!;
    expect(r.status).toBe('done');
    expect(r.findings.length).toBe(1);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test src/lib/stores/runs.test.ts` → FAIL.

- [ ] **Step 3: Implement the runs store**

`src/lib/stores/runs.ts`: a `writable<Run[]>`; `startRun(cmdline)` generates an id (`crypto.randomUUID()`), unshifts a `running` run, calls `bridge.runCommand({ id, cmdline })`, returns id; `applyEvent(e: QEvent)` finds the run by `e.id` and either pushes a line (`{text, stream, spans}`) or sets `status:'done', code, ms, findings`. Register the global listener once at module load: `onQEvent(applyEvent)`. Export `runs`, `startRun`, `applyEvent`, and a `cancelRun(id)` calling `bridge.cancelCommand(id)`.

- [ ] **Step 4: Run store test to verify it passes**

Run: `npm test src/lib/stores/runs.test.ts` → PASS.

- [ ] **Step 5: Write the Terminal render test**

`src/lib/components/views/Terminal.test.ts`: mock `$lib/stores/runs` to provide a `runs` writable seeded with one run containing a line whose `spans` mark a substring; render `<Terminal>`; assert the command line text and the flagged `<mark>` (the flagged substring) both render. (Use `@testing-library/svelte`.)

- [ ] **Step 6: Build the components**

`OutputLine.svelte`: props `{ text: string; spans: Span[] }`; compute a segment array (plain vs `<mark class="cat-{cat} sev-{sev}">` for `[s,e)` ranges, sorted, non-overlapping) and render segments through Svelte text bindings — NO `innerHTML`. Style `mark` through tokens (severity → `--crit/--high/--med`).
`Terminal.svelte`: subscribe to `runs`; render each run as a card — a `$ {cmdline}` header, the `OutputLine`s, a status row (spinner while running / `✓ code · {ms}ms` when done), and finding chips. Match the mock's terminal look using tokens.
Modify `Cockpit.svelte` to render `<Terminal />` when the active view is `output` (replace the static placeholder).

- [ ] **Step 7: Run tests + full suite**

Run: `npm test` → all prior + the new store/terminal tests PASS. `npm run check` clean.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat(ui): live Terminal view — run/stream/flag-highlight/findings"
```

---

### Task 5: Command bar + ⌘K launcher

**Files:**
- Create: `src/lib/components/CommandBar.svelte`, `src/lib/components/Launcher.svelte`, `src/lib/components/CommandBar.test.ts`, `src/lib/components/Launcher.test.ts`
- Modify: `src/lib/components/shell/TopBar.svelte` (make the ⌘K field open the launcher), `src/lib/components/shell/Cockpit.svelte` (host the command bar above the center pane + wire Run → `startRun`, switching the active view to `output`)

**Interfaces:**
- Consumes: `flattenTemplates`, `extractVars`, `resolveCmd`, `startRun`.
- Produces: `<Launcher>` (a modal palette filtering `flattenTemplates()` by a query over name/desc/tool/cat, `on:pick` emitting the chosen template) and `<CommandBar>` (an editable command line + a fill-in input per `extractVars(cmd)`; a Run button that resolves and calls `startRun`).

- [ ] **Step 1: Write the failing launcher test**

`src/lib/components/Launcher.test.ts`: render `<Launcher>` with `open=true`; type "kong" into the search; assert the "Kong portal UUID leak" row appears and an unrelated command does not. (Filter is case-insensitive substring over name/desc/tool/cat.)

- [ ] **Step 2: Run it → FAIL.** `npm test src/lib/components/Launcher.test.ts`

- [ ] **Step 3: Implement `Launcher.svelte`**

Svelte 5 runes: `let { open = false } = $props()`; a `$state` query; `const results = $derived(flattenTemplates().filter(t => match(query, t)))` where `match` lowercases and tests `name/desc/tool/cat`. Render results (name + cat + ⚠ later in Task 6); clicking a row dispatches `pick` with the template. Keyboard: `Esc` closes. Style as a centered glass palette via tokens.

- [ ] **Step 4: Run launcher test → PASS.**

- [ ] **Step 5: Write the failing command-bar test**

`src/lib/components/CommandBar.test.ts`: render `<CommandBar>` with a loaded template `{ cmd: 'curl {{url}}' }`; assert a fill input for `url` renders; set it to `https://x`; mock `$lib/stores/runs` `startRun`; click Run; assert `startRun` was called with `curl https://x`.

- [ ] **Step 6: Run it → FAIL.**

- [ ] **Step 7: Implement `CommandBar.svelte`**

`$state` for the current `cmd` string and a `vars` record; `const varNames = $derived(extractVars(cmd))`; render one input per varName bound into `vars`; a live preview of `resolveCmd(cmd, vars)`; Run calls `startRun(resolveCmd(cmd, vars))`. Accept an external `loadTemplate(t)` (sets `cmd`). Style through tokens (the mock's `.cmd` bar).

- [ ] **Step 8: Wire the shell**

`TopBar.svelte`: the ⌘K field/button toggles a `launcherOpen` state (bubble an event or use a shared store). `Cockpit.svelte`: render `<Launcher bind:open>` + `<CommandBar>`; on launcher `pick`, `loadTemplate` into the command bar and close; on Run (inside CommandBar) also set the active center view to `output` so the run is visible.

- [ ] **Step 9: Run full suite + check**

Run: `npm test` (all pass) and `npm run check` (clean).

- [ ] **Step 10: Commit**

```bash
git add -A
git commit -m "feat(ui): command bar + ⌘K launcher over all 215 commands"
```

---

### Task 6: Playbook view + Tool checker

**Files:**
- Create: `src/lib/stores/tools.ts`, `src/lib/components/Playbook.svelte`, `src/lib/components/Tools.svelte`, `src/lib/components/Playbook.test.ts`, `src/lib/stores/tools.test.ts`
- Modify: `src/lib/components/shell/Cockpit.svelte` (open Playbook from the rail/launcher; open Tools modal), `src/lib/components/Launcher.svelte` (show ⚠ when a result's `tool` is missing)

**Interfaces:**
- Consumes: `bridge.toolCheck`, `TEMPLATES`, `flattenTemplates`, `loadTemplate` (Task 5).
- Produces: a `tools` store (`ToolStatus[]` + a `missing(name?)` helper) loaded via `toolCheck()`, `<Playbook>` (categories → items, ⚠ on missing tool, click → loadTemplate), `<Tools>` (the installed/missing list with a Recheck button).

- [ ] **Step 1: Write the failing tools-store test**

`src/lib/stores/tools.test.ts`: mock `$lib/bridge` `toolCheck` → `[{name:'subfinder',found:true,hint:''},{name:'nuclei',found:false,hint:''}]`; call `loadTools()`; assert the store has 2 entries and `isMissing('nuclei')` is true, `isMissing('subfinder')` is false.

- [ ] **Step 2: Run it → FAIL.** `npm test src/lib/stores/tools.test.ts`

- [ ] **Step 3: Implement `tools.ts`**

`writable<ToolStatus[]>`; `loadTools()` awaits `toolCheck()` and sets it; `isMissing(name)` reads the current value (a tool is "missing" if present in the list with `found:false`; unknown tools are treated as present so non-tool templates never show ⚠). Keep a module-level snapshot via `subscribe` (the config-store pattern from 1a).

- [ ] **Step 4: Run tools test → PASS.**

- [ ] **Step 5: Write the failing Playbook test**

`src/lib/components/Playbook.test.ts`: mock `$lib/stores/tools` so `isMissing('amass')` is true; render `<Playbook>`; assert a category header (e.g. "Quickfire") renders, a known item renders, and an item whose `tool` is `amass` shows the ⚠ marker.

- [ ] **Step 6: Run it → FAIL.**

- [ ] **Step 7: Implement `Playbook.svelte` + `Tools.svelte`**

`Playbook.svelte`: iterate `TEMPLATES`; per category a header, per item name/desc/cmd; if `item.tool && isMissing(item.tool)` show `⚠ no {tool}`; clicking an item dispatches `pick`/`loadTemplate`. Optional filter input over the flattened list. `Tools.svelte`: list the `tools` store with ✓ installed / ✗ missing + a Recheck button calling `loadTools()`. Also add the ⚠ marker to `Launcher.svelte` results (same `isMissing` check).
Wire into `Cockpit.svelte`: the rail's Playbook/Tools affordances open these (as a drawer + modal, matching the current app's `📑 Playbook` / `🧰 Tools`). Call `loadTools()` on mount.

- [ ] **Step 8: Run full suite + check + live**

Run: `npm test` (all pass), `npm run check` (clean), then `npm run tauri dev` (free port 1420 first) — verify: open ⌘K → search → pick a no-tool command like "crt.sh" (fill `{{domain}}` = a domain you own or `example.com`) → Run → output streams into the Terminal view with any flags highlighted; open Playbook and Tools. Stop the process after. Note the deep visual/functional pass is the human's.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "feat(ui): Playbook browser + tool checker with missing-tool flags"
```

---

## Self-Review

- **Spec coverage (1b slice):** 215 commands ported (T1 ✓), template fill/resolve (T2 ✓), streaming contract (T3 ✓), live Terminal with flags + findings (T4 ✓), ⌘K launcher + command bar (T5 ✓), Playbook + tool checker (T6 ✓). Findings→report, Loot, Deck panel, Settings are Phase **1c**; Surface Map / Activity Feed real data are **1d/2** — correctly out of scope here.
- **Placeholder scan:** the two "verify the real count / verify the Span field names" notes (T1 Step 2, T3 Step 1) are deliberate reconcile-against-source instructions with concrete fallbacks, not code placeholders. No banned patterns; every code step carries real code.
- **Type consistency:** `Span`/`QEvent` defined in T3 and consumed unchanged by T4's store + `OutputLine`; `flattenTemplates`/`extractVars`/`resolveCmd` names are stable across T1/T2/T5/T6; `startRun` defined in T4 and called in T5; `isMissing` defined in T6 and used by both Playbook and Launcher.

## Execution Handoff

See the options presented after this plan is saved.
