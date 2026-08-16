import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
// Async factory + dynamic import (rather than require()) so this stays valid
// ESM in a project with no @types/node — same runtime effect as the brief's
// `require('svelte/store')` version.
vi.mock('$lib/stores/config', async () => {
  const { writable } = await import('svelte/store');
  return { config: writable({ webhookUrl: 'wh', username: 'Trapline' }), loadConfig: vi.fn() };
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
