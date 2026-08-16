// Toast/notification store — the app's only user-facing feedback channel for
// fire-and-forget async calls (Discord sends, config saves, run kickoffs,
// clipboard copies) whose success/error would otherwise be silently swallowed
// (console.error only). Deliberately minimal: an in-memory list + a 3.5s
// auto-dismiss timer, rendered by Toaster.svelte.
import { writable } from 'svelte/store';

export interface Toast {
  id: string;
  message: string;
  kind: 'ok' | 'err' | 'info';
}

export const toasts = writable<Toast[]>([]);

const AUTO_DISMISS_MS = 3500;

/** Pushes a toast and schedules its auto-dismiss. */
export function toast(message: string, kind: Toast['kind'] = 'info'): void {
  const id = crypto.randomUUID();
  toasts.update((ts) => [...ts, { id, message, kind }]);
  setTimeout(() => dismiss(id), AUTO_DISMISS_MS);
}

/** Removes a toast by id — called by the auto-dismiss timer or a click. */
export function dismiss(id: string): void {
  toasts.update((ts) => ts.filter((t) => t.id !== id));
}
