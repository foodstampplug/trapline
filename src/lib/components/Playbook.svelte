<script lang="ts">
  import { TEMPLATES, type Template } from '$lib/data/templates';
  import { isMissing } from '$lib/stores/tools';

  type FlatTemplate = Template & { cat: string };

  let {
    open = $bindable(false),
    onPick,
  }: {
    open?: boolean;
    onPick?: (t: FlatTemplate) => void;
  } = $props();

  let query = $state('');

  function match(q: string, cat: string, t: Template): boolean {
    const needle = q.trim().toLowerCase();
    if (!needle) return true;
    return (
      t.name.toLowerCase().includes(needle) ||
      (t.desc ?? '').toLowerCase().includes(needle) ||
      (t.tool ?? '').toLowerCase().includes(needle) ||
      cat.toLowerCase().includes(needle)
    );
  }

  // Category → filtered items, dropping categories the filter empties out.
  const groups = $derived(
    TEMPLATES.map((c) => ({ cat: c.cat, items: c.items.filter((it) => match(query, c.cat, it)) })).filter(
      (c) => c.items.length > 0
    )
  );

  function close(): void {
    open = false;
    query = '';
  }

  function pick(cat: string, t: Template): void {
    onPick?.({ ...t, cat });
    close();
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
    <button type="button" class="backdrop" aria-label="Close playbook" onclick={close}></button>
    <div class="drawer" role="dialog" aria-modal="true" aria-label="Playbook">
      <div class="hd">
        <span class="ic">📑</span>
        <span class="title">Playbook</span>
        <input
          type="text"
          class="filter"
          placeholder="filter 215 commands…"
          bind:value={query}
        />
        <button type="button" class="x" onclick={close} aria-label="Close playbook">✕</button>
      </div>
      <div class="body">
        {#if groups.length === 0}
          <div class="empty">No commands match "{query}"</div>
        {/if}
        {#each groups as g (g.cat)}
          <div class="cat">
            <div class="cat-h">{g.cat}</div>
            {#each g.items as t (t.name)}
              <button type="button" class="item" onclick={() => pick(g.cat, t)}>
                <div class="row1">
                  <span class="name">{t.name}</span>
                  {#if t.tool && isMissing(t.tool)}
                    <span class="warn">⚠ no {t.tool}</span>
                  {/if}
                </div>
                {#if t.desc}<div class="desc">{t.desc}</div>{/if}
                <div class="cmd">{t.cmd}</div>
              </button>
            {/each}
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
    background: rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(3px);
    display: flex;
    justify-content: flex-end;
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
  .drawer {
    position: relative;
    z-index: 1;
    width: min(420px, 94vw);
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--panel-glass);
    backdrop-filter: blur(22px);
    border-left: 1px solid var(--edge2);
    box-shadow: var(--shadow);
  }
  .hd {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 13px 12px;
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
  .hd .filter {
    flex: 1;
    background: var(--input-bg);
    border: 1px solid var(--edge2);
    border-radius: calc(var(--radius) - 8px);
    color: var(--ink);
    font: 500 11.5px/1 var(--fmono);
    padding: 7px 9px;
  }
  .hd .filter::placeholder {
    color: var(--dim);
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
    padding: 7px;
    flex: 1;
    min-height: 0;
  }
  .empty {
    padding: 20px 12px;
    text-align: center;
    color: var(--dim);
    font: 500 12px/1.5 var(--fui);
  }
  .cat {
    margin-bottom: 6px;
  }
  .cat-h {
    padding: 10px 8px 6px;
    font: 700 10px/1 var(--fmono);
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .item {
    width: 100%;
    display: block;
    text-align: left;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--edge);
    border-radius: calc(var(--radius) - 6px);
    padding: 9px 10px;
    margin-bottom: 5px;
    cursor: pointer;
    font-family: inherit;
    color: inherit;
  }
  .item:hover {
    background: color-mix(in srgb, var(--accent) 10%, rgba(255, 255, 255, 0.03));
    border-color: var(--edge2);
  }
  .row1 {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .name {
    font: 600 12px/1.3 var(--fui);
    color: var(--ink);
  }
  .warn {
    flex-shrink: 0;
    font: 700 9.5px/1 var(--fmono);
    color: var(--high);
    border: 1px solid color-mix(in srgb, var(--high) 45%, transparent);
    border-radius: 5px;
    padding: 3px 5px;
    white-space: nowrap;
  }
  .desc {
    margin-top: 3px;
    font: 500 10.5px/1.4 var(--fui);
    color: var(--dim);
  }
  .cmd {
    margin-top: 5px;
    font: 500 10.5px/1.4 var(--fmono);
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
