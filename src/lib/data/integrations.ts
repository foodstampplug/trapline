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

export type IntegrationKind = 'shodanHost' | 'shodanDomain' | 'shodanSearch' | 'leak';

export interface IntegrationEntry {
  kind: IntegrationKind;
  name: string;
  desc: string;
  cat: string;
  /** Label shown next to the single arg input (e.g. "ip", "domain", "email"). */
  argLabel: string;
  argPlaceholder: string;
  /** For kind==='leak': the LeakCheck v2 search type sent as `?type=`. */
  leakType?: string;
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
  // Every LeakCheck v2 search type (docs.leakcheck.io/pro-api/search-types).
  // All route through one generic `leakcheck_query(value, kind)` command and
  // render the same LeakResult table; `leakType` is the `?type=` value.
  // phash/origin/password are Enterprise-only.
  {
    kind: 'leak',
    leakType: 'auto',
    name: 'LeakCheck: auto',
    desc: 'auto-detect email / username / phone / hash',
    cat: 'Integrations',
    argLabel: 'query',
    argPlaceholder: 'user@example.com',
  },
  {
    kind: 'leak',
    leakType: 'email',
    name: 'LeakCheck: email',
    desc: 'breaches for a single email address',
    cat: 'Integrations',
    argLabel: 'email',
    argPlaceholder: 'user@example.com',
  },
  {
    kind: 'leak',
    leakType: 'domain',
    name: 'LeakCheck: domain',
    desc: 'every breached address at a domain',
    cat: 'Integrations',
    argLabel: 'domain',
    argPlaceholder: 'example.com',
  },
  {
    kind: 'leak',
    leakType: 'username',
    name: 'LeakCheck: username',
    desc: 'breaches tied to a username',
    cat: 'Integrations',
    argLabel: 'username',
    argPlaceholder: 'neo',
  },
  {
    kind: 'leak',
    leakType: 'phone',
    name: 'LeakCheck: phone',
    desc: 'breaches tied to a phone number',
    cat: 'Integrations',
    argLabel: 'phone',
    argPlaceholder: '15551234567',
  },
  {
    kind: 'leak',
    leakType: 'keyword',
    name: 'LeakCheck: keyword',
    desc: 'free-text keyword search across breaches',
    cat: 'Integrations',
    argLabel: 'keyword',
    argPlaceholder: 'acme',
  },
  {
    kind: 'leak',
    leakType: 'hash',
    name: 'LeakCheck: hash',
    desc: 'SHA256 hash of a lower-cased email',
    cat: 'Integrations',
    argLabel: 'sha256',
    argPlaceholder: 'a1b2c3…',
  },
  {
    kind: 'leak',
    leakType: 'phash',
    name: 'LeakCheck: password hash (Enterprise)',
    desc: 'SHA256 hash of a password',
    cat: 'Integrations',
    argLabel: 'sha256',
    argPlaceholder: 'a1b2c3…',
  },
  {
    kind: 'leak',
    leakType: 'origin',
    name: 'LeakCheck: origin (Enterprise)',
    desc: 'info-stealer logs by the site the creds belong to',
    cat: 'Integrations',
    argLabel: 'site',
    argPlaceholder: 'example.com',
  },
  {
    kind: 'leak',
    leakType: 'password',
    name: 'LeakCheck: password (Enterprise)',
    desc: 'accounts found using a given plaintext password',
    cat: 'Integrations',
    argLabel: 'password',
    argPlaceholder: 'hunter2',
  },
];

// Every LeakCheck type shares one LeakResult shape, so the card kind is
// coarser than IntegrationKind (which is already just 'leak' for them).
export type CardKind = 'shodanHost' | 'shodanDomain' | 'shodanSearch' | 'leak';

export type IntegrationCardData =
  | { kind: 'shodanHost'; data: ShodanHost }
  | { kind: 'shodanDomain'; data: ShodanDomain }
  | { kind: 'shodanSearch'; data: ShodanSearch }
  | { kind: 'leak'; data: LeakResult };
