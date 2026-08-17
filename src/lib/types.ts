// Mirrors the Rust structs in src-tauri/src/*.rs (all camelCase JSON via serde
// #[serde(rename_all = "camelCase")], or single-word snake_case field names
// that are already camelCase-identical).

// src-tauri/src/config.rs:8-30 (Config)
export interface Config {
  webhookUrl: string;
  username: string;
  shell: string;
  communityDiscord: string;
  deckPath: string;
  deckPort: number;
  deckToken: string;
  watchTargets: WatchTarget[];
  watchIntervalSecs: number;
  watchAlertThreshold: number;
  watchMaxRpm: number;
  watchEnabled: boolean;
}

// src-tauri/src/config.rs (WatchTarget) — #[serde(rename_all = "camelCase")]
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

// src-tauri/src/commands.rs:23-28 (ToolInfo)
export interface ToolStatus {
  name: string;
  found: boolean;
  hint: string;
}

// src-tauri/src/commands.rs:92-110 — run_command(app, id: String, cmdline: String, state)
// The shell is read server-side from AppState.config, not passed by the caller.
// Index signature is required because this type is passed as `invoke`'s args
// object directly (rather than spread into a fresh literal) — see bridge.ts.
export interface RunArgs {
  id: string;
  cmdline: string;
  [key: string]: unknown;
}

// src-tauri/src/deck.rs:31-41 (DeckStatus) — known-correct per task brief.
export interface DeckStatus {
  running: boolean;
  port?: number;
  url?: string;
  lanUrl?: string;
  token?: string;
  qrSvg?: string;
  message?: string;
}

// src-tauri/src/findings.rs:6-46 (HunterFinding) — #[serde(rename_all = "camelCase")]
export interface Finding {
  id: string;
  programName: string;
  platform: string;
  title: string;
  severity: string;
  status: string;
  endpoint: string;
  summary: string;
  description: string;
  steps: string;
  evidence: string;
  impact: string;
  remediation: string;
  cvss: string;
  cvssScore: string;
  notes: string;
  cmdline: string;
  createdAt: string;
  updatedAt: string;
}

// src-tauri/src/session.rs has no struct — session data is an opaque JSON blob
// persisted/read as a raw string (save(data: &str) / load() -> String). There is
// nothing in Rust to mirror a fixed shape from, so this stays a bag of fields.
export type Session = Record<string, unknown>;

// src-tauri/src/commands.rs:153-184 — send_card(id: String, title: String, state)
// Index signature required for the same reason as RunArgs above.
export interface SendCardArgs {
  id: string;
  title: string;
  [key: string]: unknown;
}

// src-tauri/src/commands.rs:193-223 — send_loot(markdown: String, state)
export interface SendLootArgs {
  markdown: string;
}
