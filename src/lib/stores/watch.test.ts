import { describe, it, expect, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';

vi.mock('$lib/bridge', () => ({
  watchStart: vi.fn(() => Promise.resolve()),
  watchStop: vi.fn(() => Promise.resolve()),
  watchRunOnce: vi.fn(() => Promise.resolve()),
  watchStatus: vi.fn(() =>
    Promise.resolve({ running: true, targets: 3, intervalSecs: 900, lastRunMs: 123, lastAssets: 18, lastNew: 2 }),
  ),
}));
vi.mock('$lib/events', () => ({
  onWatchStatus: vi.fn(() => Promise.resolve(() => {})),
  onWatchNewFinding: vi.fn(() => Promise.resolve(() => {})),
}));
vi.mock('$lib/stores/findings', () => ({ loadFindings: vi.fn(() => Promise.resolve()) }));

import { watch, refreshWatch, startWatch } from './watch';
import * as bridge from '$lib/bridge';

describe('watch store', () => {
  beforeEach(() => vi.clearAllMocks());

  it('refreshWatch pulls status from the bridge into the store', async () => {
    await refreshWatch();
    expect(bridge.watchStatus).toHaveBeenCalled();
    expect(get(watch).intervalSecs).toBe(900);
    expect(get(watch).targets).toBe(3);
  });

  it('startWatch calls the bridge then refreshes', async () => {
    await startWatch();
    expect(bridge.watchStart).toHaveBeenCalled();
    expect(bridge.watchStatus).toHaveBeenCalled();
    expect(get(watch).intervalSecs).toBe(900);
  });
});
