<script lang="ts">
  import { tools, loadTools } from '$lib/stores/tools';

  let { open = $bindable(false) }: { open?: boolean } = $props();

  function close(): void {
    open = false;
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
    <button type="button" class="backdrop" aria-label="Close tools" onclick={close}></button>
    <div class="modal" role="dialog" aria-modal="true" aria-label="Tools">
      <div class="hd">
        <span class="ic">🧰</span>
        <span class="title">Tools</span>
        <button type="button" class="recheck" onclick={() => loadTools()}>⟳ Recheck</button>
        <button type="button" class="x" onclick={close} aria-label="Close tools">✕</button>
      </div>
      <div class="body">
        {#if $tools.length === 0}
          <div class="empty">No tool data yet — click Recheck.</div>
        {/if}
        {#each $tools as t (t.name)}
          <div class="row">
            <span class="name">{t.name}</span>
            {#if t.found}
              <span class="status ok">✓ installed</span>
            {:else}
              <span class="status bad">✗ missing</span>
            {/if}
            {#if t.hint}<span class="hint">{t.hint}</span>{/if}
          </div>
        {/each}
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
    backdrop-filter: blur(3px);
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
  .modal {
    position: relative;
    z-index: 1;
    width: min(440px, 92vw);
    max-height: 70vh;
    display: flex;
    flex-direction: column;
    background: rgba(20, 22, 28, 0.9);
    backdrop-filter: blur(22px);
    border: 1px solid var(--edge2);
    border-radius: var(--radius);
    box-shadow: var(--shadow);
    overflow: hidden;
  }
  .hd {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 13px 14px;
    border-bottom: var(--bordw) solid var(--edge);
    flex-shrink: 0;
  }
  .hd .ic {
    font-size: 15px;
  }
  .hd .title {
    font: 700 12.5px/1 var(--fdisp);
    letter-spacing: 0.04em;
    color: var(--ink);
  }
  .hd .recheck {
    margin-left: auto;
    font: 700 10.5px/1 var(--fdisp);
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent) 40%, transparent);
    border-radius: calc(var(--radius) - 7px);
    padding: 7px 10px;
    cursor: pointer;
  }
  .hd .x {
    flex-shrink: 0;
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border-radius: calc(var(--radius) - 8px);
    border: 1px solid var(--edge);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
    font-size: 11px;
  }
  .body {
    overflow-y: auto;
    padding: 8px;
  }
  .empty {
    padding: 20px 12px;
    text-align: center;
    color: var(--dim);
    font: 500 12px/1.5 var(--fui);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 10px;
    border-radius: calc(var(--radius) - 6px);
    border: 1px solid var(--edge);
    background: rgba(255, 255, 255, 0.03);
    margin-bottom: 6px;
  }
  .row .name {
    font: 600 12px/1 var(--fmono);
    color: var(--ink);
  }
  .row .status {
    margin-left: auto;
    font: 700 10.5px/1 var(--fmono);
    flex-shrink: 0;
  }
  .row .status.ok {
    color: var(--ok);
  }
  .row .status.bad {
    color: var(--crit);
  }
  .row .hint {
    flex-shrink: 0;
    font: 500 10px/1 var(--fmono);
    color: var(--dim);
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
