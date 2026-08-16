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
