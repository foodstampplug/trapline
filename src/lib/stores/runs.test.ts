import { describe, it, expect, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';
vi.mock('$lib/bridge', () => ({ runCommand: vi.fn().mockResolvedValue(undefined), cancelCommand: vi.fn() }));
vi.mock('$lib/events', () => ({ onQEvent: vi.fn().mockResolvedValue(() => {}) }));
import { runs, startRun, applyEvent } from './runs';

beforeEach(() => runs.set([]));

describe('runs store', () => {
  it('startRun creates a running run and invokes the backend', async () => {
    const id = startRun('whoami');
    const r = get(runs).find((x) => x.id === id)!;
    expect(r.cmdline).toBe('whoami');
    expect(r.status).toBe('running');
  });
  it('a line event appends a line to the matching run', () => {
    const id = startRun('whoami');
    applyEvent({ id, type: 'line', text: 'shane', stream: 'out', spans: [] });
    expect(get(runs).find((x) => x.id === id)!.lines[0].text).toBe('shane');
  });
  it('a done event finalizes status + findings', () => {
    const id = startRun('whoami');
    applyEvent({ id, type: 'done', code: 0, ms: 12, findings: [{ name: 'x' }] });
    const r = get(runs).find((x) => x.id === id)!;
    expect(r.status).toBe('done');
    expect(r.findings.length).toBe(1);
  });
});
