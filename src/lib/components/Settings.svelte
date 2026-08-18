<script lang="ts">
  // Settings modal — the ⚙ rail panel for user-editable config: the
  // original 4 webhook/shell fields, plus (Phase 2) the Watch section —
  // targets, interval/threshold/rpm, and enable + run-once controls.
  // Webhook fields ported field-for-field from main:index.html #settings /
  // main.js openSettings()/saveSettings()/testWebhook(). Save sends the
  // webhook fields plus the Watch config fields (targets/interval/
  // threshold/rpm) — NOT watchEnabled, which the enable toggle drives
  // directly via startWatch()/stopWatch() ($lib/stores/watch) instead of
  // the form; those persist "enabled" themselves server-side.
  // saveConfig (src/lib/stores/config.ts) merges this patch client-side
  // over the current config before calling the backend, preserving every
  // other field (deck_*, watchEnabled) already in state (the backend's
  // set_config also preserves them as a backstop). Same modal-card shell as
  // FindingsPanel.svelte/Loot.svelte/FindingEditor.svelte.
  import { untrack } from 'svelte';
  import { config, saveConfig } from '$lib/stores/config';
  import { testWebhook } from '$lib/bridge';
  import { toast } from '$lib/stores/toasts';
  import { watch, startWatch, stopWatch, runWatchOnce } from '$lib/stores/watch';
  import type { WatchTarget } from '$lib/types';

  let { open = $bindable(false) }: { open?: boolean } = $props();

  let webhookUrl = $state('');
  let username = $state('');
  let communityDiscord = $state('');
  let shell = $state('');

  // Integrations section — Shodan / LeakCheck API keys. Masked by default
  // (type="password"); one shared reveal toggle flips both, kept simple per
  // the brief ("per-field or one shared — your call"). Rides the SAME
  // saveConfig() call as the webhook/Watch fields below — no separate save
  // path — and is never logged or echoed to a toast.
  let shodanApiKey = $state('');
  let leakcheckApiKey = $state('');
  let revealKeys = $state(false);

  // Watch section local editing state. `watchEnabled` is deliberately NOT
  // mirrored here — the enable toggle below reads `$watch.running` directly
  // and drives startWatch()/stopWatch(), which persist "enabled" themselves
  // server-side (preserve_deck_fields carries it across saves). Everything
  // else here round-trips through the SAME saveConfig() call as the webhook
  // fields — no separate save path.
  let targets = $state<WatchTarget[]>([]);
  let watchIntervalMin = $state(30);
  let watchAlertThreshold = $state(50);
  let watchMaxRpm = $state(30);
  let watchBusy = $state(false);
  let runningOnce = $state(false);

  /** One entry per line, for textarea display. */
  function toLines(arr: string[]): string {
    return arr.join('\n');
  }
  /** Inverse of toLines — trims and drops blank lines. */
  function fromLines(s: string): string[] {
    return s
      .split('\n')
      .map((t) => t.trim())
      .filter(Boolean);
  }
  function cloneTargets(ts: WatchTarget[]): WatchTarget[] {
    return ts.map((t) => ({ name: t.name, pages: [...t.pages], js: [...t.js], inScope: [...t.inScope], autoEnrich: t.autoEnrich }));
  }
  /** Guards against NaN from a momentarily-cleared number input at save time. */
  function numOr(v: number, fallback: number): number {
    return Number.isFinite(v) ? v : fallback;
  }

  // Settings is mounted once for the whole session (like Loot/FindingsPanel)
  // and only toggled via `open`, so it can't seed its local $state from a
  // one-time prop snapshot the way FindingEditor does — the config store may
  // still hold its EMPTY default the first time this component's script
  // runs (loadConfig() in +page.svelte is async). Instead, mirror the
  // vanilla app's openSettings(), which re-reads the config every time the
  // modal opens: re-seed whenever `open` flips true. The $config read is
  // untracked so this effect only reacts to `open` changing, not to every
  // subsequent store update while the panel is open (which would clobber
  // in-progress edits).
  $effect(() => {
    if (!open) return;
    untrack(() => {
      const c = $config;
      webhookUrl = c.webhookUrl;
      username = c.username;
      communityDiscord = c.communityDiscord;
      shell = c.shell;
      shodanApiKey = c.shodanApiKey ?? '';
      leakcheckApiKey = c.leakcheckApiKey ?? '';
      targets = cloneTargets(c.watchTargets ?? []);
      watchIntervalMin = Math.max(1, Math.round((c.watchIntervalSecs ?? 1800) / 60));
      watchAlertThreshold = c.watchAlertThreshold ?? 50;
      watchMaxRpm = c.watchMaxRpm ?? 30;
    });
  });

  const SHELLS: { value: string; label: string }[] = [
    { value: '', label: 'Default (PowerShell on Windows)' },
    { value: 'powershell', label: 'PowerShell' },
    { value: 'cmd', label: 'cmd.exe' },
    { value: 'bash', label: 'bash' },
    { value: 'sh', label: 'sh' },
  ];

  let saved = $state(false);
  let testing = $state(false);
  let testResult = $state<'ok' | 'err' | null>(null);
  let savedTimer: ReturnType<typeof setTimeout> | undefined;
  let testTimer: ReturnType<typeof setTimeout> | undefined;

  const canTest = $derived(webhookUrl.trim().length > 0);

  function close(): void {
    open = false;
  }

  // Deliberately does not close the panel on save (the vanilla app's
  // saveSettings() did, via a toast + closeSettings()) — there's no toast
  // system here, so staying open with an inline "Saved ✓" is the minimal
  // equivalent feedback, and lets Send test still be used right after
  // saving a new webhook without reopening the panel.
  async function save(): Promise<void> {
    try {
      await saveConfig({
        webhookUrl: webhookUrl.trim(),
        username: username.trim(),
        communityDiscord: communityDiscord.trim(),
        shell,
        shodanApiKey: shodanApiKey.trim(),
        leakcheckApiKey: leakcheckApiKey.trim(),
        // watchEnabled intentionally omitted — see comment above the
        // Watch-section state block. saveConfig merges this patch over the
        // current config, so leaving it out preserves whatever the
        // scheduler last persisted there.
        watchTargets: targets.map((t) => ({
          name: t.name.trim(),
          pages: t.pages,
          js: t.js,
          inScope: t.inScope,
          autoEnrich: t.autoEnrich,
        })),
        watchIntervalSecs: Math.max(1, Math.round(numOr(watchIntervalMin, 30))) * 60,
        watchAlertThreshold: Math.min(100, Math.max(0, Math.round(numOr(watchAlertThreshold, 50)))),
        watchMaxRpm: Math.max(0, Math.round(numOr(watchMaxRpm, 0))),
      });
      saved = true;
      clearTimeout(savedTimer);
      savedTimer = setTimeout(() => (saved = false), 1500);
      toast('Settings saved', 'ok');
    } catch (e) {
      console.error(e);
    }
  }

  // Drives the scheduler directly — does NOT write config.watchEnabled from
  // the form. startWatch()/stopWatch() (src/lib/stores/watch.ts) persist
  // "enabled" themselves and refresh the `watch` store, which this toggle
  // reflects via `$watch.running`.
  async function onToggleWatch(e: Event): Promise<void> {
    const turnOn = (e.currentTarget as HTMLInputElement).checked;
    watchBusy = true;
    try {
      if (turnOn) {
        await startWatch();
      } else {
        await stopWatch();
      }
    } catch (e2) {
      console.error(e2);
      toast('Watch toggle failed: ' + String(e2), 'err');
    } finally {
      watchBusy = false;
    }
  }

  async function onRunOnce(): Promise<void> {
    if (runningOnce) return;
    runningOnce = true;
    try {
      await runWatchOnce();
      toast('Watch cycle started', 'ok');
    } catch (e) {
      console.error(e);
      toast('Run once failed: ' + String(e), 'err');
    } finally {
      runningOnce = false;
    }
  }

  function addTarget(): void {
    targets = [...targets, { name: '', pages: [], js: [], inScope: [], autoEnrich: false }];
  }
  function removeTarget(i: number): void {
    targets = targets.filter((_, idx) => idx !== i);
  }

  async function sendTest(): Promise<void> {
    const url = webhookUrl.trim();
    if (!url || testing) return;
    testing = true;
    testResult = null;
    try {
      await testWebhook(url);
      testResult = 'ok';
      toast('Webhook test sent', 'ok');
    } catch (e) {
      console.error(e);
      testResult = 'err';
      toast('Webhook test failed', 'err');
    } finally {
      testing = false;
    }
    clearTimeout(testTimer);
    testTimer = setTimeout(() => (testResult = null), 2000);
  }

  function onWindowKeydown(e: KeyboardEvent): void {
    if (open && e.key === 'Escape') {
      e.preventDefault();
      close();
    }
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

{#if open}
  <div class="overlay">
    <button type="button" class="backdrop" aria-label="Close settings" onclick={close}></button>
    <div class="modal-card" role="dialog" aria-modal="true" aria-label="Settings">
      <div class="modal-head">
        <h2>⚙ Settings</h2>
        <button type="button" class="icon-btn sm" title="Close" onclick={close}>✕</button>
      </div>

      <div class="ff-col">
        <label for="setWebhook"
          >Discord webhook URL <span class="lbl-note">(where screenshots / loot go)</span></label
        >
        <input
          id="setWebhook"
          type="text"
          spellcheck="false"
          placeholder="https://discord.com/api/webhooks/…"
          bind:value={webhookUrl}
        />
      </div>

      <div class="ff-col">
        <label for="setUsername">Webhook display name</label>
        <input id="setUsername" type="text" spellcheck="false" placeholder="Trapline" bind:value={username} />
      </div>

      <div class="ff-col">
        <label for="setCommunity"
          >Community Discord invite <span class="lbl-note">(the "Join the Discord" link)</span></label
        >
        <input
          id="setCommunity"
          type="text"
          spellcheck="false"
          placeholder="https://discord.gg/…"
          bind:value={communityDiscord}
        />
      </div>

      <div class="ff-col">
        <label for="setShell">Shell</label>
        <select id="setShell" bind:value={shell}>
          {#each SHELLS as s (s.value)}
            <option value={s.value}>{s.label}</option>
          {/each}
        </select>
      </div>

      <div class="sec-head">Integrations <span class="lbl-note">Shodan / LeakCheck API keys, used by enrichment</span></div>

      <div class="ff-col">
        <label for="setShodanKey">Shodan API key</label>
        <input
          id="setShodanKey"
          type={revealKeys ? 'text' : 'password'}
          spellcheck="false"
          autocomplete="off"
          placeholder="Shodan API key"
          bind:value={shodanApiKey}
        />
      </div>

      <div class="ff-col">
        <label for="setLeakcheckKey">LeakCheck API key</label>
        <input
          id="setLeakcheckKey"
          type={revealKeys ? 'text' : 'password'}
          spellcheck="false"
          autocomplete="off"
          placeholder="LeakCheck API key"
          bind:value={leakcheckApiKey}
        />
      </div>

      <div class="ff-col">
        <button
          type="button"
          class="ghost-btn sm"
          title={revealKeys ? 'Hide keys' : 'Reveal keys'}
          onclick={() => (revealKeys = !revealKeys)}
        >
          {revealKeys ? '🙈 Hide keys' : '👁 Reveal keys'}
        </button>
        <span class="lbl-note key-note">Keys are stored locally and sent only to Shodan / LeakCheck.</span>
      </div>

      <div class="sec-head">Watch <span class="lbl-note">background change-detection scheduler</span></div>

      <div class="ff-col">
        <label class="chk-row" for="watchEnabled">
          <input
            id="watchEnabled"
            type="checkbox"
            role="switch"
            checked={$watch.running}
            disabled={watchBusy}
            onchange={onToggleWatch}
          />
          Enable Watch
          <span class="lbl-note">({$watch.running ? 'running' : 'stopped'})</span>
        </label>
      </div>

      <div class="ff-row">
        <div class="ff-col">
          <label for="watchInterval">Interval <span class="lbl-note">(minutes)</span></label>
          <input id="watchInterval" type="number" min="1" step="1" bind:value={watchIntervalMin} />
        </div>
        <div class="ff-col">
          <label for="watchThreshold">Alert threshold <span class="lbl-note">(0–100)</span></label>
          <input id="watchThreshold" type="number" min="0" max="100" step="1" bind:value={watchAlertThreshold} />
        </div>
        <div class="ff-col">
          <label for="watchRpm">Max req/min <span class="lbl-note">(0 = unthrottled)</span></label>
          <input id="watchRpm" type="number" min="0" step="1" bind:value={watchMaxRpm} />
        </div>
      </div>

      <div class="ff-col">
        <button type="button" class="ghost-btn" onclick={onRunOnce} disabled={runningOnce}>
          {runningOnce ? 'Running…' : 'Run once →'}
        </button>
      </div>

      <div class="group-label">Targets</div>
      {#each targets as t, i (i)}
        <div class="target-card">
          <div class="target-head">
            <input
              type="text"
              class="target-name"
              spellcheck="false"
              placeholder="Target name"
              aria-label={`Target ${i + 1} name`}
              bind:value={t.name}
            />
            <button type="button" class="ghost-btn sm" onclick={() => removeTarget(i)}>Remove</button>
          </div>
          <div class="ff-row">
            <div class="ff-col">
              <label for={`watchPages${i}`}>Pages <span class="lbl-note">(one URL per line)</span></label>
              <textarea
                id={`watchPages${i}`}
                rows="3"
                spellcheck="false"
                value={toLines(t.pages)}
                onchange={(e) => (t.pages = fromLines((e.currentTarget as HTMLTextAreaElement).value))}
              ></textarea>
            </div>
            <div class="ff-col">
              <label for={`watchJs${i}`}>JS files <span class="lbl-note">(one URL per line)</span></label>
              <textarea
                id={`watchJs${i}`}
                rows="3"
                spellcheck="false"
                value={toLines(t.js)}
                onchange={(e) => (t.js = fromLines((e.currentTarget as HTMLTextAreaElement).value))}
              ></textarea>
            </div>
            <div class="ff-col">
              <label for={`watchScope${i}`}>In-scope hosts <span class="lbl-note">(one host per line)</span></label>
              <textarea
                id={`watchScope${i}`}
                rows="3"
                spellcheck="false"
                value={toLines(t.inScope)}
                onchange={(e) => (t.inScope = fromLines((e.currentTarget as HTMLTextAreaElement).value))}
              ></textarea>
            </div>
          </div>
          <label class="chk-row">
            <input type="checkbox" bind:checked={t.autoEnrich} />
            Auto-enrich new findings
          </label>
        </div>
      {/each}
      <div class="ff-col">
        <button type="button" class="ghost-btn" onclick={addTarget}>+ Add target</button>
      </div>

      <div class="modal-actions">
        <button type="button" class="ghost-btn" onclick={sendTest} disabled={!canTest || testing}>
          {testResult === 'ok' ? 'Sent ✓' : testResult === 'err' ? 'Failed ✕' : testing ? 'Sending…' : 'Send test →'}
        </button>
        {#if saved}<span class="saved-note">Saved ✓</span>{/if}
        <span class="spacer"></span>
        <button type="button" class="ghost-btn" onclick={close}>Close</button>
        <button type="button" class="run-btn" onclick={save}>Save</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    background: var(--scrim);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .backdrop {
    position: absolute;
    inset: 0;
    z-index: 0;
    width: 100%;
    height: 100%;
    padding: 0;
    margin: 0;
    border: none;
    background: transparent;
    cursor: default;
  }
  .modal-card {
    position: relative;
    z-index: 1;
    /* Wider than the original 460px webhook-only card — the Watch section's
       3-up target rows (pages/js/inScope) need the room. Same min(92vw, …)
       pattern as FindingEditor.svelte's finding-card (800px there). */
    width: min(92vw, 640px);
    max-height: 90vh;
    overflow-y: auto;
    background: var(--card-glass);
    backdrop-filter: blur(28px);
    border: var(--bordw) solid var(--edge2);
    border-radius: var(--radius);
    padding: 22px;
    box-shadow: var(--shadow);
  }
  .modal-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 14px;
  }
  .modal-head h2 {
    margin: 0;
    font: 700 18px/1 var(--fdisp);
    color: var(--ink);
  }
  .icon-btn {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border-radius: calc(var(--radius) - 8px);
    border: var(--bordw) solid var(--edge);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
    font-size: 13px;
  }
  .icon-btn:hover {
    color: var(--ink);
    border-color: var(--accent);
  }

  .ff-col {
    display: flex;
    flex-direction: column;
    gap: 5px;
    margin-bottom: 14px;
  }
  .ff-col label {
    font: 700 11px/1 var(--fui);
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .lbl-note {
    text-transform: none;
    letter-spacing: 0;
    color: var(--dim);
    font-weight: 400;
  }
  .ff-col input,
  .ff-col select,
  .ff-col textarea {
    width: 100%;
    background: var(--input-bg);
    border: var(--bordw) solid var(--edge2);
    border-radius: calc(var(--radius) - 6px);
    color: var(--ink);
    padding: 9px 11px;
    outline: none;
    font: 500 13px/1.4 var(--fui);
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .ff-col input:focus,
  .ff-col select:focus,
  .ff-col textarea:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--glowa);
  }
  .ff-col select {
    cursor: pointer;
  }
  .ff-col select option {
    background: var(--bg);
  }
  .ff-col textarea {
    resize: vertical;
    line-height: 1.55;
    font-family: var(--fmono);
    font-size: 12px;
  }

  /* Multi-column field rows — same shape as FindingEditor.svelte's .ff-row,
     used here for the Watch section's interval/threshold/rpm and the
     per-target pages/js/inScope textareas. */
  .ff-row {
    display: flex;
    gap: 12px;
    margin-bottom: 12px;
    flex-wrap: wrap;
  }
  .ff-row .ff-col {
    flex: 1;
    min-width: 150px;
  }

  .sec-head {
    font: 700 11px/1 var(--fui);
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin: 4px 0 14px;
    padding-top: 14px;
    border-top: var(--bordw) solid var(--edge);
  }
  .group-label {
    font: 700 11px/1 var(--fui);
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin-bottom: 8px;
  }
  .chk-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font: 500 12.5px/1.4 var(--fui);
    color: var(--ink);
    cursor: pointer;
    text-transform: none;
    letter-spacing: 0;
  }
  .chk-row input[type='checkbox'] {
    width: auto;
    cursor: pointer;
  }
  .target-card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    background: rgba(255, 255, 255, 0.03);
    border: var(--bordw) solid var(--edge);
    border-radius: calc(var(--radius) - 4px);
    padding: 12px;
    margin-bottom: 12px;
  }
  .target-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .target-name {
    flex: 1;
    min-width: 0;
    background: var(--input-bg);
    border: var(--bordw) solid var(--edge2);
    border-radius: calc(var(--radius) - 6px);
    color: var(--ink);
    padding: 9px 11px;
    outline: none;
    font: 600 13px/1.4 var(--fui);
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .target-name:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--glowa);
  }

  .modal-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 18px;
    padding-top: 14px;
    border-top: var(--bordw) solid var(--edge);
  }
  .modal-actions .spacer {
    flex: 1;
  }
  .saved-note {
    font: 600 12px/1 var(--fui);
    color: var(--ok);
  }
  .ghost-btn {
    background: transparent;
    border: var(--bordw) solid var(--edge2);
    color: var(--muted);
    border-radius: calc(var(--radius) - 6px);
    padding: 9px 14px;
    font: 600 13px/1 var(--fui);
    cursor: pointer;
    transition: 0.15s;
  }
  .ghost-btn:hover:not(:disabled) {
    color: var(--ink);
    border-color: var(--accent);
  }
  .ghost-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .ghost-btn.sm {
    padding: 7px 10px;
    font-size: 12px;
    flex-shrink: 0;
  }
  /* Reveal-keys note sits directly under its toggle button, not inline in a
     label like the other .lbl-note usages — same token/color, just given
     its own line. */
  .key-note {
    display: block;
    margin-top: 6px;
  }
  .run-btn {
    font: 700 13px/1 var(--fdisp);
    /* Dark-on-amber text — same hex Loot.svelte/FindingEditor.svelte already
       use for buttons on var(--accent); reused, not a new hex. */
    color: #221a06;
    background: var(--accent);
    border: none;
    border-radius: calc(var(--radius) - 6px);
    padding: 10px 16px;
    cursor: pointer;
  }
  .run-btn:hover {
    filter: brightness(1.09);
  }
  .run-btn:active {
    transform: translateY(1px);
  }
</style>
