import { listen } from '@tauri-apps/api/event';

export interface Span { s: number; e: number; cat: string; sev: string; label: string }
export interface QEvent {
  id: string; type: 'line' | 'done';
  text?: string; stream?: 'out' | 'err'; spans?: Span[];
  code?: number; ms?: number; findings?: unknown[];
}

export function onQEvent(handler: (e: QEvent) => void): Promise<() => void> {
  return listen<QEvent>('q_event', (ev) => handler(ev.payload));
}
