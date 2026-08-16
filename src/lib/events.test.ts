import { describe, it, expect, vi } from 'vitest';
const listen = vi.fn().mockResolvedValue(() => {});
vi.mock('@tauri-apps/api/event', () => ({ listen: (...a: unknown[]) => listen(...a) }));
import { onQEvent } from './events';

describe('onQEvent', () => {
  it('subscribes to "q_event" and forwards the payload to the handler', async () => {
    const seen: unknown[] = [];
    await onQEvent((e) => seen.push(e));
    expect(listen).toHaveBeenCalledWith('q_event', expect.any(Function));
    // simulate a backend emit
    const cb = listen.mock.calls[0][1] as (ev: { payload: unknown }) => void;
    cb({ payload: { id: 'j1', type: 'line', text: 'hi', spans: [] } });
    expect(seen).toEqual([{ id: 'j1', type: 'line', text: 'hi', spans: [] }]);
  });
});
