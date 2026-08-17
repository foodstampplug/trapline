// Watch store — live status of the built-in change-detection scheduler.
// Fed by watch:status / watch:new-finding events plus an initial watchStatus()
// pull. A new finding triggers a findings reload so every findings surface
// (RightDock, FindingsPanel, Activity feed) updates.
import { writable } from 'svelte/store';
import type { WatchStatus } from '$lib/types';
import { watchStart, watchStop, watchStatus, watchRunOnce } from '$lib/bridge';
import { onWatchStatus, onWatchNewFinding } from '$lib/events';
import { loadFindings } from '$lib/stores/findings';

const EMPTY: WatchStatus = {
  running: false, targets: 0, intervalSecs: 1800, lastRunMs: 0, lastAssets: 0, lastNew: 0,
};

export const watch = writable<WatchStatus>(EMPTY);

export async function refreshWatch(): Promise<void> {
  try {
    watch.set(await watchStatus());
  } catch {
    /* backend not ready — keep last known */
  }
}

export async function startWatch(): Promise<void> {
  await watchStart();
  await refreshWatch();
}

export async function stopWatch(): Promise<void> {
  await watchStop();
  await refreshWatch();
}

export async function runWatchOnce(): Promise<void> {
  await watchRunOnce();
}

/** Wire the live event listeners once (call from the shell's onMount). */
export async function initWatch(): Promise<void> {
  await onWatchStatus((s) => watch.set(s));
  await onWatchNewFinding(() => {
    void refreshWatch();
    void loadFindings();
  });
  await refreshWatch();
}
