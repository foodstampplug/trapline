# Phase 1a — SvelteKit Foundation & Command Bridge — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up the new SvelteKit front end inside the Tauri app — it launches in the Tauri window, loads config from the Rust backend through a typed command bridge, and renders the cockpit shell frame in the Obsidian-amber design system.

**Architecture:** A fresh SvelteKit app (TypeScript, `adapter-static` SPA) replaces the vanilla `src/` front end **on this branch only** (`redesign/free-cockpit`); `main` keeps the shipping app until the redesign merges. The Rust backend is untouched. A single typed module (`src/lib/bridge.ts`) wraps all 18 existing Tauri commands so components never call `invoke` directly. Design lives in CSS custom-property tokens applied once.

**Tech Stack:** SvelteKit 2 + Svelte 5, TypeScript, `@sveltejs/adapter-static`, Vite (already present), Tauri v2, Vitest + `@testing-library/svelte` for tests. Windows build per the `tauri-sveltekit-windows` skill.

**Spec:** `docs/superpowers/specs/2026-08-15-free-cockpit-rebuild-design.md`

## Global Constraints

- **No feature regression (spec G5)** — this phase adds no user features; it only re-platforms the shell. Parity work lands in 1b/1c.
- **Same-origin only (spec §6)** — no external fonts/CDNs; bundle font faces as local assets so the strict CSP (`script-src 'self'`) shipped in PR #6 stays valid. `tauri.conf.json` `security.csp` is NOT changed here.
- **Backend is read-only for this phase** — do not modify any file under `src-tauri/`. Only `tauri.conf.json` build fields may change (Task 1).
- **Design tokens (verbatim):** `--bg:#0a0b0e`, `--panel:rgba(255,255,255,.03)`, `--edge:rgba(255,255,255,.08)`, `--ink:#eef1f6`, `--muted:#98a1af`, `--accent:#ffbf47`, `--accent2:#7aa2ff`, `--ok:#4fe3a0`, `--high:#ff9d3a`, `--crit:#ff6b6b`, `--med:#ffd166`. Fonts: UI = system-ui stack; mono = bundled JetBrains Mono.
- **The 18 commands (exact names):** `run_command, cancel_command, tool_check, send_card, send_loot, get_config, set_config, test_webhook, open_url, save_finding, load_findings, delete_finding, save_session, load_session, clear_session, deck_start, deck_stop, deck_status, deck_set_folder`.
- **Commit after every task.** Conventional Commits. Author with the repo-local noreply email already configured.

---

### Task 1: Scaffold SvelteKit + adapter-static, wire Tauri

**Files:**
- Create: `package.json` (replace scripts/deps), `svelte.config.js`, `vite.config.ts`, `tsconfig.json`, `src/app.html`, `src/routes/+layout.ts`, `src/routes/+layout.svelte`, `src/routes/+page.svelte`
- Modify: `src-tauri/tauri.conf.json:6-9` (build block only)
- Remove from build path (leave in git history): old `index.html`, `src/main.js`, `src/style.css`

**Interfaces:**
- Produces: a running SvelteKit SPA served at `http://localhost:1420` in dev, built to `build/` for Tauri.

- [ ] **Step 1: Install SvelteKit + adapter-static**

```bash
npm install -D @sveltejs/kit @sveltejs/adapter-static @sveltejs/vite-plugin-svelte svelte svelte-check typescript vite
```

- [ ] **Step 2: Configure static adapter for SPA (Tauri needs a fixed index)**

`svelte.config.js`:
```js
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
export default {
  preprocess: vitePreprocess(),
  kit: { adapter: adapter({ fallback: 'index.html' }), alias: { $lib: 'src/lib' } },
};
```

`src/routes/+layout.ts`:
```ts
export const ssr = false;      // Tauri is a client-only shell
export const prerender = true;
```

- [ ] **Step 3: Point Tauri at SvelteKit**

`vite.config.ts`:
```ts
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
export default defineConfig({
  plugins: [sveltekit()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
});
```

Modify `src-tauri/tauri.conf.json` build block to:
```json
"build": {
  "frontendDist": "../build",
  "devUrl": "http://localhost:1420",
  "beforeDevCommand": "npm run dev",
  "beforeBuildCommand": "npm run build"
}
```
Set `package.json` scripts: `"dev": "vite dev"`, `"build": "vite build"`, `"tauri": "tauri"`, `"test": "vitest run"`, `"check": "svelte-check"`.

- [ ] **Step 4: Minimal page so the window isn't blank**

`src/routes/+page.svelte`:
```svelte
<h1>Trapline</h1>
<p>cockpit boot ok</p>
```

- [ ] **Step 5: Verify it launches in Tauri**

Run: `npm run tauri dev`
Expected: the Tauri window opens showing "Trapline / cockpit boot ok" (kill any stale process on port 1420 first: `netstat -ano | grep :1420`). Stop after confirming.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(ui): scaffold SvelteKit + adapter-static front end, wire Tauri"
```

---

### Task 2: Vitest + typed command bridge over all 18 commands

**Files:**
- Create: `src/lib/bridge.ts`, `src/lib/types.ts`, `src/lib/bridge.test.ts`, `vitest-setup.ts`
- Modify: `vite.config.ts` (add test config)

**Interfaces:**
- Consumes: `@tauri-apps/api/core`'s `invoke`.
- Produces: typed async functions — `getConfig(): Promise<Config>`, `setConfig(c: Config): Promise<void>`, `runCommand(a: RunArgs): Promise<string>`, `cancelCommand(id: string): Promise<void>`, `toolCheck(): Promise<ToolStatus[]>`, `sendCard(a: SendCardArgs)`, `sendLoot(a: SendLootArgs)`, `testWebhook(url: string)`, `openUrl(url: string)`, `saveFinding(f: Finding)`, `loadFindings(): Promise<Finding[]>`, `deleteFinding(id: string)`, `saveSession(s: Session)`, `loadSession(): Promise<Session>`, `clearSession()`, `deckStart()`, `deckStop()`, `deckStatus(): Promise<DeckStatus>`, `deckSetFolder(path: string)`. Every function is a thin `invoke("<exact_name>", args)`.

- [ ] **Step 1: Install test deps + Tauri API**

```bash
npm install @tauri-apps/api
npm install -D vitest @testing-library/svelte @testing-library/jest-dom jsdom
```
Add to `vite.config.ts`: `test: { environment: 'jsdom', setupFiles: ['./vitest-setup.ts'] }`.
`vitest-setup.ts`: `import '@testing-library/jest-dom/vitest';`

- [ ] **Step 2: Write the failing test (bridge maps names + args)**

`src/lib/bridge.test.ts`:
```ts
import { describe, it, expect, vi, beforeEach } from 'vitest';
const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invoke(...a) }));
import * as bridge from './bridge';

beforeEach(() => invoke.mockReset());

describe('command bridge', () => {
  it('getConfig calls invoke("get_config")', async () => {
    invoke.mockResolvedValue({ webhookUrl: '' });
    await bridge.getConfig();
    expect(invoke).toHaveBeenCalledWith('get_config');
  });
  it('runCommand forwards args to invoke("run_command")', async () => {
    invoke.mockResolvedValue('ok');
    await bridge.runCommand({ id: 'j1', command: 'whoami', shell: '' });
    expect(invoke).toHaveBeenCalledWith('run_command', { id: 'j1', command: 'whoami', shell: '' });
  });
  it('deckSetFolder forwards the path', async () => {
    invoke.mockResolvedValue(undefined);
    await bridge.deckSetFolder('C:/x/trapline-deck');
    expect(invoke).toHaveBeenCalledWith('deck_set_folder', { path: 'C:/x/trapline-deck' });
  });
});
```

- [ ] **Step 3: Run test to verify it fails**

Run: `npm test`
Expected: FAIL — `./bridge` has no exports yet.

- [ ] **Step 4: Implement `types.ts` then `bridge.ts`**

`src/lib/types.ts` — mirror the Rust structs (camelCase JSON). Minimum for this phase:
```ts
export interface Config {
  webhookUrl: string; username: string; shell: string; communityDiscord: string;
  deckPath: string; deckPort: number; deckToken: string;
}
export interface ToolStatus { name: string; found: boolean; hint: string; }
export interface RunArgs { id: string; command: string; shell: string; }
export interface DeckStatus { running: boolean; port?: number; url?: string; lanUrl?: string; token?: string; qrSvg?: string; message?: string; }
// Finding / Session / SendCardArgs / SendLootArgs: mirror findings.rs / session.rs / commands.rs
export type Finding = Record<string, unknown>;
export type Session = Record<string, unknown>;
export type SendCardArgs = { title: string; desc: string; filename: string; data: number[]; color: number };
export type SendLootArgs = { title: string; items: unknown[] };
```

`src/lib/bridge.ts`:
```ts
import { invoke } from '@tauri-apps/api/core';
import type { Config, ToolStatus, RunArgs, DeckStatus, Finding, Session, SendCardArgs, SendLootArgs } from './types';

export const getConfig = () => invoke<Config>('get_config');
export const setConfig = (config: Config) => invoke<void>('set_config', { config });
export const runCommand = (a: RunArgs) => invoke<string>('run_command', a);
export const cancelCommand = (id: string) => invoke<void>('cancel_command', { id });
export const toolCheck = () => invoke<ToolStatus[]>('tool_check');
export const sendCard = (a: SendCardArgs) => invoke<void>('send_card', a);
export const sendLoot = (a: SendLootArgs) => invoke<void>('send_loot', a);
export const testWebhook = (url: string) => invoke<void>('test_webhook', { url });
export const openUrl = (url: string) => invoke<void>('open_url', { url });
export const saveFinding = (finding: Finding) => invoke<void>('save_finding', { finding });
export const loadFindings = () => invoke<Finding[]>('load_findings');
export const deleteFinding = (id: string) => invoke<void>('delete_finding', { id });
export const saveSession = (session: Session) => invoke<void>('save_session', { session });
export const loadSession = () => invoke<Session>('load_session');
export const clearSession = () => invoke<void>('clear_session');
export const deckStart = () => invoke<DeckStatus>('deck_start');
export const deckStop = () => invoke<void>('deck_stop');
export const deckStatus = () => invoke<DeckStatus>('deck_status');
export const deckSetFolder = (path: string) => invoke<void>('deck_set_folder', { path });
```

> NOTE for the implementer: the exact arg-object keys for `send_card`, `send_loot`, `save_finding`, `save_session` must match the Rust `#[tauri::command]` parameter names in `src-tauri/src/commands.rs` / `session.rs` / `findings.rs`. Read those signatures and align the `types.ts` shapes + arg keys before finalizing. The three test cases above (config, run, deck) are the guaranteed-correct anchors.

- [ ] **Step 5: Run test to verify it passes**

Run: `npm test`
Expected: PASS (3 tests).

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(ui): typed command bridge over all 18 Tauri commands + vitest"
```

---

### Task 3: Config store (boot-load via bridge)

**Files:**
- Create: `src/lib/stores/config.ts`, `src/lib/stores/config.test.ts`

**Interfaces:**
- Consumes: `bridge.getConfig`, `bridge.setConfig`.
- Produces: `config` (a Svelte readable store of `Config`), `loadConfig(): Promise<void>`, `saveConfig(patch: Partial<Config>): Promise<void>` (merges, persists, updates the store).

- [ ] **Step 1: Write the failing test**

`src/lib/stores/config.test.ts`:
```ts
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';
vi.mock('$lib/bridge', () => ({
  getConfig: vi.fn().mockResolvedValue({ webhookUrl: 'wh', username: 'Trapline' }),
  setConfig: vi.fn().mockResolvedValue(undefined),
}));
import { config, loadConfig, saveConfig } from './config';
import * as bridge from '$lib/bridge';

beforeEach(() => vi.clearAllMocks());

describe('config store', () => {
  it('loadConfig populates the store from the backend', async () => {
    await loadConfig();
    expect(get(config).webhookUrl).toBe('wh');
  });
  it('saveConfig merges a patch and persists it', async () => {
    await loadConfig();
    await saveConfig({ username: 'Neo' });
    expect(get(config).username).toBe('Neo');
    expect(bridge.setConfig).toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test src/lib/stores/config.test.ts`
Expected: FAIL — `./config` has no exports.

- [ ] **Step 3: Implement the store**

`src/lib/stores/config.ts`:
```ts
import { writable } from 'svelte/store';
import type { Config } from '$lib/types';
import { getConfig, setConfig } from '$lib/bridge';

const EMPTY: Config = { webhookUrl: '', username: 'Trapline', shell: '', communityDiscord: '', deckPath: '', deckPort: 8787, deckToken: '' };
export const config = writable<Config>(EMPTY);
let current = EMPTY;
config.subscribe((v) => (current = v));

export async function loadConfig(): Promise<void> {
  const c = await getConfig();
  config.set({ ...EMPTY, ...c });
}
export async function saveConfig(patch: Partial<Config>): Promise<void> {
  const merged = { ...current, ...patch };
  await setConfig(merged);
  config.set(merged);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test src/lib/stores/config.test.ts`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(ui): config store with boot-load + merge-save"
```

---

### Task 4: Design tokens (Obsidian-amber) + global styles

**Files:**
- Create: `src/lib/styles/tokens.css`, `src/lib/styles/app.css`, `static/fonts/JetBrainsMono.woff2` (bundled)
- Modify: `src/routes/+layout.svelte` (import global styles), `src/app.html` (font preload)

**Interfaces:**
- Produces: global CSS custom properties (the token set from Global Constraints) on `:root`, a bundled mono `@font-face`, and base element styles. No JS.

- [ ] **Step 1: Write the tokens**

`src/lib/styles/tokens.css` — define every token from Global Constraints on `:root` plus derived surfaces (`--panel2`, `--panel3`, `--edge2`, `--dim`, `--radius:14px`, `--bordw:1px`, `--shadow`, `--glowa:rgba(255,191,71,.12)`), and the two font stacks. Bundle JetBrains Mono via `@font-face` pointing at `/fonts/JetBrainsMono.woff2` (local — no CDN, keeps CSP valid).

- [ ] **Step 2: Base app styles + ambient background**

`src/lib/styles/app.css`: `html,body{margin:0}`, `body{background: radial-gradient(...amber...) , radial-gradient(...blue...), var(--bg); color:var(--ink); font-family:var(--fui)}`, box-sizing reset, focus-visible outline in `--accent`, `@media (prefers-reduced-motion)` guard.

- [ ] **Step 3: Import globally**

`src/routes/+layout.svelte`:
```svelte
<script>import '$lib/styles/tokens.css'; import '$lib/styles/app.css';</script>
<slot />
```

- [ ] **Step 4: Verify tokens apply (smoke)**

Add a temporary `<div style="color:var(--accent)">amber</div>` to `+page.svelte`; run `npm run dev`; confirm amber renders (or write a Vitest render test asserting `getComputedStyle` sees the custom property on `:root`). Remove the temp div.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(ui): Obsidian-amber design tokens + bundled mono font + global styles"
```

---

### Task 5: Cockpit shell frame wired to config

**Files:**
- Create: `src/lib/components/shell/Cockpit.svelte`, `TopBar.svelte`, `AppRail.svelte`, `TargetsPanel.svelte`, `RightDock.svelte`, `StatusBar.svelte`, `src/lib/components/shell/Cockpit.test.ts`
- Modify: `src/routes/+page.svelte` (mount `<Cockpit />`, call `loadConfig()` on mount)

**Interfaces:**
- Consumes: `config` store, `loadConfig`.
- Produces: `<Cockpit>` — the full frame (grid rows `58px 1fr 30px`; body columns `52px 166px 1fr 292px`) with a center-pane `<slot>`/placeholder. Later phases fill the center + rail actions.

- [ ] **Step 1: Write the failing render test**

`src/lib/components/shell/Cockpit.test.ts`:
```ts
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
vi.mock('$lib/stores/config', () => {
  const { writable } = require('svelte/store');
  return { config: writable({ webhookUrl: 'wh', username: 'Trapline' }), loadConfig: vi.fn() };
});
import Cockpit from './Cockpit.svelte';

describe('Cockpit shell', () => {
  it('renders the brand and the three center-view tabs', () => {
    render(Cockpit);
    expect(screen.getByText('TRAPLINE')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Terminal/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Surface Map/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Activity/i })).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test src/lib/components/shell/Cockpit.test.ts`
Expected: FAIL — `Cockpit.svelte` missing.

- [ ] **Step 3: Build the shell components**

Compose `Cockpit.svelte` from the sub-components, styling each through tokens, matching the approved mock: `TopBar` (brand ◎ TRAPLINE + ⌘K launcher input placeholder + action buttons), `AppRail` (Recon/Watch/Deck/Findings/Settings icons), `TargetsPanel` (header + tree placeholder), a center `<main>` with the three view tabs (Terminal / Surface Map / Activity) whose active view is a placeholder `<slot name="view">`, `RightDock` (Watch + Findings placeholders), `StatusBar`. Reproduce the layout/CSS from the approved artifact `trapline-cockpit-final.html` (glass panels, hairline borders, amber accent). Ports/CVE/finding content are placeholders until 1b/1c.

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test src/lib/components/shell/Cockpit.test.ts`
Expected: PASS.

- [ ] **Step 5: Mount it and live-verify**

`src/routes/+page.svelte`:
```svelte
<script>import { onMount } from 'svelte'; import Cockpit from '$lib/components/shell/Cockpit.svelte'; import { loadConfig } from '$lib/stores/config'; onMount(loadConfig);</script>
<Cockpit />
```
Run: `npm run tauri dev` → the Obsidian-amber cockpit frame renders in the Tauri window, view tabs switch a placeholder, config loads without error. Stop after confirming.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(ui): cockpit shell frame (top bar, rail, targets, dock, status) wired to config"
```

---

## Self-Review

- **Spec coverage (1a slice):** SvelteKit + adapter-static (T1 ✓), same-origin/CSP-safe fonts (T4 ✓), typed bridge over the exact 18 commands (T2 ✓), config incl. deck fields (T3 ✓), cockpit shell in Obsidian-amber (T5 ✓). Watch/Shodan/LeakCheck/playbook/findings are explicitly out of 1a (land in 1b–1d).
- **Placeholder scan:** the one NOTE in T2 is a deliberate "read the Rust signatures to align arg keys" instruction with the three verified anchors given — not a code placeholder. No banned patterns.
- **Type consistency:** `Config` shape is identical across T2 (`types.ts`) and T3 (store `EMPTY`); bridge function names in T2 match their uses in T3/T5.

## Execution Handoff

See the options below.
