<script lang="ts">
  import { watch } from '$lib/stores/watch';
  import { findings } from '$lib/stores/findings';

  // Mirrors FindingsPanel.svelte's SEV_ORDER — worst-first sort.
  const SEV_ORDER: Record<string, number> = { critical: 0, high: 1, medium: 2, low: 3, info: 4 };
  const SEV_ABBR: Record<string, string> = { critical: 'C', high: 'H', medium: 'M', low: 'L', info: 'I' };

  function timeAgo(ms: number): string {
    if (!ms) return '—';
    const s = Math.max(0, Math.floor((Date.now() - ms) / 1000));
    if (s < 60) return `${s}s`;
    if (s < 3600) return `${Math.floor(s / 60)}m`;
    return `${Math.floor(s / 3600)}h`;
  }

  // Worst-first, capped for the dock.
  $: sortedFindings = [...$findings].sort(
    (a, b) => (SEV_ORDER[a.severity] ?? 5) - (SEV_ORDER[b.severity] ?? 5) || b.createdAt.localeCompare(a.createdAt),
  );
</script>

<div class="right">
  <div class="wbox">
    <div class="ph" style="border:0;padding:0 0 9px">
      Watch
      {#if $watch.running}
        <span class="wlive"><span class="p"></span>live</span>
      {:else}
        <span class="wlive off">idle</span>
      {/if}
    </div>
    {#if $watch.lastNew > 0}
      <div class="wl">
        <span class="sev high">new</span>
        <div class="t"><b>{$watch.lastNew} new artifacts</b><span>since last cycle</span></div>
      </div>
    {:else}
      <div class="wl off">
        <span class="sev">—</span>
        <div class="t">No changes<span>since last cycle</span></div>
      </div>
    {/if}
    <div class="wm">
      <div>Last<b>{timeAgo($watch.lastRunMs)}</b></div>
      <div>Assets<b>{$watch.lastAssets}</b></div>
      <div>New<b>{$watch.lastNew}</b></div>
      <div>Every<b>{Math.round($watch.intervalSecs / 60)}m</b></div>
    </div>
  </div>
  <div class="findbox">
    <div class="ph">Findings <span class="c">{$findings.length}</span></div>
    <div class="fl">
      {#each sortedFindings.slice(0, 6) as f (f.id)}
        <div class="fi">
          <span class="sev {f.severity}">{SEV_ABBR[f.severity] ?? '•'}</span>
          <div class="t">{f.title}<span>{f.programName || f.endpoint}</span></div>
          <span class="go">›</span>
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  @keyframes bl {
    50% {
      opacity: 0;
    }
  }

  .right {
    border-left: var(--bordw) solid var(--edge);
    background: rgba(255, 255, 255, 0.02);
    backdrop-filter: blur(16px);
    display: grid;
    grid-template-rows: auto 1fr;
    min-height: 0;
  }
  .ph {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 9px 12px;
    border-bottom: var(--bordw) solid var(--edge);
    font: 700 10px/1 var(--fmono);
    letter-spacing: 0.11em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .ph .c {
    margin-left: auto;
    color: var(--dim);
  }
  .sev {
    font: 800 8.5px/1 var(--fmono);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    border: 1.5px solid currentColor;
    border-radius: 5px;
    padding: 3px 5px;
    white-space: nowrap;
  }
  .sev.high {
    color: var(--high);
  }
  .sev.crit,
  .sev.critical {
    color: var(--crit);
  }
  .sev.medium {
    color: var(--med);
  }
  /* No dedicated --low/--info tokens in tokens.css — reuse the closest
     existing token, same mapping FindingsPanel.svelte's .dot.low/.dot.info use. */
  .sev.low {
    color: var(--ok);
  }
  .sev.info {
    color: var(--dim);
  }
  .wbox {
    padding: 11px;
    border-bottom: var(--bordw) solid var(--edge);
  }
  .wl {
    display: flex;
    align-items: center;
    gap: 9px;
    background: color-mix(in srgb, var(--high) 12%, rgba(255, 255, 255, 0.03));
    border: 1px solid color-mix(in srgb, var(--high) 40%, transparent);
    border-radius: calc(var(--radius) - 3px);
    padding: 10px;
  }
  .wl.off {
    background: rgba(255, 255, 255, 0.03);
    border-color: var(--edge);
    color: var(--dim);
  }
  .wl .t {
    font: 600 11.5px/1.35 var(--fui);
  }
  .wl .t b {
    font-family: var(--fmono);
  }
  .wl .t span {
    display: block;
    color: var(--dim);
    font: 500 10px/1 var(--fmono);
    margin-top: 2px;
  }
  .wm {
    display: flex;
    gap: 13px;
    margin-top: 10px;
  }
  .wm div {
    font: 600 10px/1.2 var(--fui);
    color: var(--muted);
  }
  .wm b {
    display: block;
    color: var(--ink);
    font: 700 14px/1.2 var(--fmono);
    margin-top: 3px;
    font-variant-numeric: tabular-nums;
  }
  .wlive {
    display: flex;
    align-items: center;
    gap: 6px;
    font: 700 9px/1 var(--fmono);
    letter-spacing: 0.1em;
    color: var(--ok);
    text-transform: uppercase;
    margin-left: auto;
  }
  .wlive .p {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--ok);
    animation: bl 1.4s infinite;
  }
  .wlive.off {
    color: var(--dim);
  }
  .findbox {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .fl {
    padding: 9px;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .fi {
    display: flex;
    align-items: center;
    gap: 9px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--edge);
    border-radius: calc(var(--radius) - 3px);
    padding: 9px 10px;
    cursor: pointer;
  }
  .fi .t {
    font: 600 11.5px/1.35 var(--fui);
  }
  .fi .t span {
    display: block;
    color: var(--dim);
    font: 500 10px/1 var(--fmono);
    margin-top: 2px;
  }
  .fi .go {
    margin-left: auto;
    color: var(--dim);
  }
</style>
