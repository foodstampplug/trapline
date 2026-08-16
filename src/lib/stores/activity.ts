// Activity feed store — merges recon runs and findings into a single
// time-sorted event list (newest first) for the center-pane Activity view.
// Pure derivation, same shape as surface.ts: no bridge calls, no writable
// state of its own — `activity` is a `derived([runs, findings], …)` store
// that recomputes whenever either upstream store changes.
import { derived } from 'svelte/store';
import { runs as runsStore, type Run } from './runs';
import { findings as findingsStore } from './findings';
import type { Finding } from '$lib/types';

/** One entry in the merged activity timeline. `kind` distinguishes a
 * completed/running recon run from a saved finding; `sub` carries a short
 * secondary line (flag count + status for runs, endpoint/program for
 * findings). `severity` and `status` are only meaningful for their
 * respective `kind`. */
export interface ActivityEvent {
  kind: 'recon' | 'finding';
  ts: number;
  title: string;
  sub?: string;
  severity?: string;
  status?: string;
}

/** Merges `runs` and `findings` into a single newest-first timeline.
 * Each run becomes a `'recon'` event timestamped at `startedAt` (older runs
 * predating this field default to `0`, sorting last); each finding becomes
 * a `'finding'` event timestamped at `Date.parse(createdAt)` (invalid/empty
 * dates also default to `0`). */
export function buildActivity(runs: Run[], findings: Finding[]): ActivityEvent[] {
  const runEvents: ActivityEvent[] = runs.map((run) => ({
    kind: 'recon',
    ts: run.startedAt ?? 0,
    title: run.cmdline,
    sub: `${run.findings.length} flag(s) · ${run.status}`,
    status: run.status,
  }));

  const findingEvents: ActivityEvent[] = findings.map((finding) => ({
    kind: 'finding',
    ts: Date.parse(finding.createdAt) || 0,
    title: finding.title || '(untitled)',
    sub: finding.endpoint || finding.programName,
    severity: finding.severity,
  }));

  return [...runEvents, ...findingEvents].sort((a, b) => b.ts - a.ts);
}

export const activity = derived([runsStore, findingsStore], ([$runs, $findings]) =>
  buildActivity($runs, $findings)
);
