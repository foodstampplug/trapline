import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { loot, addLoot, lootMarkdown, clearLoot } from './loot';

beforeEach(() => clearLoot());

describe('loot store', () => {
  it('dedupes identical flag hits across runs', () => {
    addLoot([{ cat: 'secret', sev: 'high', name: 'AWS key', value: 'AKIA…' }], 'cmd1');
    addLoot([{ cat: 'secret', sev: 'high', name: 'AWS key', value: 'AKIA…' }], 'cmd2');
    expect(get(loot).length).toBe(1);
  });

  it('lootMarkdown lists collected findings', () => {
    addLoot([{ cat: 'recon', sev: 'medium', name: 'admin', value: 'admin.x.com' }], 'c');
    expect(lootMarkdown()).toContain('admin.x.com');
  });

  it('clearLoot empties the store and resets dedup', () => {
    addLoot([{ cat: 'secret', sev: 'high', name: 'AWS key', value: 'AKIA…' }], 'cmd1');
    clearLoot();
    expect(get(loot).length).toBe(0);
    expect(lootMarkdown()).toBe('');
    addLoot([{ cat: 'secret', sev: 'high', name: 'AWS key', value: 'AKIA…' }], 'cmd2');
    expect(get(loot).length).toBe(1);
  });
});
