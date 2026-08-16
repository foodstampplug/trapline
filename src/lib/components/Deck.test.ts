import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/svelte';

// Async factory + dynamic import — same pattern Settings.test.ts/Loot.test.ts
// use, so this stays valid ESM in a project with no @types/node.
vi.mock('$lib/bridge', () => ({
  deckStatus: vi.fn().mockResolvedValue({ running: false }),
  deckStart: vi.fn().mockResolvedValue({
    running: true,
    lanUrl: 'http://192.168.1.5:8787',
    token: 'abc',
    qrSvg: '<svg></svg>',
    url: 'http://localhost:8787',
    port: 8787,
  }),
  deckStop: vi.fn(),
  deckSetFolder: vi.fn(),
  openUrl: vi.fn(),
}));

import Deck from './Deck.svelte';
import { deckStart, deckStatus, openUrl } from '$lib/bridge';

// This project's vite.config.ts doesn't set `test.globals`, so
// @testing-library/svelte's built-in auto-cleanup never registers — every
// file with more than one `it`/`render` cleans up explicitly between tests
// (same reason as Settings.test.ts/Loot.test.ts).
afterEach(cleanup);

describe('Deck', () => {
  it('loads status when opened, starts the deck, and shows the LAN URL', async () => {
    render(Deck, { props: { open: true } });

    // Panel opened → status is fetched to reflect current (stopped) state.
    expect(deckStatus).toHaveBeenCalled();

    const startBtn = await screen.findByRole('button', { name: /start deck/i });
    await fireEvent.click(startBtn);

    expect(deckStart).toHaveBeenCalledTimes(1);
    expect(await screen.findByText(/192\.168\.1\.5/)).toBeInTheDocument();

    // "Open on this PC" must route through the Tauri bridge (tauri-plugin-opener),
    // not a plain <a target="_blank"> — the app's CSP has no navigation allowlist,
    // so a bare anchor would no-op or navigate the webview away from the app.
    const openBtn = await screen.findByRole('button', { name: /open on this pc/i });
    await fireEvent.click(openBtn);

    expect(openUrl).toHaveBeenCalledWith('http://localhost:8787');
  });
});
