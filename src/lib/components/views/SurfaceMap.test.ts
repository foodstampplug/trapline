import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup, fireEvent, within } from '@testing-library/svelte';

// Async factory + dynamic import mirrors the pattern Terminal.test.ts/
// Cockpit.test.ts use for their store mocks — stays valid ESM without
// needing require() in this @types/node-less project. Seeds one scope with
// a flagged host, an unflagged host, and a file-name-shaped pseudo-host
// (`x.config.json`) that the noise filter must drop.
vi.mock('$lib/stores/surface', async () => {
  const { writable } = await import('svelte/store');
  return {
    surface: writable([
      {
        domain: 'acme.com',
        nodes: [
          { host: 'admin.acme.com', flagged: true, severity: 'high' },
          { host: 'api.acme.com', flagged: false },
          { host: 'x.config.json', flagged: false },
        ],
      },
    ]),
  };
});

import SurfaceMap from './SurfaceMap.svelte';

// This project's vite.config.ts doesn't set `test.globals`, so
// @testing-library/svelte's built-in auto-cleanup never registers — every
// file with more than one `it`/`render` cleans up explicitly between tests
// (same pattern as FindingEditor.test.ts/Loot.test.ts/Settings.test.ts).
afterEach(cleanup);

describe('SurfaceMap view', () => {
  it('renders the center domain and host nodes, filtering out file-name-shaped pseudo-hosts', () => {
    render(SurfaceMap);

    expect(screen.getByText('acme.com')).toBeInTheDocument();
    expect(screen.getByText('admin.acme.com')).toBeInTheDocument();
    expect(screen.getByText('api.acme.com')).toBeInTheDocument();
    expect(screen.queryByText('x.config.json')).not.toBeInTheDocument();
  });

  it('opens an inspector with the host and a → Finding control when a node is clicked', async () => {
    render(SurfaceMap);

    await fireEvent.click(screen.getByText('admin.acme.com'));

    const inspector = screen.getByRole('dialog', { name: /node inspector/i });
    expect(within(inspector).getByText('admin.acme.com')).toBeInTheDocument();
    expect(within(inspector).getByText('high')).toBeInTheDocument();
    expect(within(inspector).getByRole('button', { name: /finding/i })).toBeInTheDocument();
  });

  it('calls onCreateFinding with the host when → Finding is clicked', async () => {
    const onCreateFinding = vi.fn();
    render(SurfaceMap, { props: { onCreateFinding } });

    await fireEvent.click(screen.getByText('admin.acme.com'));
    await fireEvent.click(screen.getByRole('button', { name: /finding/i }));

    expect(onCreateFinding).toHaveBeenCalledWith('admin.acme.com');
  });
});
