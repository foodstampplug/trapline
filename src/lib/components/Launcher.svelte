<script lang="ts">
  import { flattenTemplates, type Template } from '$lib/data/templates';
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
  let activeIndex = $state(0);
  let searchEl: HTMLInputElement | undefined = $state();

  function match(q: string, t: FlatTemplate): boolean {
    const needle = q.trim().toLowerCase();
    if (!needle) return true;
    return (
      t.name.toLowerCase().includes(needle) ||
      (t.desc ?? '').toLowerCase().includes(needle) ||
      (t.tool ?? '').toLowerCase().includes(needle) ||
      t.cat.toLowerCase().includes(needle)
    );
  }

  const results = $derived(flattenTemplates().filter((t) => match(query, t)));

  // Keep the highlighted row in range whenever the result set changes.
  $effect(() => {
    if (activeIndex >= results.length) activeIndex = Math.max(0, results.length - 1);
  });

  $effect(() => {
    if (open) searchEl?.focus();
  });

  function close(): void {
    open = false;
    query = '';
    activeIndex = 0;
  }

  function pick(t: FlatTemplate): void {
    onPick?.(t);
    close();
  }

  function onWindowKeydown(e: KeyboardEvent): void {
    if (!open) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      close();
    }
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape') {
      e.preventDefault();
      close();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      activeIndex = Math.min(activeIndex + 1, results.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      activeIndex = Math.max(activeIndex - 1, 0);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const t = results[activeIndex];
      if (t) pick(t);
    }
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

{#if open}
  <div class="overlay">
    <button type="button" class="backdrop" aria-label="Close command launcher" onclick={close}></button>
    <div class="palette" role="dialog" aria-modal="true" aria-label="Command launcher" tabindex="-1">
      <div class="searchrow">
        <span class="ic">⌘K</span>
        <input
          type="text"
          class="q"
          placeholder="Search 215 recon commands…"
          bind:value={query}
          bind:this={searchEl}
          onkeydown={onKeydown}
        />
        <span class="hint">esc</span>
      </div>
      <div class="results">
        {#if results.length === 0}
          <div class="empty">No commands match "{query}"</div>
        {/if}
        {#each results as t, i (t.cat + '::' + t.name)}
          <button
            type="button"
            class="row"
            class:active={i === activeIndex}
            onmouseenter={() => (activeIndex = i)}
            onclick={() => pick(t)}
          >
            <span class="name">
              {t.name}
              {#if t.tool && isMissing(t.tool)}
                <span class="warn">⚠ no {t.tool}</span>
              {/if}
            </span>
            <span class="cat">{t.cat}</span>
          </button>
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
    background: var(--scrim);
    backdrop-filter: blur(3px);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 12vh;
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
  .palette {
    position: relative;
    z-index: 1;
    width: min(600px, 92vw);
    max-height: 62vh;
    display: flex;
    flex-direction: column;
    background: rgba(20, 22, 28, 0.86);
    backdrop-filter: blur(22px);
    border: 1px solid var(--edge2);
    border-radius: var(--radius);
    box-shadow: var(--shadow);
    overflow: hidden;
  }
  .searchrow {
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 13px 15px;
    border-bottom: var(--bordw) solid var(--edge);
  }
  .searchrow .ic {
    font: 700 10px/1 var(--fmono);
    color: var(--dim);
    border: 1px solid var(--edge2);
    border-radius: 5px;
    padding: 4px 6px;
  }
  .searchrow .q {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: var(--ink);
    font: 500 14px/1 var(--fmono);
  }
  .searchrow .q::placeholder {
    color: var(--dim);
  }
  .searchrow .hint {
    font: 600 9.5px/1 var(--fmono);
    color: var(--dim);
    border: 1px solid var(--edge2);
    border-radius: 5px;
    padding: 3px 5px;
  }
  .results {
    overflow-y: auto;
    padding: 7px;
  }
  .empty {
    padding: 20px 12px;
    text-align: center;
    color: var(--dim);
    font: 500 12px/1.5 var(--fui);
  }
  .row {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 10px 11px;
    border: none;
    border-radius: calc(var(--radius) - 6px);
    background: transparent;
    text-align: left;
    cursor: pointer;
    font-family: inherit;
    color: inherit;
  }
  .row.active {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }
  .row .name {
    font: 600 12.5px/1.3 var(--fui);
    color: var(--ink);
  }
  .row .cat {
    flex-shrink: 0;
    font: 700 9.5px/1 var(--fmono);
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .row .warn {
    margin-left: 8px;
    font: 700 9.5px/1 var(--fmono);
    color: var(--high);
    border: 1px solid color-mix(in srgb, var(--high) 45%, transparent);
    border-radius: 5px;
    padding: 3px 5px;
    white-space: nowrap;
  }
</style>
