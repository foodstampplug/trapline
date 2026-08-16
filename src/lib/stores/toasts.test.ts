import { describe, it, expect, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { toasts, toast, dismiss } from './toasts';

beforeEach(() => {
  for (const t of get(toasts)) dismiss(t.id);
});

describe('toasts', () => {
  it('toast() pushes a message that can be dismissed', () => {
    toast('Copied', 'ok');
    const list = get(toasts);
    expect(list.at(-1)!.message).toBe('Copied');
    expect(list.at(-1)!.kind).toBe('ok');
    dismiss(list.at(-1)!.id);
    expect(get(toasts).find((t) => t.message === 'Copied')).toBeUndefined();
  });
});
