# Phase 1 Pre-Merge Polish Sweep — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Clear the deferred-minor checklist accumulated across Phases 1a–1d so the redesign is merge-ready: a real toast system for user feedback, a `--panel-glass` design token replacing raw glass literals, rail count badges, and a bundled mono font — plus a handful of one-line cleanups.

**Architecture:** Pure front-end polish on branch `redesign/free-cockpit`. No backend changes, no new features beyond the toast infra. Each task is independently testable and low-risk.

**Tech Stack:** SvelteKit 2 + Svelte 5 runes, TypeScript, Vitest.

**Spec:** `docs/superpowers/specs/2026-08-15-free-cockpit-rebuild-design.md`

## Global Constraints

- **No behavior regression** — Phase 1 is at parity; this sweep only refines. Every task keeps `npm test` green and `npm run check` clean.
- **Backend read-only** — no `src-tauri/` edits.
- **Same-origin only** — any bundled font is a LOCAL asset (no CDN); the strict CSP (`script-src 'self'`) must stay valid.
- **Tokens** — the new `--panel-glass` values must EQUAL the literals they replace (a visual no-op refactor), so no visual regression.
- **Commit after every task.** Conventional Commits; repo-local noreply email.

---

### Task 1: Toast system + wire feedback paths

**Files:**
- Create: `src/lib/stores/toasts.ts`, `src/lib/stores/toasts.test.ts`, `src/lib/components/Toaster.svelte`
- Modify: `src/lib/components/shell/Cockpit.svelte` (mount `<Toaster>` once), plus these components to call `toast(...)` on their existing success/error branches: `src/lib/stores/runs.ts` (startRun failure), `src/lib/components/Loot.svelte` (Copy / Send to Discord), `src/lib/components/Settings.svelte` (Save / Send test), `src/lib/components/FindingEditor.svelte` (Save error/success), `src/lib/components/Deck.svelte` (Start/Stop/Copy errors), `src/lib/components/ReportModal.svelte` (Copy).

**Interfaces:**
- Produces: `toast(message: string, kind?: 'ok' | 'err' | 'info'): void`, a `toasts` readable store of `{ id, message, kind }[]`, and `dismiss(id)`. Toasts auto-dismiss after ~3.5s. `<Toaster>` renders the stack (bottom-right), each dismissible.

- [ ] **Step 1: Write the failing store test**

`src/lib/stores/toasts.test.ts`:
```ts
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { toasts, toast, dismiss } from './toasts';
beforeEach(() => { for (const t of get(toasts)) dismiss(t.id); });
describe('toasts', () => {
  it('toast() pushes a message that can be dismissed', () => {
    toast('Copied', 'ok');
    const list = get(toasts);
    expect(list.at(-1)!.message).toBe('Copied');
    expect(list.at(-1)!.kind).toBe('ok');
    dismiss(list.at(-1)!.id);
    expect(get(toasts).find((t) => t.message === 'Copied')).toBeUndefined();
  });
});
```

- [ ] **Step 2: Run it → FAIL.** `npm test src/lib/stores/toasts.test.ts`

- [ ] **Step 3: Implement `toasts.ts` + `Toaster.svelte`**

`toasts.ts`: a `writable<{id,message,kind}[]>`; `toast(message, kind='info')` pushes `{ id: crypto.randomUUID(), message, kind }` and schedules `dismiss(id)` after 3500ms (`setTimeout`); `dismiss(id)` removes it. `Toaster.svelte`: subscribe to `toasts`, render a fixed bottom-right stack; each toast styled by kind (`ok`→`--ok`, `err`→`--crit`, `info`→`--muted`) via tokens, click to dismiss; respect `prefers-reduced-motion`.

- [ ] **Step 4: Run store test → PASS.**

- [ ] **Step 5: Mount + wire**

`Cockpit.svelte`: `import Toaster` and render `<Toaster />` once (top level). Then add `toast(...)` calls to the EXISTING branches in the listed files — e.g. `Loot.svelte` Copy → `toast('Loot copied', 'ok')`, Send success → `toast('Loot sent to Discord', 'ok')`, Send catch → `toast('Discord: ' + <err>, 'err')`; `Settings.svelte` Save → `toast('Settings saved', 'ok')`, Send test success/fail; `ReportModal`/Deck copy → `toast('Copied', 'ok')`; `FindingEditor` save catch → `toast('Save failed', 'err')`; `runs.ts` startRun — wrap the `void runCommand(...)` with `.catch((e) => toast('Run failed: ' + String(e), 'err'))`. Do NOT change control flow, only add feedback. Keep it minimal and consistent.

- [ ] **Step 6: Run full suite + check**

`npm test` (all pass, incl. the new store test) + `npm run check` clean. (Wiring toasts into `catch` branches doesn't need a test per call site; the store test covers the mechanism.)

- [ ] **Step 7: Commit** `feat(ui): toast system + wire success/error feedback`

---

### Task 2: `--panel-glass` token + one-line cleanups

**Files:**
- Modify: `src/lib/styles/tokens.css` (add tokens), and the components using raw glass literals — `CommandBar, FindingEditor, FindingsPanel, Loot, Launcher, Settings, Deck, Tools, ReportModal, Playbook, views/SurfaceMap, shell/TopBar` (and shell `Cockpit/RightDock/TargetsPanel/AppRail` if they carry the same literals). Also: remove `class="wide"`/`class="modal-card wide"` dead class (`FindingsPanel`, `ReportModal`); fix the inaccurate "server-side" comment in `Settings.svelte`; change the Deck panel `<label for="deckLanUrl/deckToken/deckTunnelCmd">` (which point at `<code>`, not form controls) to plain text/`aria-label`.

**Interfaces:**
- Produces: `--panel-glass` (the translucent panel background used across modals/panels) and `--scrim` (the modal backdrop) in `tokens.css`, set to the SAME values the literals currently use, so replacement is a visual no-op.

- [ ] **Step 1: Add the tokens**

In `tokens.css`, add (using the values already in the code — read one component to confirm the exact alpha): `--panel-glass: rgba(20, 22, 28, 0.82);` (the panel/inspector glass) and `--scrim: rgba(0, 0, 0, 0.6);` (modal backdrop). If the codebase uses a couple of distinct glass alphas, add `--panel-glass-2` for the second — match reality, don't invent.

- [ ] **Step 2: Replace the literals**

Across the listed components, replace the raw `rgba(20,22,28,α)` glass backgrounds with `var(--panel-glass)` (or `--panel-glass-2`) and the `rgba(0,0,0,.6)`-style modal backdrops with `var(--scrim)`. Each replacement must be value-identical (confirm the alpha matches the token). Leave the QR-card `#fff`/`#333` (functional white quiet-zone) and the `#221a06` on-amber text as-is.

- [ ] **Step 3: The one-liners**

Remove the dead `.wide` class from `FindingsPanel.svelte` + `ReportModal.svelte` markup. Fix `Settings.svelte`'s merge comment to say the deck-field merge happens client-side in `saveConfig` (+ the backend also preserves them). Change the three Deck `<label for=…>` to non-`for` text or `aria-label` on the value row.

- [ ] **Step 4: Verify no visual change**

`npm run check` clean; `npm run build` clean. Since tokens equal the old literals, this is a no-op refactor. (Grep to confirm no stray `rgba(20, 22, 28` / `class="wide"` remain.)

- [ ] **Step 5: Commit** `refactor(ui): --panel-glass/--scrim tokens + dead-class/comment/a11y cleanups`

---

### Task 3: Rail count badges

**Files:**
- Modify: `src/lib/components/shell/AppRail.svelte` (badges on 🐛 Findings + 💰 Loot), `src/lib/components/shell/Cockpit.svelte` (pass counts, or AppRail reads the stores)
- Create: `src/lib/components/shell/AppRail.test.ts` (if none exists)

**Interfaces:**
- Consumes: `findings` store (count), `loot` store (count).
- Produces: a small badge on the 🐛 and 💰 rail buttons showing the count when > 0 (hidden at 0).

- [ ] **Step 1: Write the failing test**

`AppRail.test.ts`: mock `$lib/stores/findings` (`findings` writable seeded with 2) + `$lib/stores/loot` (`loot` writable seeded with 3); render `<AppRail>`; assert a badge showing `2` near the Findings button and `3` near Loot; with empty stores, assert no badge.

- [ ] **Step 2: Run it → FAIL.**

- [ ] **Step 3: Implement**

In `AppRail.svelte`, import the `findings` + `loot` stores; on the 🐛 and 💰 buttons render a `.count` badge (`{#if $findings.length}` / `{#if $loot.length}`) styled via tokens (reuse the existing `.dt` dot / a small count pill — match the old app's count badge look). Ensure `Cockpit.test.ts` still passes (AppRail now imports findings/loot stores — those are already mocked in Cockpit.test, or add the mocks if needed).

- [ ] **Step 4: Run test → PASS**, then `npm test` + `npm run check`.

- [ ] **Step 5: Commit** `feat(ui): rail count badges for findings + loot`

---

### Task 4: Bundle JetBrains Mono

**Files:**
- Create: `static/fonts/JetBrainsMono-Regular.woff2` (+ `-Medium.woff2` if easy)
- Modify: `src/lib/styles/tokens.css` (`@font-face` + put "JetBrains Mono" first in `--fmono`)

**Interfaces:**
- Produces: a locally-bundled mono face so the data font renders identically on machines without JetBrains Mono installed. No CDN — keeps the CSP valid.

- [ ] **Step 1: Fetch the font (trusted OFL source)**

Download from the official JetBrains repo (OFL-licensed):
`curl -L -o static/fonts/JetBrainsMono-Regular.woff2 https://github.com/JetBrains/JetBrainsMono/raw/master/fonts/webfonts/JetBrainsMono-Regular.woff2`
(and optionally `-Medium.woff2` the same way). Verify the file is a real woff2 (`file` says WOFF2 / non-trivial size, e.g. >50KB). If the download fails or the environment has no network, STOP and report — do NOT invent a font file; leave the system stack and note it.

- [ ] **Step 2: Add `@font-face` + reference it**

In `tokens.css`: `@font-face { font-family: 'JetBrains Mono'; src: url('/fonts/JetBrainsMono-Regular.woff2') format('woff2'); font-weight: 400; font-display: swap; }` (+ a 500 face if fetched). Confirm `--fmono` already lists `"JetBrains Mono"` first (it does) so no stack change is needed — the bundled face now actually loads.

- [ ] **Step 3: Verify**

`npm run build` clean; the woff2 is under `static/` so it ships same-origin (CSP-safe). Confirm no external URL was introduced. `npm test` still green.

- [ ] **Step 4: Commit** `feat(ui): bundle JetBrains Mono woff2 (local, CSP-safe)`

---

## Self-Review

- **Checklist coverage:** toast system (T1) — resolves the `startRun`/save/send fire-and-forget UX; `--panel-glass`/`--scrim` + dead `.wide` + Settings comment + Deck a11y (T2); rail count badges (T3); bundled mono font (T4). Remaining acceptable non-items: `registrableDomain` co.uk heuristic (documented), ActivityFeed "Today" divider (cosmetic, no day-bucketing) — left as-is, noted.
- **Placeholder scan:** every step has concrete code/commands; the font URL is a specific trusted source with a fail-safe.
- **No behavior regression:** T2 is value-identical (no-op refactor); T1 only adds feedback on existing branches; T3/T4 are additive. All keep tests green.

## Execution Handoff

Subagent-driven (per REQUIRED SUB-SKILL).
