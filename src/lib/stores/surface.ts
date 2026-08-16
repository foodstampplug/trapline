// Surface store — derives a host graph (Surface Map view) from recon run
// output and findings. Pure derivation: no bridge calls, no writable state
// of its own — `surface` is a `derived([runs, findings], …)` store that
// recomputes whenever either upstream store changes.
import { derived } from 'svelte/store';
import { runs as runsStore, type Run } from './runs';
import { findings as findingsStore } from './findings';
import type { Finding } from '$lib/types';

/** One discovered host within a `Scope`. `flagged` is true when the host
 * matches a finding's endpoint (or title) host; `severity` carries that
 * finding's severity when flagged. */
export interface SurfaceNode {
  host: string;
  flagged: boolean;
  severity?: string;
}

/** A registrable domain and every host discovered under it. */
export interface Scope {
  domain: string;
  nodes: SurfaceNode[];
}

// Matches dotted hostnames (e.g. `api.acme.com`) inside arbitrary text,
// including URLs (`https://api.acme.com/v2` — the regex just finds the host
// portion, ignoring the scheme and path). Requires at least one label plus a
// final TLD-like label of 2+ letters, so bare words with no dot never match.
const HOST_RE = /(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z]{2,}/gi;

/** Extracts every distinct hostname mentioned in `text`, lowercased and in
 * first-seen order. Returns `[]` when no host is present. */
export function extractHosts(text: string): string[] {
  const matches = text.match(HOST_RE) ?? [];
  const seen = new Set<string>();
  for (const m of matches) seen.add(m.toLowerCase());
  return [...seen];
}

/** Registrable-domain heuristic: the last two dot-separated labels of a
 * host (`admin.app.acme.com` -> `acme.com`). This is deliberately naive —
 * it does NOT handle multi-label public-suffix TLDs (e.g. `co.uk`, where
 * `foo.co.uk` should collapse to `foo.co.uk` not `co.uk`). Acceptable for
 * v1 grouping; a real public-suffix list can replace this later without
 * changing the `Scope`/`SurfaceNode` shape. */
export function registrableDomain(host: string): string {
  return host.split('.').slice(-2).join('.');
}

/** Builds the flagged-host -> severity map from findings: every hostname
 * found in a finding's `endpoint`, plus any hostname mentioned in its
 * `title` (findings sometimes name the host there instead of/along with
 * the endpoint URL). Later findings win on collision. */
function flaggedHostSeverity(findings: Finding[]): Map<string, string> {
  const out = new Map<string, string>();
  for (const f of findings) {
    const hosts = [...extractHosts(f.endpoint ?? ''), ...extractHosts(f.title ?? '')];
    for (const h of hosts) out.set(h, f.severity);
  }
  return out;
}

/** Derives the Surface Map's scopes from a run list and a finding list:
 * collects every host mentioned anywhere in run output, groups them by
 * registrable domain, and flags nodes that match a finding's host (with
 * that finding's severity carried along). Scopes containing at least one
 * flagged node sort first; within a scope, flagged nodes sort first. */
export function buildSurface(runs: Run[], findings: Finding[]): Scope[] {
  const flagged = flaggedHostSeverity(findings);

  const hosts: string[] = [];
  const seenHosts = new Set<string>();
  for (const run of runs) {
    for (const line of run.lines) {
      for (const h of extractHosts(line.text)) {
        if (seenHosts.has(h)) continue;
        seenHosts.add(h);
        hosts.push(h);
      }
    }
  }

  const byDomain = new Map<string, SurfaceNode[]>();
  for (const host of hosts) {
    const domain = registrableDomain(host);
    const severity = flagged.get(host);
    const node: SurfaceNode = { host, flagged: flagged.has(host), severity };
    const nodes = byDomain.get(domain);
    if (nodes) nodes.push(node);
    else byDomain.set(domain, [node]);
  }

  const scopes: Scope[] = [...byDomain.entries()].map(([domain, nodes]) => ({
    domain,
    nodes: [...nodes].sort((a, b) => Number(b.flagged) - Number(a.flagged)),
  }));

  scopes.sort((a, b) => {
    const aFlagged = a.nodes.some((n) => n.flagged);
    const bFlagged = b.nodes.some((n) => n.flagged);
    return Number(bFlagged) - Number(aFlagged);
  });

  return scopes;
}

export const surface = derived([runsStore, findingsStore], ([$runs, $findings]) =>
  buildSurface($runs, $findings)
);
