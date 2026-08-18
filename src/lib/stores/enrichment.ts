// Enrichment store — Shodan/LeakCheck-derived host intel, keyed by host.
// Fed two ways: passively via the `enrich:host` event the watch scheduler's
// auto-enrich pass emits (src-tauri/src/watch/scheduler.rs), and actively via
// an on-demand shodanHost() lookup from the UI (refreshEnrich). Both paths
// upsert into the same map — the latest write for a given host wins, same
// "latest wins" shape as watch.ts's status store.
import { writable } from 'svelte/store';
import type { EnrichHost, ShodanHost, ShodanService } from '$lib/types';
import { shodanHost } from '$lib/bridge';
import { onEnrichHost } from '$lib/events';

export interface EnrichEntry {
  ports: number[];
  services?: ShodanService[];
  cves: string[];
  org: string;
  lastEnriched: number;
}

export const enrichment = writable<Map<string, EnrichEntry>>(new Map());

/** Upserts by e.host — the passive path, fed from the enrich:host event. */
export function applyEnrichHost(e: EnrichHost): void {
  enrichment.update((m) => {
    const next = new Map(m);
    next.set(e.host, { ports: e.ports, cves: e.cves, org: e.org, lastEnriched: Date.now() });
    return next;
  });
}

/** Upserts by host — the active path, fed from an on-demand shodanHost() result. */
export function applyShodanHost(host: string, s: ShodanHost): void {
  enrichment.update((m) => {
    const next = new Map(m);
    next.set(host, { ports: s.ports, services: s.services, cves: s.cves, org: s.org, lastEnriched: Date.now() });
    return next;
  });
}

/** On-demand refresh: looks a single host up via Shodan and folds it in. */
export async function refreshEnrich(host: string): Promise<void> {
  const result = await shodanHost(host);
  applyShodanHost(host, result);
}

/** Wire the live event listener once (call from the shell's onMount). */
export async function initEnrich(): Promise<void> {
  await onEnrichHost(applyEnrichHost);
}
