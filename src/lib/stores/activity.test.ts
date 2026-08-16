import { describe, it, expect, vi } from 'vitest';
// activity.ts imports the `runs` store, which wires itself to the Tauri
// event bridge at module load (see runs.ts bottom + runs.test.ts). Mock the
// same two modules here so that import doesn't reach for a real Tauri
// runtime (same pattern as surface.test.ts).
vi.mock('$lib/bridge', () => ({ runCommand: vi.fn().mockResolvedValue(undefined), cancelCommand: vi.fn() }));
vi.mock('$lib/events', () => ({ onQEvent: vi.fn().mockResolvedValue(() => {}) }));
import { buildActivity } from './activity';

describe('activity feed', () => {
  it('merges runs + findings newest-first', () => {
    const runs = [{ id: 'r', cmdline: 'subfinder -d acme.com', status: 'done', startedAt: 1000, findings: [{}, {}] }] as any;
    const findings = [{ id: 'f', title: 'Admin panel', severity: 'high', createdAt: new Date(2000).toISOString() }] as any;
    const ev = buildActivity(runs, findings);
    expect(ev[0].kind).toBe('finding'); // ts 2000 newer than 1000
    expect(ev[1].kind).toBe('recon');
    expect(ev[0].severity).toBe('high');
    expect(ev[1].sub).toContain('2'); // flag/finding count summary
  });
});
