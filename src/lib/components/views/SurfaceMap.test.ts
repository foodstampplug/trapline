import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { render, screen, cleanup, fireEvent, within, waitFor } from '@testing-library/svelte';

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
          // IP-shaped node, distinct from the two hostnames above — exercises
          // the Enrich action's shodanHost (not shodanDomain) path.
          { host: '1.2.3.4', flagged: false },
        ],
      },
    ]),
  };
});

// Seeded enrichment for `api.acme.com` only (deliberately not admin.acme.com,
// so these new assertions stay independent of the existing severity/flagged
// coverage above) — one port + one CVE, enough to assert both badge kinds.
// `applyShodanHost` is mocked so the IP-path Enrich test below can assert it
// was called without touching the real store.
vi.mock('$lib/stores/enrichment', async () => {
  const { writable } = await import('svelte/store');
  return {
    enrichment: writable(
      new Map([['api.acme.com', { ports: [443], cves: ['CVE-1'], org: 'Acme', lastEnriched: 0 }]])
    ),
    applyShodanHost: vi.fn(),
  };
});

// $lib/bridge wraps @tauri-apps/api/core's invoke(), which throws outside a
// real Tauri webview — mocked the same way Launcher.test.ts/IntegrationCard
// tests mock it, since SurfaceMap now imports shodanHost/shodanDomain for
// the Enrich action.
vi.mock('$lib/bridge', () => ({ shodanHost: vi.fn(), shodanDomain: vi.fn() }));

import SurfaceMap from './SurfaceMap.svelte';
import { applyShodanHost } from '$lib/stores/enrichment';
import { shodanHost, shodanDomain } from '$lib/bridge';

// This project's vite.config.ts doesn't set `test.globals`, so
// @testing-library/svelte's built-in auto-cleanup never registers — every
// file with more than one `it`/`render` cleans up explicitly between tests
// (same pattern as FindingEditor.test.ts/Loot.test.ts/Settings.test.ts).
afterEach(cleanup);

beforeEach(() => {
  vi.clearAllMocks();
});

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

  it('shows a ports badge and a CVE indicator on a node with enrichment data', () => {
    render(SurfaceMap);

    const node = screen.getByText('api.acme.com').closest('button');
    expect(node).not.toBeNull();
    expect(within(node as HTMLElement).getByText('443')).toBeInTheDocument();
    expect(within(node as HTMLElement).getByText(/1 CVE/i)).toBeInTheDocument();
  });

  it('shows no enrichment badges on a node with no enrichment data', () => {
    render(SurfaceMap);

    const node = screen.getByText('admin.acme.com').closest('button');
    expect(node).not.toBeNull();
    expect(within(node as HTMLElement).queryByText('443')).not.toBeInTheDocument();
    expect(within(node as HTMLElement).queryByText(/CVE/i)).not.toBeInTheDocument();
  });

  it('clicking Enrich on a domain-shaped host calls shodanDomain (not shodanHost) and skips applyShodanHost', async () => {
    vi.mocked(shodanDomain).mockResolvedValue({ domain: 'api.acme.com', subdomains: [], records: [] });
    render(SurfaceMap);

    await fireEvent.click(screen.getByText('api.acme.com'));
    const inspector = screen.getByRole('dialog', { name: /node inspector/i });
    await fireEvent.click(within(inspector).getByRole('button', { name: /enrich/i }));

    await waitFor(() => expect(shodanDomain).toHaveBeenCalledWith('api.acme.com'));
    expect(shodanHost).not.toHaveBeenCalled();
    expect(applyShodanHost).not.toHaveBeenCalled();
  });

  it('clicking Enrich on an IP-shaped host calls shodanHost and feeds applyShodanHost', async () => {
    const data = { ip: '1.2.3.4', org: 'Acme', hostnames: [], ports: [22], services: [], cves: [] };
    vi.mocked(shodanHost).mockResolvedValue(data);
    render(SurfaceMap);

    await fireEvent.click(screen.getByText('1.2.3.4'));
    const inspector = screen.getByRole('dialog', { name: /node inspector/i });
    await fireEvent.click(within(inspector).getByRole('button', { name: /enrich/i }));

    await waitFor(() => expect(shodanHost).toHaveBeenCalledWith('1.2.3.4'));
    expect(applyShodanHost).toHaveBeenCalledWith('1.2.3.4', data);
    expect(shodanDomain).not.toHaveBeenCalled();
  });
});
