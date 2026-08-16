// Live-run store for the Terminal view. Owns the running/completed command
// list the UI renders — Terminal.svelte only ever reads this store, it never
// talks to the bridge or event stream directly.
import { writable } from 'svelte/store';
import type { QEvent, Span } from '$lib/events';
import { onQEvent } from '$lib/events';
import { runCommand, cancelCommand } from '$lib/bridge';

export interface RunLine {
  text: string;
  stream: 'out' | 'err';
  spans: Span[];
}

export interface Run {
  id: string;
  cmdline: string;
  lines: RunLine[];
  status: 'running' | 'done';
  code?: number;
  ms?: number;
  findings: unknown[];
}

export const runs = writable<Run[]>([]);

/** Starts a new command run: creates a `running` entry, kicks off the
 * backend process, and returns the run id immediately (streaming updates
 * arrive later via `applyEvent`, wired to `onQEvent` below). */
export function startRun(cmdline: string): string {
  const id = crypto.randomUUID();
  const run: Run = { id, cmdline, lines: [], status: 'running', findings: [] };
  runs.update((rs) => [run, ...rs]);
  void runCommand({ id, cmdline });
  return id;
}

/** Applies one streamed `q_event` to the matching run. */
export function applyEvent(e: QEvent): void {
  runs.update((rs) =>
    rs.map((r) => {
      if (r.id !== e.id) return r;
      if (e.type === 'line') {
        const line: RunLine = { text: e.text ?? '', stream: e.stream ?? 'out', spans: e.spans ?? [] };
        return { ...r, lines: [...r.lines, line] };
      }
      // 'done'
      return { ...r, status: 'done', code: e.code, ms: e.ms, findings: e.findings ?? [] };
    })
  );
}

export function cancelRun(id: string): void {
  void cancelCommand(id);
}

// Wire the store to the backend event stream exactly once, at module load.
void onQEvent(applyEvent);
