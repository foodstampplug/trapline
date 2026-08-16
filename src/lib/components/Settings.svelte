<script lang="ts">
  // Settings modal — the ⚙ rail panel for the 4 user-editable config fields
  // (webhook URL, webhook display name, community Discord invite, shell).
  // Ported field-for-field from main:index.html #settings / main.js
  // openSettings()/saveSettings()/testWebhook(). Save sends only these 4
  // fields — the backend's set_config server-side merges the deck_* fields
  // in (see src/lib/stores/config.ts saveConfig), so there's nothing else
  // to send here. Same modal-card shell as
  // FindingsPanel.svelte/Loot.svelte/FindingEditor.svelte.
  import { untrack } from 'svelte';
  import { config, saveConfig } from '$lib/stores/config';
  import { testWebhook } from '$lib/bridge';

  let { open = $bindable(false) }: { open?: boolean } = $props();

  let webhookUrl = $state('');
  let username = $state('');
  let communityDiscord = $state('');
  let shell = $state('');

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
      });
      saved = true;
      clearTimeout(savedTimer);
      savedTimer = setTimeout(() => (saved = false), 1500);
    } catch (e) {
      console.error(e);
    }
  }

  async function sendTest(): Promise<void> {
    const url = webhookUrl.trim();
    if (!url || testing) return;
    testing = true;
    testResult = null;
    try {
      await testWebhook(url);
      testResult = 'ok';
    } catch (e) {
      console.error(e);
      testResult = 'err';
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
    background: rgba(0, 0, 0, 0.6);
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
    width: min(92vw, 460px);
    max-height: 90vh;
    overflow-y: auto;
    background: rgba(12, 13, 16, 0.92);
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
  .ff-col select {
    width: 100%;
    background: rgba(0, 0, 0, 0.28);
    border: var(--bordw) solid var(--edge2);
    border-radius: calc(var(--radius) - 6px);
    color: var(--ink);
    padding: 9px 11px;
    outline: none;
    font: 500 13px/1.4 var(--fui);
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .ff-col input:focus,
  .ff-col select:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--glowa);
  }
  .ff-col select {
    cursor: pointer;
  }
  .ff-col select option {
    background: var(--bg);
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
