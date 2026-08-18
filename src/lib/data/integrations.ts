// Integration entries for the ⌘K launcher (Launcher.svelte) — Shodan + breach
// (LeakCheck / Snusbase / DeHashed / LeakRadar) bridge calls
// (src-tauri/src/integrations/commands.rs, wrapped in $lib/bridge.ts) that each
// prompt for a single arg and render an IntegrationCard, instead of resolving
// `{{placeholder}}`s into a shell command and streaming through runCommand.
//
// Kept out of templates.ts/TEMPLATES on purpose: TEMPLATES' category count is
// asserted exactly (30) in templates.test.ts, and flattenTemplates() feeds
// CommandBar's `{{var}}`-fill + `runCommand` path, which these entries don't use.
//
// Every breach entry (any provider) has kind==='leak' and routes through the one
// generic `breach_query(provider, value, kind)` command, rendering the same
// LeakResult table. `provider` picks the API + key; `leakType` is its search type.
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
  /** For kind==='leak': the breach provider (leakcheck/snusbase/dehashed/leakradar). */
  provider?: string;
  /** For kind==='leak': the provider's search type. */
  leakType?: string;
}

/** Display names for the breach providers (also the card title). */
export const PROVIDER_LABELS: Record<string, string> = {
  leakcheck: 'LeakCheck',
  snusbase: 'Snusbase',
  dehashed: 'DeHashed',
  leakradar: 'LeakRadar',
};

interface LeakSpec {
  type: string;
  label: string;
  desc: string;
  arg: string;
  ph: string;
}

function leakEntries(provider: string, specs: LeakSpec[]): IntegrationEntry[] {
  const name = PROVIDER_LABELS[provider];
  return specs.map((s) => ({
    kind: 'leak' as const,
    provider,
    leakType: s.type,
    name: `${name}: ${s.label}`,
    desc: s.desc,
    cat: 'Integrations',
    argLabel: s.arg,
    argPlaceholder: s.ph,
  }));
}

const SHODAN: IntegrationEntry[] = [
  { kind: 'shodanHost', name: 'Shodan: host lookup', desc: 'org, ports, services + CVEs for an IP', cat: 'Integrations', argLabel: 'ip', argPlaceholder: '1.2.3.4' },
  { kind: 'shodanDomain', name: 'Shodan: domain lookup', desc: 'subdomains + DNS records for a domain', cat: 'Integrations', argLabel: 'domain', argPlaceholder: 'example.com' },
  { kind: 'shodanSearch', name: 'Shodan: search', desc: 'raw Shodan search query — total + matches', cat: 'Integrations', argLabel: 'query', argPlaceholder: 'port:8001 kong' },
];

// LeakCheck v2 (docs.leakcheck.io/pro-api/search-types). phash/origin/password = Enterprise.
const LEAKCHECK = leakEntries('leakcheck', [
  { type: 'auto', label: 'auto', desc: 'auto-detect email / username / phone / hash', arg: 'query', ph: 'user@example.com' },
  { type: 'email', label: 'email', desc: 'breaches for an email address', arg: 'email', ph: 'user@example.com' },
  { type: 'domain', label: 'domain', desc: 'every breached address at a domain', arg: 'domain', ph: 'example.com' },
  { type: 'username', label: 'username', desc: 'breaches tied to a username', arg: 'username', ph: 'neo' },
  { type: 'phone', label: 'phone', desc: 'breaches tied to a phone number', arg: 'phone', ph: '15551234567' },
  { type: 'keyword', label: 'keyword', desc: 'free-text keyword search', arg: 'keyword', ph: 'acme' },
  { type: 'hash', label: 'hash', desc: 'SHA256 hash of a lower-cased email', arg: 'sha256', ph: 'a1b2c3…' },
  { type: 'phash', label: 'password hash (Enterprise)', desc: 'SHA256 hash of a password', arg: 'sha256', ph: 'a1b2c3…' },
  { type: 'origin', label: 'origin (Enterprise)', desc: 'info-stealer logs by site the creds belong to', arg: 'site', ph: 'example.com' },
  { type: 'password', label: 'password (Enterprise)', desc: 'accounts using a given plaintext password', arg: 'password', ph: 'hunter2' },
]);

// Snusbase (docs.snusbase.com). Cleartext passwords; results grouped by breach table.
const SNUSBASE = leakEntries('snusbase', [
  { type: 'email', label: 'email', desc: 'records for an email address', arg: 'email', ph: 'user@example.com' },
  { type: 'username', label: 'username', desc: 'records for a username', arg: 'username', ph: 'neo' },
  { type: 'password', label: 'password', desc: 'accounts using a plaintext password', arg: 'password', ph: 'hunter2' },
  { type: 'name', label: 'name', desc: 'records by full name', arg: 'name', ph: 'John Doe' },
  { type: '_domain', label: 'domain', desc: 'every record at an email domain', arg: 'domain', ph: 'example.com' },
  { type: 'hash', label: 'hash', desc: 'records by password hash', arg: 'hash', ph: 'a1b2c3…' },
  { type: 'lastip', label: 'last IP', desc: 'records by last-seen IP address', arg: 'ip', ph: '1.2.3.4' },
]);

// DeHashed v2. `raw` passes your own field:value query; the rest build `field:"value"`.
const DEHASHED = leakEntries('dehashed', [
  { type: 'email', label: 'email', desc: 'records for an email address', arg: 'email', ph: 'user@example.com' },
  { type: 'username', label: 'username', desc: 'records for a username', arg: 'username', ph: 'neo' },
  { type: 'password', label: 'password', desc: 'accounts using a plaintext password', arg: 'password', ph: 'hunter2' },
  { type: 'hashed_password', label: 'password hash', desc: 'records by hashed password', arg: 'hash', ph: 'a1b2c3…' },
  { type: 'name', label: 'name', desc: 'records by name', arg: 'name', ph: 'John Doe' },
  { type: 'phone', label: 'phone', desc: 'records by phone number', arg: 'phone', ph: '15551234567' },
  { type: 'domain', label: 'domain', desc: 'records at an email domain', arg: 'domain', ph: 'example.com' },
  { type: 'ip_address', label: 'IP address', desc: 'records by IP address', arg: 'ip', ph: '1.2.3.4' },
  { type: 'raw', label: 'raw query', desc: 'your own DeHashed field:value query', arg: 'query', ph: 'email:a@b.com' },
]);

// LeakRadar (api.leakradar.io) — info-stealer credentials (url = the login site).
const LEAKRADAR = leakEntries('leakradar', [
  { type: 'email', label: 'email', desc: 'stealer-log creds for an email', arg: 'email', ph: 'user@example.com' },
  { type: 'domain', label: 'domain', desc: 'stealer-log creds at a domain', arg: 'domain', ph: 'example.com' },
  { type: 'raw', label: 'raw search', desc: 'free-text stealer-log search', arg: 'query', ph: 'acme' },
]);

export const INTEGRATIONS: IntegrationEntry[] = [
  ...SHODAN,
  ...LEAKCHECK,
  ...SNUSBASE,
  ...DEHASHED,
  ...LEAKRADAR,
];

// Every breach provider shares one LeakResult shape, so the card kind ('leak')
// is coarser than the provider; the card carries `provider` for its title.
export type CardKind = 'shodanHost' | 'shodanDomain' | 'shodanSearch' | 'leak';

export type IntegrationCardData =
  | { kind: 'shodanHost'; data: ShodanHost }
  | { kind: 'shodanDomain'; data: ShodanDomain }
  | { kind: 'shodanSearch'; data: ShodanSearch }
  | { kind: 'leak'; provider: string; data: LeakResult };
