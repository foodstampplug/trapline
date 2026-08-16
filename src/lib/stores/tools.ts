// Tool-checker store for the Playbook/Launcher's missing-tool (⚠) markers.
// Follows the same writable + module-level-snapshot pattern as
// `src/lib/stores/config.ts` so `isMissing()` can be read synchronously from
// non-reactive contexts (template item rendering) without a `$` subscription.
import { writable } from 'svelte/store';
import type { ToolStatus } from '$lib/types';
import { toolCheck } from '$lib/bridge';

export const tools = writable<ToolStatus[]>([]);
let current: ToolStatus[] = [];
tools.subscribe((v) => (current = v));

/** Refreshes the tool list from the backend. A failed invoke (e.g. outside a
 * real Tauri webview, or a tool-check panic) must never throw unhandled —
 * swallow it and leave the list empty rather than crash the caller. */
export async function loadTools(): Promise<void> {
  try {
    tools.set(await toolCheck());
  } catch {
    tools.set([]);
  }
}

/** True only when `name` is present in the list with `found: false`. A name
 * absent from the list (non-tool templates, e.g. Google-dork commands with
 * no `tool` field, or a tool name the backend doesn't know about) is treated
 * as present so it never shows a spurious ⚠. */
export function isMissing(name: string): boolean {
  const t = current.find((t) => t.name === name);
  return t ? t.found === false : false;
}
