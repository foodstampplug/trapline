// Loot store — aggregates deduped flag hits (`flags::Finding`, emitted on a
// run's `done` event) into a single session list, exportable as Discord-ready
// markdown. Ported from main:src/main.js ~lines 642-684
// (addLoot/sevRank/renderLoot/lootMarkdown). Follows the same
// writable + module-level-snapshot pattern as `src/lib/stores/config.ts` /
// `tools.ts`, so `lootMarkdown()` can read the current array synchronously.
//
// IMPORTANT: this module must never import from `./runs` — `runs.ts` imports
// `addLoot` from here (runs → loot, one-way) to avoid a circular import.
import { writable } from 'svelte/store';

/** One flag hit as emitted by a run's `done` event (mirrors `flags::Finding`
 * — src-tauri/src/flags.rs:36-41 — cat/sev are the lowercase-rename_all enum
 * values: "secret"|"recon"|"http"|"id" and "critical"|"high"|"medium"|"info"). */
export interface FlagHit {
  cat: string;
  sev: string;
  name: string;
  value: string;
}

/** A flag hit plus the command that produced it — one row in the loot list. */
export interface LootItem extends FlagHit {
  cmd: string;
}

export const loot = writable<LootItem[]>([]);
let current: LootItem[] = [];
loot.subscribe((v) => (current = v));

// Dedup set keyed `sev|name|value` — a duplicate hit across runs (or within
// one run's findings array) is added once. Rebuilt from `current` on
// `clearLoot()` rather than tracked independently, so the store stays the
// single source of truth.
const seen = new Set<string>();

function keyOf(x: FlagHit): string {
  return `${x.sev}|${x.name}|${x.value}`;
}

/** Feeds one run's flag hits into the loot list, deduping against everything
 * already collected this session. `cmd` is the command line that produced
 * them (`run.cmdline`), stamped onto each new row for provenance. */
export function addLoot(findings: FlagHit[], cmd: string): void {
  const fresh: LootItem[] = [];
  for (const x of findings ?? []) {
    const key = keyOf(x);
    if (seen.has(key)) continue;
    seen.add(key);
    fresh.push({ cat: x.cat, sev: x.sev, name: x.name, value: x.value, cmd });
  }
  if (fresh.length) loot.update((l) => [...l, ...fresh]);
}

// main.js SEV_ORDER_ARR (~line 654) — worst-first ordering for both the panel
// groups and the markdown export.
const SEV_ORDER = ['critical', 'high', 'medium', 'info'];

/** Sort rank for a severity string; unknown severities sort last. */
export function sevRank(sev: string): number {
  const i = SEV_ORDER.indexOf(sev);
  return i < 0 ? 99 : i;
}

/** Builds the session markdown export (title + count + severity-grouped
 * list), ready to copy or send to Discord. Empty loot yields `""`. */
export function lootMarkdown(): string {
  if (!current.length) return '';
  let md = `# Trapline session — ${new Date().toISOString()}\n\n**${current.length} findings**  ·  by @foodstampplug\n`;
  const sorted = [...current].sort((a, b) => sevRank(a.sev) - sevRank(b.sev));
  let curSev: string | null = null;
  for (const x of sorted) {
    if (x.sev !== curSev) {
      curSev = x.sev;
      md += `\n## ${x.sev.toUpperCase()}\n`;
    }
    md += `- **${x.name}** (${x.cat}) — \`${x.value}\`  _from_ \`${x.cmd}\`\n`;
  }
  return md;
}

/** Clears the collected loot and the dedup set together, so a hit removed by
 * Clear can be re-collected if it fires again. */
export function clearLoot(): void {
  seen.clear();
  loot.set([]);
}
