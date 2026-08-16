import { describe, it, expect, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';
vi.mock('$lib/bridge', () => ({
  toolCheck: vi.fn().mockResolvedValue([
    { name: 'subfinder', found: true, hint: '' },
    { name: 'nuclei', found: false, hint: '' },
  ]),
}));
import { tools, loadTools, isMissing } from './tools';
import * as bridge from '$lib/bridge';

beforeEach(() => vi.clearAllMocks());

describe('tools store', () => {
  it('loadTools populates the store from toolCheck()', async () => {
    await loadTools();
    expect(get(tools)).toHaveLength(2);
    expect(bridge.toolCheck).toHaveBeenCalled();
  });

  it('isMissing is true only for a listed tool with found:false', async () => {
    await loadTools();
    expect(isMissing('nuclei')).toBe(true);
    expect(isMissing('subfinder')).toBe(false);
  });

  it('isMissing treats an unknown tool name as present', async () => {
    await loadTools();
    expect(isMissing('curl')).toBe(false);
  });

  it('a failed toolCheck() never throws and leaves the list empty', async () => {
    vi.mocked(bridge.toolCheck).mockRejectedValueOnce(new Error('no tauri'));
    await expect(loadTools()).resolves.toBeUndefined();
    expect(get(tools)).toEqual([]);
  });
});
