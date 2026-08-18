import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { render, screen, cleanup, fireEvent, waitFor } from '@testing-library/svelte';

// Task 8: Launcher now dispatches 5 integration entries through the bridge
// (T7) + enrichment store (T7) + toast channel instead of runCommand, so
// mounting it now touches all three — mock them the same way
// Settings.test.ts/Deck.test.ts mock $lib/bridge for bridge-calling
// components.
vi.mock('$lib/bridge', () => ({
  shodanHost: vi.fn(),
  shodanDomain: vi.fn(),
  shodanSearch: vi.fn(),
  leakcheckDomain: vi.fn(),
  leakcheckEmail: vi.fn(),
}));
vi.mock('$lib/stores/enrichment', () => ({ applyShodanHost: vi.fn() }));
vi.mock('$lib/stores/toasts', () => ({ toast: vi.fn() }));

import Launcher from './Launcher.svelte';
import { shodanHost, leakcheckEmail } from '$lib/bridge';
import { applyShodanHost } from '$lib/stores/enrichment';
import { toast } from '$lib/stores/toasts';

// This project's vite.config.ts doesn't set `test.globals`, so
// @testing-library/svelte's built-in auto-cleanup never registers — every
// file with more than one `it`/`render` cleans up explicitly between tests
// (same reason as Settings.test.ts/Loot.test.ts).
afterEach(cleanup);
beforeEach(() => vi.clearAllMocks());

describe('Launcher', () => {
  it('filters all 215 commands by a case-insensitive substring over name/desc/tool/cat', async () => {
    render(Launcher, { props: { open: true } });

    const search = screen.getByPlaceholderText(/search/i);
    await fireEvent.input(search, { target: { value: 'kong' } });

    expect(screen.getByText('Kong portal UUID leak')).toBeInTheDocument();
    expect(screen.queryByText('subfinder')).not.toBeInTheDocument();
  });
});

describe('Launcher — integration entries', () => {
  it('search surfaces all 5 integration entries by name', async () => {
    render(Launcher, { props: { open: true } });

    const search = screen.getByPlaceholderText(/search/i);
    await fireEvent.input(search, { target: { value: 'shodan' } });

    expect(screen.getByText('Shodan: host lookup')).toBeInTheDocument();
    expect(screen.getByText('Shodan: domain lookup')).toBeInTheDocument();
    expect(screen.getByText('Shodan: search')).toBeInTheDocument();

    await fireEvent.input(search, { target: { value: 'leakcheck' } });
    expect(screen.getByText('LeakCheck: domain breach check')).toBeInTheDocument();
    expect(screen.getByText('LeakCheck: email breach check')).toBeInTheDocument();
  });

  it('picking Shodan host prompts for an ip, calls shodanHost + applyShodanHost, and opens the card', async () => {
    vi.mocked(shodanHost).mockResolvedValue({
      ip: '1.2.3.4',
      org: 'Acme Corp',
      hostnames: [],
      ports: [443],
      services: [],
      cves: ['CVE-2021-41773'],
    });

    render(Launcher, { props: { open: true } });

    const search = screen.getByPlaceholderText(/search/i);
    await fireEvent.input(search, { target: { value: 'Shodan: host lookup' } });
    await fireEvent.click(screen.getByText('Shodan: host lookup').closest('button')!);

    // Arg-prompt mode: the search list is gone, replaced by a single ip field.
    expect(screen.queryByText('Shodan: domain lookup')).not.toBeInTheDocument();
    const argInput = screen.getByPlaceholderText('1.2.3.4');
    await fireEvent.input(argInput, { target: { value: '1.2.3.4' } });
    await fireEvent.click(screen.getByRole('button', { name: /run/i }));

    expect(shodanHost).toHaveBeenCalledWith('1.2.3.4');
    await waitFor(() => expect(applyShodanHost).toHaveBeenCalledWith('1.2.3.4', expect.objectContaining({ org: 'Acme Corp' })));
    expect(await screen.findByText('Acme Corp')).toBeInTheDocument();
    expect(await screen.findByText('CVE-2021-41773')).toBeInTheDocument();
  });

  it('picking LeakCheck email prompts for an email (Enter submits) and opens a leak card', async () => {
    vi.mocked(leakcheckEmail).mockResolvedValue({
      found: 2,
      sources: [{ name: 'BreachCo', date: '2020-01-01' }],
      results: [],
    });

    render(Launcher, { props: { open: true } });

    const search = screen.getByPlaceholderText(/search/i);
    await fireEvent.input(search, { target: { value: 'LeakCheck: email' } });
    await fireEvent.click(screen.getByText('LeakCheck: email breach check').closest('button')!);

    const argInput = screen.getByPlaceholderText('user@example.com');
    await fireEvent.input(argInput, { target: { value: 'user@acme.com' } });
    await fireEvent.keyDown(argInput, { key: 'Enter' });

    expect(leakcheckEmail).toHaveBeenCalledWith('user@acme.com');
    expect(await screen.findByText('BreachCo')).toBeInTheDocument();
    // Never a rendered password field — see IntegrationCard.test.ts for the
    // dedicated redacted-summary coverage.
  });

  it('a bridge error (e.g. missing key) surfaces via toast, never rendered as card data, and no key leaks', async () => {
    vi.mocked(shodanHost).mockRejectedValue('Shodan API key not set — add it in Settings');

    render(Launcher, { props: { open: true } });

    const search = screen.getByPlaceholderText(/search/i);
    await fireEvent.input(search, { target: { value: 'Shodan: host lookup' } });
    await fireEvent.click(screen.getByText('Shodan: host lookup').closest('button')!);

    const argInput = screen.getByPlaceholderText('1.2.3.4');
    await fireEvent.input(argInput, { target: { value: '9.9.9.9' } });
    await fireEvent.click(screen.getByRole('button', { name: /run/i }));

    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith('Shodan API key not set — add it in Settings', 'err')
    );
    expect(screen.queryByText(/9\.9\.9\.9/)).not.toBeInTheDocument();
    expect(applyShodanHost).not.toHaveBeenCalled();
  });
});
