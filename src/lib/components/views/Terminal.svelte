<script lang="ts">
  import { runs, type Run } from '$lib/stores/runs';
  import OutputLine from './OutputLine.svelte';

  function findingLabel(f: unknown): string {
    if (f && typeof f === 'object') {
      const o = f as Record<string, unknown>;
      if (typeof o.title === 'string') return o.title;
      if (typeof o.name === 'string') return o.name;
    }
    return typeof f === 'string' ? f : JSON.stringify(f);
  }

  function statusLabel(run: Run): string {
    return `✓ ${run.code ?? 0} · ${run.ms ?? 0}ms`;
  }
</script>

<div class="term">
  {#if $runs.length === 0}
    <div class="empty">No runs yet — launch a command to see live output here.</div>
  {/if}
  {#each $runs as run (run.id)}
    <div class="run">
      <div class="ln head"><span class="p">$</span> {run.cmdline}</div>
      {#each run.lines as line, i (i)}
        <div class="ln" class:err={line.stream === 'err'}>
          <OutputLine text={line.text} spans={line.spans} />
        </div>
      {/each}
      <div class="statusrow">
        {#if run.status === 'running'}
          <span class="running"><span class="dot"></span> running…</span>
        {:else}
          <span class="done" class:bad={run.code !== 0}>{statusLabel(run)}</span>
        {/if}
      </div>
      {#if run.findings.length > 0}
        <div class="chipline">
          {#each run.findings as f, i (i)}
            <span>{findingLabel(f)}</span>
          {/each}
        </div>
      {/if}
    </div>
  {/each}
</div>

<style>
  .term {
    height: 100%;
    overflow: auto;
    padding: 13px 15px;
    font: 12.5px/1.75 var(--fmono);
  }
  .empty {
    color: var(--dim);
    font: 500 12px/1.6 var(--fui);
    padding: 8px 2px;
  }
  .run {
    padding-bottom: 16px;
    margin-bottom: 16px;
    border-bottom: var(--bordw) solid var(--edge);
  }
  .run:last-child {
    border-bottom: 0;
    margin-bottom: 0;
    padding-bottom: 0;
  }
  .ln {
    display: block;
    white-space: pre-wrap;
    color: #c6cdda;
  }
  .ln.head {
    font-weight: 700;
    color: var(--ink);
    margin-bottom: 3px;
  }
  .ln.err {
    color: var(--crit);
  }
  .p {
    color: var(--accent);
  }
  .statusrow {
    margin-top: 6px;
    font: 600 11px/1 var(--fmono);
  }
  .running {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--dim);
  }
  .running .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent2);
    animation: bl 1.4s infinite;
  }
  .done {
    color: var(--ok);
  }
  .done.bad {
    color: var(--crit);
  }
  .chipline {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin: 10px 0 2px;
  }
  .chipline span {
    font: 600 11px/1 var(--fmono);
    color: #d6b26a;
    background: rgba(255, 191, 71, 0.08);
    border: 1px solid rgba(255, 191, 71, 0.25);
    border-radius: 999px;
    padding: 6px 10px;
  }
  @keyframes bl {
    50% {
      opacity: 0;
    }
  }
</style>
