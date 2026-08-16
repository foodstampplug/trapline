<script lang="ts">
  import { extractVars, resolveCmd } from '$lib/templates/engine';
  import { startRun } from '$lib/stores/runs';

  let {
    cmd = $bindable(''),
    onRun,
  }: {
    cmd?: string;
    onRun?: (id: string) => void;
  } = $props();

  let vars = $state<Record<string, string>>({});

  const varNames = $derived(extractVars(cmd));
  const preview = $derived(resolveCmd(cmd, vars));

  /** Loads a picked template into the bar and resets any prior fill-in values.
   * Exposed as a component export so the shell can call it via `bind:this`
   * from the launcher's `pick` handler. */
  export function loadTemplate(t: { cmd: string }): void {
    cmd = t.cmd;
    vars = {};
  }

  function run(): void {
    const resolved = resolveCmd(cmd, vars);
    if (!resolved.trim()) return;
    const id = startRun(resolved);
    onRun?.(id);
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      run();
    }
  }
</script>

<div class="cmd">
  <div class="row">
    <span class="prompt">$</span>
    <input
      type="text"
      class="cmdline"
      placeholder="type a command, or ⌘K to browse 215…"
      bind:value={cmd}
      onkeydown={onKeydown}
    />
    <button type="button" class="run" onclick={run} disabled={!cmd.trim()}>▶ Run</button>
  </div>

  {#if varNames.length > 0}
    <div class="fills">
      {#each varNames as name (name)}
        <label class="fill">
          <span class="tag">{name}</span>
          <input type="text" placeholder={`{{${name}}}`} bind:value={vars[name]} />
        </label>
      {/each}
    </div>
    <div class="preview"><span class="k">resolved</span> {preview}</div>
  {/if}
</div>

<style>
  .cmd {
    padding: 10px 14px;
    border-bottom: var(--bordw) solid var(--edge);
    background: rgba(255, 255, 255, 0.022);
    backdrop-filter: blur(14px);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--input-bg);
    border: 1px solid var(--edge2);
    border-radius: calc(var(--radius) - 4px);
    padding: 9px 11px;
    box-shadow: 0 0 0 3px var(--glowa);
  }
  .prompt {
    color: var(--accent);
    font: 700 13px/1 var(--fmono);
  }
  .cmdline {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: var(--ink);
    font: 500 13px/1 var(--fmono);
  }
  .cmdline::placeholder {
    color: var(--dim);
  }
  .run {
    flex-shrink: 0;
    font: 700 11px/1 var(--fdisp);
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent) 40%, transparent);
    border-radius: calc(var(--radius) - 6px);
    padding: 7px 12px;
    cursor: pointer;
  }
  .run:disabled {
    color: var(--dim);
    background: rgba(255, 255, 255, 0.03);
    border-color: var(--edge2);
    cursor: default;
  }
  .fills {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 9px;
  }
  .fill {
    display: flex;
    align-items: center;
    gap: 6px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--edge);
    border-radius: calc(var(--radius) - 6px);
    padding: 5px 6px 5px 10px;
  }
  .fill .tag {
    font: 700 9.5px/1 var(--fmono);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .fill input {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid var(--edge2);
    border-radius: calc(var(--radius) - 9px);
    color: var(--ink);
    font: 500 11.5px/1 var(--fmono);
    padding: 5px 7px;
    width: 150px;
  }
  .fill input::placeholder {
    color: var(--dim);
  }
  .preview {
    margin-top: 8px;
    font: 500 11px/1.5 var(--fmono);
    color: var(--dim);
    word-break: break-all;
  }
  .preview .k {
    font: 700 9px/1 var(--fmono);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
    margin-right: 6px;
  }
</style>
