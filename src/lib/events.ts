import { listen } from '@tauri-apps/api/event';
import type { WatchStatus, EnrichHost } from './types';

export interface Span { s: number; e: number; cat: string; sev: string; label: string }
export interface QEvent {
  id: string; type: 'line' | 'done';
  text?: string; stream?: 'out' | 'err'; spans?: Span[];
  code?: number; ms?: number; findings?: unknown[];
}

export function onQEvent(handler: (e: QEvent) => void): Promise<() => void> {
  return listen<QEvent>('q_event', (ev) => handler(ev.payload));
}

export function onWatchStatus(handler: (s: WatchStatus) => void): Promise<() => void> {
  return listen<WatchStatus>('watch:status', (ev) => handler(ev.payload));
}

export interface WatchNewFinding { count: number; ts: number }
export function onWatchNewFinding(handler: (e: WatchNewFinding) => void): Promise<() => void> {
  return listen<WatchNewFinding>('watch:new-finding', (ev) => handler(ev.payload));
}

export function onEnrichHost(handler: (e: EnrichHost) => void): Promise<() => void> {
  return listen<EnrichHost>('enrich:host', (ev) => handler(ev.payload));
}
