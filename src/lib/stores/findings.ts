// Findings store — the single source of truth for the finding tracker views
// (list/editor/report/loot). Mirrors the config/runs store pattern: a thin
// writable wrapping the bridge, with save/delete always refreshing from the
// backend rather than optimistically mutating local state.
import { writable } from 'svelte/store';
import type { Finding } from '$lib/types';
import { loadFindings as bridgeLoadFindings, saveFinding as bridgeSaveFinding, deleteFinding as bridgeDeleteFinding } from '$lib/bridge';

export const findings = writable<Finding[]>([]);

export async function loadFindings(): Promise<void> {
  const all = await bridgeLoadFindings();
  findings.set(all);
}

export async function saveFinding(f: Finding): Promise<void> {
  await bridgeSaveFinding(f);
  await loadFindings();
}

export async function deleteFinding(id: string): Promise<void> {
  await bridgeDeleteFinding(id);
  await loadFindings();
}

/** A blank finding ready for the editor: fresh id, sane defaults, everything else empty. */
export function newFinding(): Finding {
  return {
    id: crypto.randomUUID(),
    programName: '',
    platform: '',
    title: '',
    severity: 'medium',
    status: 'draft',
    endpoint: '',
    summary: '',
    description: '',
    steps: '',
    evidence: '',
    impact: '',
    remediation: '',
    cvss: '',
    cvssScore: '',
    notes: '',
    cmdline: '',
    createdAt: '',
    updatedAt: '',
  };
}
