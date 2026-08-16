<script lang="ts">
  // Loot panel — the session's deduped flag hits (fed by every finished run
  // in runs.ts), grouped by severity, with Copy Markdown / Send to Discord /
  // Clear. Same modal-card shell as FindingsPanel.svelte/ReportModal.svelte.
  // Loot values are untrusted (attacker-controlled content pulled out of
  // scanned command output) — everything below renders as plain Svelte text
  // interpolation, never {@html}/innerHTML.
  import { loot, lootMarkdown, clearLoot, sevRank } from '$lib/stores/loot';
  import type { LootItem } from '$lib/stores/loot';
  import { sendLoot } from '$lib/bridge';
  import { toast } from '$lib/stores/toasts';

  let { open = $bindable(false) }: { open?: boolean } = $props();

  interface Group {
    sev: string;
    items: LootItem[];
  }

  // Severity-sorted groups, one header per severity present, worst-first —
  // mirrors main:src/main.js renderLoot() (~line 656).
  const groups = $derived.by((): Group[] => {
    const sorted = [...$loot].sort((a, b) => sevRank(a.sev) - sevRank(b.sev));
    const out: Group[] = [];
    for (const x of sorted) {
      const last = out[out.length - 1];
      if (last && last.sev === x.sev) last.items.push(x);
      else out.push({ sev: x.sev, items: [x] });
    }
    return out;
  });

  let copied = $state(false);
  let sending = $state(false);
  let sent = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | undefined;
  let sentTimer: ReturnType<typeof setTimeout> | undefined;

  function close(): void {
    open = false;
  }

  async function copy(): Promise<void> {
    const md = lootMarkdown();
    if (!md) return;
    try {
      await navigator.clipboard.writeText(md);
      copied = true;
      clearTimeout(copyTimer);
      copyTimer = setTimeout(() => (copied = false), 1500);
      toast('Loot copied', 'ok');
    } catch (e) {
      console.error(e);
    }
  }

  // Only ever fires on an explicit click of the Send button — never
  // automatically. Guards on empty loot so there's nothing to POST to the
  // configured webhook when nothing has been collected yet.
  async function send(): Promise<void> {
    const md = lootMarkdown();
    if (!md || sending) return;
    sending = true;
    try {
      await sendLoot({ markdown: md });
      sent = true;
      clearTimeout(sentTimer);
      sentTimer = setTimeout(() => (sent = false), 1500);
      toast('Loot sent to Discord', 'ok');
    } catch (e) {
      console.error(e);
      toast('Discord: ' + String(e), 'err');
    } finally {
      sending = false;
    }
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
    <button type="button" class="backdrop" aria-label="Close loot" onclick={close}></button>
    <div class="modal-card wide" role="dialog" aria-modal="true" aria-label="Loot">
      <div class="modal-head">
        <h2>💰 Loot <span class="count">{$loot.length ? `(${$loot.length})` : ''}</span></h2>
        <button type="button" class="icon-btn sm" title="Close" onclick={close}>✕</button>
      </div>
      <p class="modal-sub">
        Every flag from this session, deduped. Export to your report or fire it to Discord.
      </p>
      <div class="list">
        {#if $loot.length === 0}
          <div class="empty">No flags collected yet. Run some commands — every flagged finding lands here.</div>
        {/if}
        {#each groups as g (g.sev)}
          <div class="sev-head {g.sev}">{g.sev}</div>
          {#each g.items as x (x.sev + '|' + x.name + '|' + x.value)}
            <div class="row cat-{x.cat}">
              <span class="dot {x.sev}"></span>
              <span class="ln">{x.name}</span>
              <span class="lv">{x.value}</span>
              <span class="lc">{x.cmd}</span>
            </div>
          {/each}
        {/each}
      </div>
      <div class="modal-actions">
        <button type="button" class="ghost-btn" onclick={clearLoot} disabled={$loot.length === 0}>Clear</button>
        <span class="spacer"></span>
        <button type="button" class="ghost-btn" onclick={copy} disabled={$loot.length === 0}>
          {copied ? 'Copied ✓' : 'Copy Markdown'}
        </button>
        <button type="button" class="run-btn" onclick={send} disabled={$loot.length === 0 || sending}>
          {sent ? 'Sent ✓' : sending ? 'Sending…' : 'Send to Discord'}
        </button>
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
    width: min(92vw, 800px);
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
    gap: 10px;
    margin-bottom: 8px;
  }
  .modal-head h2 {
    margin: 0;
    font: 700 18px/1 var(--fdisp);
    color: var(--ink);
  }
  .modal-head .count {
    font: 600 13px/1 var(--fui);
    color: var(--muted);
    margin-left: 4px;
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
    margin-left: auto;
    flex-shrink: 0;
  }
  .icon-btn:hover {
    color: var(--ink);
    border-color: var(--accent);
  }
  .modal-sub {
    font: 500 12px/1.5 var(--fui);
    color: var(--dim);
    margin: 0 0 14px;
  }

  .list {
    max-height: 55vh;
    overflow-y: auto;
    padding-right: 2px;
  }
  .empty {
    padding: 32px 12px;
    text-align: center;
    color: var(--dim);
    font: 500 12.5px/1.5 var(--fui);
  }

  .sev-head {
    font: 700 10px/1 var(--fmono);
    text-transform: uppercase;
    letter-spacing: 0.1em;
    margin: 14px 2px 6px;
  }
  .sev-head:first-child {
    margin-top: 2px;
  }
  .sev-head.critical {
    color: var(--crit);
  }
  .sev-head.high {
    color: var(--high);
  }
  .sev-head.medium {
    color: var(--med);
  }
  .sev-head.info {
    color: var(--muted);
  }

  .row {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 7px 8px;
    border-radius: calc(var(--radius) - 8px);
    background: rgba(255, 255, 255, 0.03);
    margin-bottom: 4px;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
    align-self: center;
  }
  .dot.critical {
    background: var(--crit);
    box-shadow: 0 0 6px color-mix(in srgb, var(--crit) 60%, transparent);
  }
  .dot.high {
    background: var(--high);
    box-shadow: 0 0 6px color-mix(in srgb, var(--high) 60%, transparent);
  }
  .dot.medium {
    background: var(--med);
    box-shadow: 0 0 6px color-mix(in srgb, var(--med) 60%, transparent);
  }
  .dot.info {
    background: var(--dim);
  }
  .ln {
    font: 600 12.5px/1.3 var(--fui);
    color: var(--ink);
    white-space: nowrap;
    flex-shrink: 0;
  }
  .lv {
    font: 500 12px/1.3 var(--fmono);
    color: var(--muted);
    word-break: break-all;
    flex: 1;
    min-width: 0;
  }
  .lc {
    font: 500 10.5px/1.3 var(--fmono);
    color: var(--dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 190px;
    flex-shrink: 0;
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
  .run-btn {
    font: 700 13px/1 var(--fdisp);
    /* Dark-on-amber text — same hex ReportModal.svelte/TargetsPanel.svelte/
       FindingEditor.svelte already use for buttons on var(--accent); reused,
       not a new hex. */
    color: #221a06;
    background: var(--accent);
    border: none;
    border-radius: calc(var(--radius) - 6px);
    padding: 10px 16px;
    cursor: pointer;
  }
  .run-btn:hover:not(:disabled) {
    filter: brightness(1.09);
  }
  .run-btn:active:not(:disabled) {
    transform: translateY(1px);
  }
  .ghost-btn:disabled,
  .run-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
</style>
