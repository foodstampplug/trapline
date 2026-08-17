import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/svelte';

// Async factory + dynamic import (rather than require()) so this stays valid
// ESM in a project with no @types/node — same pattern Cockpit.test.ts /
// FindingsPanel.test.ts use for their store mocks.
vi.mock('$lib/stores/config', async () => {
  const { writable } = await import('svelte/store');
  return {
    config: writable({
      webhookUrl: 'wh',
      username: 'Trapline',
      communityDiscord: '',
      shell: '',
      deckPath: '',
      deckPort: 8787,
      deckToken: '',
      watchTargets: [
        { name: 'acme', pages: ['https://acme.com'], js: [], inScope: ['acme.com'], autoEnrich: false },
      ],
      watchIntervalSecs: 1800,
      watchAlertThreshold: 50,
      watchMaxRpm: 30,
      watchEnabled: false,
    }),
    saveConfig: vi.fn(),
  };
});

vi.mock('$lib/bridge', () => ({
  testWebhook: vi.fn(),
}));

// Same isolation pattern as RightDock.test.ts — mock the whole watch store
// module so the component's `startWatch`/`stopWatch`/`runWatchOnce` calls
// are observable without touching the real Tauri bridge or event listeners.
vi.mock('$lib/stores/watch', async () => {
  const { writable } = await import('svelte/store');
  return {
    watch: writable({ running: false, targets: 1, intervalSecs: 1800, lastRunMs: 0, lastAssets: 0, lastNew: 0 }),
    startWatch: vi.fn(),
    stopWatch: vi.fn(),
    runWatchOnce: vi.fn(),
  };
});

import Settings from './Settings.svelte';
import { saveConfig } from '$lib/stores/config';
import { testWebhook } from '$lib/bridge';
import { startWatch } from '$lib/stores/watch';

// This project's vite.config.ts doesn't set `test.globals`, so
// @testing-library/svelte's built-in auto-cleanup never registers — every
// file with more than one `it`/`render` cleans up explicitly between tests
// (same reason as FindingEditor.test.ts/Loot.test.ts).
afterEach(cleanup);

describe('Settings', () => {
  it('seeds fields from config, saves the 4 fields on Save, and sends a test webhook', async () => {
    render(Settings, { props: { open: true } });

    const webhookInput = screen.getByLabelText(/discord webhook url/i) as HTMLInputElement;
    expect(webhookInput.value).toBe('wh');

    await fireEvent.input(webhookInput, {
      target: { value: 'https://discord.com/api/webhooks/123/newtoken' },
    });

    const saveBtn = screen.getByRole('button', { name: /^save$/i });
    await fireEvent.click(saveBtn);

    expect(saveConfig).toHaveBeenCalledTimes(1);
    const patch = vi.mocked(saveConfig).mock.calls[0][0];
    expect(patch).toMatchObject({
      webhookUrl: 'https://discord.com/api/webhooks/123/newtoken',
      username: 'Trapline',
      communityDiscord: '',
      shell: '',
    });

    const testBtn = screen.getByRole('button', { name: /send test/i });
    await fireEvent.click(testBtn);

    expect(testWebhook).toHaveBeenCalledWith('https://discord.com/api/webhooks/123/newtoken');
  });

  it('disables Send test when the webhook field is empty', async () => {
    render(Settings, { props: { open: true } });
    const webhookInput = screen.getByLabelText(/discord webhook url/i) as HTMLInputElement;
    await fireEvent.input(webhookInput, { target: { value: '' } });

    const testBtn = screen.getByRole('button', { name: /send test/i }) as HTMLButtonElement;
    expect(testBtn.disabled).toBe(true);
  });
});

describe('Settings — Watch section', () => {
  it('renders the Watch section with interval + a seeded target editable', () => {
    render(Settings, { props: { open: true } });

    // 1800s from the seeded config, shown in MINUTES per the Global
    // Constraints (interval is edited in minutes, not raw seconds).
    const intervalInput = screen.getByLabelText(/interval/i) as HTMLInputElement;
    expect(intervalInput.value).toBe('30');

    const targetName = screen.getByDisplayValue('acme') as HTMLInputElement;
    expect(targetName).toBeTruthy();
  });

  it('a seeded target name field is editable', async () => {
    render(Settings, { props: { open: true } });
    const targetName = screen.getByDisplayValue('acme') as HTMLInputElement;
    await fireEvent.input(targetName, { target: { value: 'acme-corp' } });
    expect(targetName.value).toBe('acme-corp');
  });

  it('the enable toggle calls startWatch when turned on', async () => {
    render(Settings, { props: { open: true } });
    const toggle = screen.getByLabelText(/enable watch/i) as HTMLInputElement;
    expect(toggle.checked).toBe(false);

    await fireEvent.click(toggle);

    expect(startWatch).toHaveBeenCalledTimes(1);
  });
});
