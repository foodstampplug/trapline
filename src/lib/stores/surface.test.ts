import { describe, it, expect, vi } from 'vitest';
// surface.ts imports the `runs` store, which wires itself to the Tauri event
// bridge at module load (see runs.ts bottom + runs.test.ts). Mock the same
// two modules here so that import doesn't reach for a real Tauri runtime.
vi.mock('$lib/bridge', () => ({ runCommand: vi.fn().mockResolvedValue(undefined), cancelCommand: vi.fn() }));
vi.mock('$lib/events', () => ({ onQEvent: vi.fn().mockResolvedValue(() => {}) }));
import { extractHosts, registrableDomain, buildSurface } from './surface';

describe('surface derivation', () => {
  it('extracts hostnames from an output line, ignoring non-hosts', () => {
    expect(extractHosts('admin.app.acme.com   [401] Admin — auth')).toEqual(['admin.app.acme.com']);
    expect(extractHosts('https://api.app.acme.com/v2 ok')).toContain('api.app.acme.com');
    expect(extractHosts('just some words, no host here')).toEqual([]);
  });
  it('registrableDomain takes the last two labels', () => {
    expect(registrableDomain('admin.app.acme.com')).toBe('acme.com');
    expect(registrableDomain('acme.com')).toBe('acme.com');
  });
  it('groups discovered hosts under their registrable domain and flags finding-hosts', () => {
    const runs = [{ id:'r1', cmdline:'subfinder -d acme.com', status:'done', lines:[
      { text:'api.acme.com [200]', stream:'out', spans:[] },
      { text:'admin.acme.com [401]', stream:'out', spans:[] },
    ], findings:[] }] as any;
    const findings = [{ id:'f', endpoint:'https://admin.acme.com/x', severity:'high', title:'Admin' }] as any;
    const scopes = buildSurface(runs, findings);
    const acme = scopes.find((s) => s.domain === 'acme.com')!;
    expect(acme.nodes.map((n) => n.host).sort()).toEqual(['admin.acme.com','api.acme.com']);
    const admin = acme.nodes.find((n) => n.host === 'admin.acme.com')!;
    expect(admin.flagged).toBe(true);
    expect(admin.severity).toBe('high');
  });
});
