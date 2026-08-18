// Integration entries for the ⌘K launcher (Launcher.svelte) — Shodan/LeakCheck
// bridge calls (src-tauri/src/integrations/commands.rs, wrapped in
// $lib/bridge.ts) that each prompt for a single arg and render an
// IntegrationCard, instead of resolving `{{placeholder}}`s into a shell
// command and streaming through runCommand like a normal Template.
//
// Kept out of templates.ts/TEMPLATES on purpose: TEMPLATES' category count is
// asserted exactly (30) in templates.test.ts, and flattenTemplates() feeds
// CommandBar's `{{var}}`-fill + `runCommand` path, which these entries don't
// use at all.
import type { ShodanHost, ShodanDomain, ShodanSearch, LeakResult } from '$lib/types';

export type IntegrationKind = 'shodanHost' | 'shodanDomain' | 'shodanSearch' | 'leakDomain' | 'leakEmail';

export interface IntegrationEntry {
  kind: IntegrationKind;
  name: string;
  desc: string;
  cat: string;
  /** Label shown next to the single arg input (e.g. "ip", "domain", "email"). */
  argLabel: string;
  argPlaceholder: string;
}

export const INTEGRATIONS: IntegrationEntry[] = [
  {
    kind: 'shodanHost',
    name: 'Shodan: host lookup',
    desc: 'org, ports, services + CVEs for an IP',
    cat: 'Integrations',
    argLabel: 'ip',
    argPlaceholder: '1.2.3.4',
  },
  {
    kind: 'shodanDomain',
    name: 'Shodan: domain lookup',
    desc: 'subdomains + DNS records for a domain',
    cat: 'Integrations',
    argLabel: 'domain',
    argPlaceholder: 'example.com',
  },
  {
    kind: 'shodanSearch',
    name: 'Shodan: search',
    desc: 'raw Shodan search query — total + matches',
    cat: 'Integrations',
    argLabel: 'query',
    argPlaceholder: 'port:8001 kong',
  },
  {
    kind: 'leakDomain',
    name: 'LeakCheck: domain breach check',
    desc: 'found count + sources for every address at a domain',
    cat: 'Integrations',
    argLabel: 'domain',
    argPlaceholder: 'example.com',
  },
  {
    kind: 'leakEmail',
    name: 'LeakCheck: email breach check',
    desc: 'found count + sources for a single email',
    cat: 'Integrations',
    argLabel: 'email',
    argPlaceholder: 'user@example.com',
  },
];

// LeakCheck's two entries (leakDomain/leakEmail) share one LeakResult shape,
// so the card kind is coarser than IntegrationKind.
export type CardKind = 'shodanHost' | 'shodanDomain' | 'shodanSearch' | 'leak';

export type IntegrationCardData =
  | { kind: 'shodanHost'; data: ShodanHost }
  | { kind: 'shodanDomain'; data: ShodanDomain }
  | { kind: 'shodanSearch'; data: ShodanSearch }
  | { kind: 'leak'; data: LeakResult };
