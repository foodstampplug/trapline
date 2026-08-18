<script lang="ts">
  import { flattenTemplates, type Template } from '$lib/data/templates';
  import { isMissing } from '$lib/stores/tools';
  import { INTEGRATIONS, type IntegrationEntry, type IntegrationCardData } from '$lib/data/integrations';
  import { shodanHost, shodanDomain, shodanSearch, leakcheckDomain, leakcheckEmail } from '$lib/bridge';
  import { applyShodanHost } from '$lib/stores/enrichment';
  import { toast } from '$lib/stores/toasts';
  import IntegrationCard from './IntegrationCard.svelte';

  type FlatTemplate = Template & { cat: string };
  // Two row shapes share the same result list + search: a normal Template
  // (resolves `{{var}}`s into a shell command, dispatched via onPick ->
  // CommandBar.loadTemplate -> runCommand) and an integration entry (prompts
  // for one arg, calls the Shodan/LeakCheck bridge, and opens a card instead
  // of ever touching runCommand). `rowKind` is the discriminant.
  type TemplateRow = FlatTemplate & { rowKind: 'template' };
  type IntegrationRow = IntegrationEntry & { rowKind: 'integration' };
  type ResultRow = TemplateRow | IntegrationRow;

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

  // Arg-prompt mode: picking an integration row doesn't close the palette —
  // it swaps the search list for a single-field prompt for that entry's arg
  // (ip/domain/query/email), reusing the same overlay/dialog shell.
  let mode = $state<'search' | 'arg'>('search');
  let pendingIntegration = $state<IntegrationEntry | null>(null);
  let argValue = $state('');
  let argEl: HTMLInputElement | undefined = $state();
  let loading = $state(false);

  // The open card persists independently of `open`/`mode` — picking an
  // integration entry runs the bridge call then closes the whole launcher,
  // same as picking a template closes it after loading the command bar.
  let activeCard = $state<IntegrationCardData | null>(null);
  let activeCardArg = $state('');

  function match(q: string, row: ResultRow): boolean {
    const needle = q.trim().toLowerCase();
    if (!needle) return true;
    return (
      row.name.toLowerCase().includes(needle) ||
      (row.desc ?? '').toLowerCase().includes(needle) ||
      (row.rowKind === 'template' && (row.tool ?? '').toLowerCase().includes(needle)) ||
      row.cat.toLowerCase().includes(needle)
    );
  }

  const results = $derived(
    [
      ...flattenTemplates().map((t): TemplateRow => ({ ...t, rowKind: 'template' })),
      ...INTEGRATIONS.map((i): IntegrationRow => ({ ...i, rowKind: 'integration' })),
    ].filter((row) => match(query, row))
  );

  function rowKey(row: ResultRow): string {
    return row.rowKind === 'template' ? 'template::' + row.cat + '::' + row.name : 'integration::' + row.kind;
  }

  // Keep the highlighted row in range whenever the result set changes.
  $effect(() => {
    if (activeIndex >= results.length) activeIndex = Math.max(0, results.length - 1);
  });

  $effect(() => {
    if (open && mode === 'search') searchEl?.focus();
  });

  $effect(() => {
    if (open && mode === 'arg') argEl?.focus();
  });

  function close(): void {
    open = false;
    query = '';
    activeIndex = 0;
    mode = 'search';
    pendingIntegration = null;
    argValue = '';
  }

  function pick(row: ResultRow): void {
    if (row.rowKind === 'integration') {
      pendingIntegration = row;
      argValue = '';
      mode = 'arg';
      return;
    }
    onPick?.(row);
    close();
  }

  function backToSearch(): void {
    mode = 'search';
    pendingIntegration = null;
    argValue = '';
  }

  function closeCard(): void {
    activeCard = null;
    activeCardArg = '';
  }

  async function callIntegration(entry: IntegrationEntry, arg: string): Promise<IntegrationCardData> {
    if (entry.kind === 'shodanHost') {
      const data = await shodanHost(arg);
      // Feeds the Surface Map's enrichment overlay — an exact type match
      // (ShodanHost -> EnrichEntry). shodanDomain's result (ShodanDomain:
      // domain/subdomains/records) carries none of applyShodanHost's
      // required fields (ports/services/cves/org), so it's deliberately NOT
      // forced through here — that would silently blank out any real
      // enrichment already held for that host.
      applyShodanHost(arg, data);
      return { kind: 'shodanHost', data };
    }
    if (entry.kind === 'shodanDomain') {
      const data = await shodanDomain(arg);
      return { kind: 'shodanDomain', data };
    }
    if (entry.kind === 'shodanSearch') {
      const data = await shodanSearch(arg);
      return { kind: 'shodanSearch', data };
    }
    if (entry.kind === 'leakDomain') {
      const data = await leakcheckDomain(arg);
      return { kind: 'leak', data };
    }
    const data = await leakcheckEmail(arg);
    return { kind: 'leak', data };
  }

  async function runIntegration(): Promise<void> {
    if (!pendingIntegration || loading) return;
    const arg = argValue.trim();
    if (!arg) return;
    const entry = pendingIntegration;
    loading = true;
    try {
      const result = await callIntegration(entry, arg);
      activeCard = result;
      activeCardArg = arg;
      close();
    } catch (e) {
      // Errors (e.g. "Shodan API key not set — add it in Settings") surface
      // via the existing toast channel, never rendered as if they were
      // result data, and never anything beyond the backend's own message —
      // no key material ever reaches this catch block.
      toast(e instanceof Error ? e.message : String(e), 'err');
    } finally {
      loading = false;
    }
  }

  function onWindowKeydown(e: KeyboardEvent): void {
    if (!open) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      if (mode === 'arg') backToSearch();
      else close();
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

  function onArgKeydown(e: KeyboardEvent): void {
    if (e.key === 'Enter') {
      e.preventDefault();
      void runIntegration();
    }
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

{#if open}
  <div class="overlay">
    <button type="button" class="backdrop" aria-label="Close command launcher" onclick={close}></button>
    <div class="palette" role="dialog" aria-modal="true" aria-label="Command launcher" tabindex="-1">
      {#if mode === 'search'}
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
          {#each results as row, i (rowKey(row))}
            <button
              type="button"
              class="row"
              class:active={i === activeIndex}
              onmouseenter={() => (activeIndex = i)}
              onclick={() => pick(row)}
            >
              <span class="name">
                {row.name}
                {#if row.rowKind === 'template' && row.tool && isMissing(row.tool)}
                  <span class="warn">⚠ no {row.tool}</span>
                {/if}
              </span>
              <span class="cat">{row.cat}</span>
            </button>
          {/each}
        </div>
      {:else if mode === 'arg' && pendingIntegration}
        <div class="arghead">
          <button type="button" class="back" onclick={backToSearch} aria-label="Back to search">←</button>
          <div class="argtitle">
            <span class="name">{pendingIntegration.name}</span>
            <span class="desc">{pendingIntegration.desc}</span>
          </div>
        </div>
        <div class="argrow">
          <span class="tag">{pendingIntegration.argLabel}</span>
          <input
            type="text"
            class="argin"
            aria-label={pendingIntegration.argLabel}
            placeholder={pendingIntegration.argPlaceholder}
            bind:value={argValue}
            bind:this={argEl}
            onkeydown={onArgKeydown}
            disabled={loading}
          />
          <button
            type="button"
            class="argrun"
            onclick={() => void runIntegration()}
            disabled={!argValue.trim() || loading}
          >
            {loading ? 'Running…' : '▶ Run'}
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}

{#if activeCard}
  <IntegrationCard card={activeCard} arg={activeCardArg} onClose={closeCard} />
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

  /* Arg-prompt mode (an integration entry was picked) — same searchrow shell
     as the query input, swapped for a single-field prompt + Run/Back. */
  .arghead {
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 13px 15px;
    border-bottom: var(--bordw) solid var(--edge);
  }
  .arghead .back {
    flex-shrink: 0;
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border-radius: calc(var(--radius) - 8px);
    border: 1px solid var(--edge2);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
    font-size: 13px;
  }
  .arghead .back:hover {
    color: var(--ink);
    border-color: var(--accent);
  }
  .arghead .argtitle {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .arghead .argtitle .name {
    font: 700 12.5px/1.3 var(--fui);
    color: var(--ink);
  }
  .arghead .argtitle .desc {
    font: 500 10.5px/1.3 var(--fui);
    color: var(--dim);
  }
  .argrow {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 13px 15px;
  }
  .argrow .tag {
    flex-shrink: 0;
    font: 700 9.5px/1 var(--fmono);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
    border: 1px solid var(--edge2);
    border-radius: 5px;
    padding: 5px 7px;
  }
  .argrow .argin {
    flex: 1;
    min-width: 0;
    background: var(--input-bg);
    border: 1px solid var(--edge2);
    border-radius: calc(var(--radius) - 8px);
    color: var(--ink);
    font: 500 13px/1 var(--fmono);
    padding: 9px 11px;
  }
  .argrow .argin::placeholder {
    color: var(--dim);
  }
  .argrow .argin:disabled {
    opacity: 0.6;
  }
  .argrow .argrun {
    flex-shrink: 0;
    font: 700 11px/1 var(--fdisp);
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent) 40%, transparent);
    border-radius: calc(var(--radius) - 6px);
    padding: 9px 12px;
    cursor: pointer;
    white-space: nowrap;
  }
  .argrow .argrun:disabled {
    color: var(--dim);
    background: rgba(255, 255, 255, 0.03);
    border-color: var(--edge2);
    cursor: default;
  }
</style>
