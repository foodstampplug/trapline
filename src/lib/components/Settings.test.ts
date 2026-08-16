import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/svelte';

// Async factory + dynamic import (rather than require()) so this stays valid
// ESM in a project with no @types/node — same pattern Cockpit.test.ts /
// FindingsPanel.test.ts use for their store mocks.
vi.mock('$lib/stores/config', async () => {
  const { writable } = await import('svelte/store');
  return {
    config: writable({ webhookUrl: 'wh', username: 'Trapline', communityDiscord: '', shell: '' }),
    saveConfig: vi.fn(),
  };
});

vi.mock('$lib/bridge', () => ({
  testWebhook: vi.fn(),
}));

import Settings from './Settings.svelte';
import { saveConfig } from '$lib/stores/config';
import { testWebhook } from '$lib/bridge';

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
