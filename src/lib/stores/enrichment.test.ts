import { describe, it, expect, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';

// Isolation: importing the store must never trigger a real Tauri call at
// module load — mock both $lib/bridge (the on-demand shodanHost() path) and
// $lib/events (the passive enrich:host listener), same pattern as
// watch.test.ts mocks $lib/bridge + $lib/events.
vi.mock('$lib/bridge', () => ({ shodanDomain: vi.fn(), shodanHost: vi.fn() }));
vi.mock('$lib/events', () => ({ onEnrichHost: vi.fn(() => Promise.resolve(() => {})) }));

import { enrichment, applyEnrichHost, applyShodanHost, initEnrich, refreshEnrich } from './enrichment';
import * as bridge from '$lib/bridge';
import * as events from '$lib/events';

beforeEach(() => {
  vi.clearAllMocks();
  enrichment.set(new Map());
});

describe('enrichment store', () => {
  it('applyEnrichHost inserts a host entry retrievable by host', () => {
    applyEnrichHost({ target: 'acme', host: 'a.acme.com', ports: [443], cves: ['CVE-1'], org: 'Acme' });
    const entry = get(enrichment).get('a.acme.com');
    expect(entry?.ports).toEqual([443]);
    expect(entry?.cves).toEqual(['CVE-1']);
    expect(entry?.org).toBe('Acme');
    expect(typeof entry?.lastEnriched).toBe('number');
  });

  it('applyEnrichHost merging the same host keeps latest (last write wins)', () => {
    applyEnrichHost({ target: 'acme', host: 'a.acme.com', ports: [443], cves: [], org: 'Acme' });
    applyEnrichHost({ target: 'acme', host: 'a.acme.com', ports: [80, 443], cves: ['CVE-2'], org: 'Acme Inc' });
    const map = get(enrichment);
    expect(map.size).toBe(1);
    const entry = map.get('a.acme.com');
    expect(entry?.ports).toEqual([80, 443]);
    expect(entry?.cves).toEqual(['CVE-2']);
    expect(entry?.org).toBe('Acme Inc');
  });

  it('applyEnrichHost keeps distinct hosts as separate entries', () => {
    applyEnrichHost({ target: 'acme', host: 'a.acme.com', ports: [443], cves: [], org: 'Acme' });
    applyEnrichHost({ target: 'acme', host: 'b.acme.com', ports: [22], cves: [], org: 'Acme' });
    expect(get(enrichment).size).toBe(2);
  });

  it('applyShodanHost upserts by host from a ShodanHost result', () => {
    applyShodanHost('a.acme.com', {
      ip: '1.2.3.4',
      org: 'Acme',
      hostnames: ['a.acme.com'],
      ports: [22, 443],
      services: [{ port: 443, product: 'nginx', version: '1.20' }],
      cves: ['CVE-3'],
    });
    const entry = get(enrichment).get('a.acme.com');
    expect(entry?.ports).toEqual([22, 443]);
    expect(entry?.services?.[0]).toEqual({ port: 443, product: 'nginx', version: '1.20' });
    expect(entry?.org).toBe('Acme');
  });

  it('refreshEnrich pulls a host via the bridge and feeds the store', async () => {
    vi.mocked(bridge.shodanHost).mockResolvedValue({
      ip: '1.2.3.4',
      org: 'Acme',
      hostnames: [],
      ports: [443],
      services: [],
      cves: [],
    });
    await refreshEnrich('a.acme.com');
    expect(bridge.shodanHost).toHaveBeenCalledWith('a.acme.com');
    expect(get(enrichment).get('a.acme.com')?.org).toBe('Acme');
  });

  it('initEnrich wires the enrich:host event listener', async () => {
    await initEnrich();
    expect(events.onEnrichHost).toHaveBeenCalledTimes(1);
    expect(events.onEnrichHost).toHaveBeenCalledWith(applyEnrichHost);
  });
});
