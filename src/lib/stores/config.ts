import { writable } from 'svelte/store';
import type { Config } from '$lib/types';
import { getConfig, setConfig } from '$lib/bridge';

const EMPTY: Config = {
  webhookUrl: '', username: 'Trapline', shell: '', communityDiscord: '', deckPath: '', deckPort: 8787, deckToken: '',
  watchTargets: [], watchIntervalSecs: 1800, watchAlertThreshold: 50, watchMaxRpm: 30, watchEnabled: false,
  shodanApiKey: '', leakcheckApiKey: '', snusbaseApiKey: '', dehashedApiKey: '', leakradarApiKey: '',
};
export const config = writable<Config>(EMPTY);
let current = EMPTY;
config.subscribe((v) => (current = v));

export async function loadConfig(): Promise<void> {
  const c = await getConfig();
  config.set({ ...EMPTY, ...c });
}
export async function saveConfig(patch: Partial<Config>): Promise<void> {
  const merged = { ...current, ...patch };
  await setConfig(merged);
  config.set(merged);
}
