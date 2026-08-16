import { describe, it, expect, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';
vi.mock('$lib/bridge', () => ({
  getConfig: vi.fn().mockResolvedValue({ webhookUrl: 'wh', username: 'Trapline' }),
  setConfig: vi.fn().mockResolvedValue(undefined),
}));
import { config, loadConfig, saveConfig } from './config';
import * as bridge from '$lib/bridge';

beforeEach(() => vi.clearAllMocks());

describe('config store', () => {
  it('loadConfig populates the store from the backend', async () => {
    await loadConfig();
    expect(get(config).webhookUrl).toBe('wh');
  });
  it('saveConfig merges a patch and persists it', async () => {
    await loadConfig();
    await saveConfig({ username: 'Neo' });
    expect(get(config).username).toBe('Neo');
    expect(bridge.setConfig).toHaveBeenCalled();
  });
});
