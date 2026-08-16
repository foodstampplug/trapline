import { describe, it, expect, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';
vi.mock('$lib/bridge', () => ({
  loadFindings: vi.fn().mockResolvedValue([{ id: 'a', title: 'T', severity: 'high' }]),
  saveFinding: vi.fn().mockResolvedValue(undefined),
  deleteFinding: vi.fn().mockResolvedValue(undefined),
}));
import { findings, loadFindings, saveFinding, deleteFinding, newFinding } from './findings';
import * as bridge from '$lib/bridge';
beforeEach(() => vi.clearAllMocks());
describe('findings store', () => {
  it('loadFindings populates the store from the backend', async () => {
    await loadFindings();
    expect(get(findings)[0].title).toBe('T');
  });
  it('saveFinding persists then reloads', async () => {
    await saveFinding(newFinding());
    expect(bridge.saveFinding).toHaveBeenCalled();
    expect(bridge.loadFindings).toHaveBeenCalled(); // refresh after save
  });
  it('newFinding has a fresh id and empty fields', () => {
    const f = newFinding();
    expect(f.id).toBeTruthy();
    expect(f.title).toBe('');
  });
});
