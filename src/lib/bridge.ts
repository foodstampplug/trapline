// Typed command bridge over every #[tauri::command] registered in
// src-tauri/src/lib.rs. Components must never call `invoke` directly — they
// go through these thin wrappers so the argument shapes stay centrally
// aligned with the Rust signatures.
import { invoke } from '@tauri-apps/api/core';
import type {
  Config,
  ToolStatus,
  RunArgs,
  DeckStatus,
  Finding,
  Session,
  SendCardArgs,
  SendLootArgs,
  WatchStatus,
  ShodanHost,
  ShodanDomain,
  ShodanSearch,
  LeakResult,
} from './types';

// ── Run / cancel commands (src-tauri/src/commands.rs:92-122) ────────────────
export const runCommand = (a: RunArgs) => invoke<void>('run_command', a);
export const cancelCommand = (id: string) => invoke<void>('cancel_command', { id });

// ── Tool check (src-tauri/src/commands.rs:55-65) ─────────────────────────────
export const toolCheck = () => invoke<ToolStatus[]>('tool_check');

// ── Send card / loot to Discord (src-tauri/src/commands.rs:152-223) ─────────
// send_card takes (id, title) — it looks up the command's captured output
// server-side, it does not take title/desc/filename/data/color from the caller.
export const sendCard = (a: SendCardArgs) => invoke<void>('send_card', a);
// send_loot takes a single markdown string.
export const sendLoot = (a: SendLootArgs) => invoke<void>('send_loot', { markdown: a.markdown });

// ── Config (src-tauri/src/commands.rs:227-257) ───────────────────────────────
export const getConfig = () => invoke<Config>('get_config');
export const setConfig = (config: Config) => invoke<void>('set_config', { config });
export const testWebhook = (url: string) => invoke<void>('test_webhook', { url });

// ── URL opener (src-tauri/src/commands.rs:261-266) ───────────────────────────
export const openUrl = (url: string) => invoke<void>('open_url', { url });

// ── Session (src-tauri/src/commands.rs:270-283, src-tauri/src/session.rs) ───
// save_session/load_session move an opaque JSON string across the bridge
// (`data: String`), so the bridge stringifies/parses on either side.
export const saveSession = (session: Session) =>
  invoke<void>('save_session', { data: JSON.stringify(session) });
export const loadSession = async (): Promise<Session> =>
  JSON.parse(await invoke<string>('load_session')) as Session;
export const clearSession = () => invoke<void>('clear_session');

// ── Findings (src-tauri/src/commands.rs:287-300, src-tauri/src/findings.rs) ─
// Same pattern as session: save_finding/load_findings carry a JSON string.
export const saveFinding = (finding: Finding) =>
  invoke<void>('save_finding', { data: JSON.stringify(finding) });
export const loadFindings = async (): Promise<Finding[]> =>
  JSON.parse(await invoke<string>('load_findings')) as Finding[];
export const deleteFinding = (id: string) => invoke<void>('delete_finding', { id });

// ── Deck (src-tauri/src/deck.rs:63-228) ──────────────────────────────────────
export const deckStart = () => invoke<DeckStatus>('deck_start');
export const deckStop = () => invoke<void>('deck_stop');
export const deckStatus = () => invoke<DeckStatus>('deck_status');
export const deckSetFolder = (path: string) => invoke<void>('deck_set_folder', { path });

// ── Watch (src-tauri/src/watch/scheduler.rs) ────────────────────────────────
export const watchStart = () => invoke<void>('watch_start');
export const watchStop = () => invoke<void>('watch_stop');
export const watchStatus = () => invoke<WatchStatus>('watch_status');
export const watchRunOnce = () => invoke<void>('watch_run_once');

// ── Integrations: Shodan + LeakCheck (src-tauri/src/integrations/commands.rs) ─
export const shodanHost = (ip: string) => invoke<ShodanHost>('shodan_host', { ip });
export const shodanDomain = (domain: string) => invoke<ShodanDomain>('shodan_domain', { domain });
export const shodanSearch = (query: string) => invoke<ShodanSearch>('shodan_search', { query });
export const leakcheckDomain = (domain: string) => invoke<LeakResult>('leakcheck_domain', { domain });
export const leakcheckEmail = (email: string) => invoke<LeakResult>('leakcheck_email', { email });
// Generic LeakCheck lookup — backs every ⌘K LeakCheck command. `kind` is the v2
// search type (auto/email/domain/username/phone/keyword/hash/phash/origin/password).
export const leakcheckQuery = (value: string, kind: string) =>
  invoke<LeakResult>('leakcheck_query', { value, kind });
// Generic breach lookup across every provider (leakcheck/snusbase/dehashed/
// leakradar). `kind` is the provider's search type.
export const breachQuery = (provider: string, value: string, kind: string) =>
  invoke<LeakResult>('breach_query', { provider, value, kind });
// Export a breach result to a report file (md/html/csv/json/txt) → returns path.
export const exportBreach = (provider: string, value: string, resultJson: string, format: string) =>
  invoke<string>('export_breach', { provider, value, resultJson, format });
// Add a breach result to the bug-bounty findings (report generator) → finding id.
export const breachToFinding = (provider: string, value: string, resultJson: string) =>
  invoke<string>('breach_to_finding', { provider, value, resultJson });
