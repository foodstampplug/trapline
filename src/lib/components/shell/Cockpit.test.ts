import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
// Async factory + dynamic import (rather than require()) so this stays valid
// ESM in a project with no @types/node — same runtime effect as the brief's
// `require('svelte/store')` version.
vi.mock('$lib/stores/config', async () => {
  const { writable } = await import('svelte/store');
  return { config: writable({ webhookUrl: 'wh', username: 'Trapline' }), loadConfig: vi.fn() };
});
// Cockpit now mounts <Terminal>, which imports the real runs store — that
// store calls onQEvent (Tauri's `listen`) at module load, which throws
// outside a real Tauri webview. Mock it out here the same way runs.test.ts
// and Terminal.test.ts do, since this test only cares about the shell chrome.
vi.mock('$lib/stores/runs', async () => {
  const { writable } = await import('svelte/store');
  return { runs: writable([]) };
});
// Cockpit now also calls loadTools() on mount (→ real toolCheck()/invoke,
// which throws outside a real Tauri webview) — mock the tools store the
// same way, so this render-only test never touches the Tauri bridge.
vi.mock('$lib/stores/tools', async () => {
  const { writable } = await import('svelte/store');
  return { tools: writable([]), loadTools: vi.fn(), isMissing: () => false };
});
// Cockpit now also hosts <FindingsPanel> and calls loadFindings() on mount
// (→ real bridge invoke, which throws outside a real Tauri webview) — mock
// the findings store the same way as runs/tools above.
vi.mock('$lib/stores/findings', async () => {
  const { writable } = await import('svelte/store');
  return {
    findings: writable([]),
    loadFindings: vi.fn(),
    saveFinding: vi.fn(),
    deleteFinding: vi.fn(),
    newFinding: () => ({
      id: 'new-1',
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
    }),
  };
});
// Cockpit now also calls initWatch() on mount (→ real Tauri listen()/invoke,
// which throws outside a real Tauri webview) — mock the watch store the same
// way as findings/tools/runs above, mirroring watch.test.ts's bridge/events
// mocks but at the store level since this test only cares about shell chrome.
vi.mock('$lib/stores/watch', async () => {
  const { writable } = await import('svelte/store');
  return {
    watch: writable({ running: false, targets: 0, intervalSecs: 1800, lastRunMs: 0, lastAssets: 0, lastNew: 0 }),
    initWatch: vi.fn(() => Promise.resolve()),
  };
});
import Cockpit from './Cockpit.svelte';

describe('Cockpit shell', () => {
  it('renders the brand and the three center-view tabs', () => {
    render(Cockpit);
    expect(screen.getByText('TRAPLINE')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Terminal/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Surface Map/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Activity/i })).toBeInTheDocument();
  });
});
