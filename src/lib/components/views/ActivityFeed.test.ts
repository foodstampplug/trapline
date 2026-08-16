import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/svelte';

// Async factory + dynamic import mirrors SurfaceMap.test.ts's store-mock
// pattern — stays valid ESM without needing require() in this
// @types/node-less project. Seeds one recon event and one finding event.
vi.mock('$lib/stores/activity', async () => {
  const { writable } = await import('svelte/store');
  return {
    activity: writable([
      { kind: 'recon', ts: 1000, title: 'subfinder -d acme.com', sub: '2 flag(s) · done', status: 'done' },
      { kind: 'finding', ts: 2000, title: 'Admin panel', sub: 'admin.acme.com', severity: 'high' },
    ]),
  };
});

import ActivityFeed from './ActivityFeed.svelte';

// This project's vite.config.ts doesn't set `test.globals`, so
// @testing-library/svelte's built-in auto-cleanup never registers — every
// file with more than one `it`/`render` cleans up explicitly between tests
// (same pattern as SurfaceMap.test.ts/FindingEditor.test.ts/Loot.test.ts).
afterEach(cleanup);

describe('ActivityFeed view', () => {
  it('renders both recon and finding events with filter controls', () => {
    render(ActivityFeed);

    expect(screen.getByText('subfinder -d acme.com')).toBeInTheDocument();
    expect(screen.getByText('Admin panel')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'All' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Recon' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Findings' })).toBeInTheDocument();
  });

  it('hides the recon event and keeps the finding event when Findings is clicked', async () => {
    render(ActivityFeed);

    await fireEvent.click(screen.getByRole('button', { name: 'Findings' }));

    expect(screen.queryByText('subfinder -d acme.com')).not.toBeInTheDocument();
    expect(screen.getByText('Admin panel')).toBeInTheDocument();
  });
});
