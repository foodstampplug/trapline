<script lang="ts">
  import { activity } from '$lib/stores/activity';

  type FilterId = 'all' | 'recon' | 'finding';

  const FILTERS: { id: FilterId; label: string }[] = [
    { id: 'all', label: 'All' },
    { id: 'recon', label: 'Recon' },
    { id: 'finding', label: 'Findings' },
  ];

  let filter = $state<FilterId>('all');

  let events = $derived($activity.filter((e) => filter === 'all' || e.kind === filter));

  function setFilter(id: FilterId): void {
    filter = id;
  }

  /** Pure relative-time formatter: sub-minute reads "now", then "Nm" up to an
   * hour, "Nh" up to a day, "Nd" beyond that. Takes `now` as a parameter
   * (rather than reading `Date.now()` internally) so it stays a plain,
   * independently-testable function — callers pass `Date.now()` at render. */
  function timeAgo(ts: number, now: number): string {
    const diffMin = Math.floor(Math.max(0, now - ts) / 60000);
    if (diffMin < 1) return 'now';
    if (diffMin < 60) return `${diffMin}m`;
    const diffHr = Math.floor(diffMin / 60);
    if (diffHr < 24) return `${diffHr}h`;
    return `${Math.floor(diffHr / 24)}d`;
  }

  /** Buckets a finding's free-text severity into the three chip colors this
   * view has tokens for. Mirrors SurfaceMap.svelte's `severityClass`:
   * critical/high get their own color, everything else (medium/low/info)
   * falls back to `med`. */
  function sevClass(sev: string): string {
    if (sev === 'critical') return 'crit';
    if (sev === 'high') return 'high';
    return 'med';
  }
</script>

<div class="feed">
  <div class="filters">
    {#each FILTERS as f (f.id)}
      <button type="button" class:on={filter === f.id} onclick={() => setFilter(f.id)}>{f.label}</button>
    {/each}
  </div>

  {#if events.length === 0}
    <div class="empty">No activity yet — run a command.</div>
  {:else}
    <div class="tl">
      {#each events as event, i (event.kind + '-' + event.ts + '-' + i)}
        <div class="ev" class:r={event.kind === 'recon'} class:f={event.kind === 'finding'}>
          <div class="c">
            <div class="eh">
              <span class="k">{event.kind === 'recon' ? '▚ Recon' : '🐛 Finding'}</span>
              {#if event.severity}
                <span class="sev {sevClass(event.severity)}">{event.severity}</span>
              {/if}
              <span class="tm">{timeAgo(event.ts, Date.now())}</span>
            </div>
            <div class="tt" class:mono={event.kind === 'recon'}>{event.title}</div>
            {#if event.sub}
              <div class="sub">{event.sub}</div>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .feed {
    display: flex;
    flex-direction: column;
  }

  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding-bottom: 14px;
  }
  .filters button {
    font: 700 10.5px/1 var(--fmono);
    color: var(--muted);
    background: rgba(255, 255, 255, 0.03);
    border: var(--bordw) solid var(--edge);
    border-radius: calc(var(--radius) - 8px);
    padding: 6px 11px;
    cursor: pointer;
  }
  .filters button.on {
    color: var(--ink);
    border-color: var(--accent2);
    background: color-mix(in srgb, var(--accent2) 14%, transparent);
  }

  .empty {
    padding: 32px 12px;
    text-align: center;
    color: var(--dim);
    font: 500 12.5px/1.5 var(--fui);
  }

  .tl {
    position: relative;
    padding-left: 30px;
  }
  .tl::before {
    content: '';
    position: absolute;
    left: 9px;
    top: 6px;
    bottom: 6px;
    width: 2px;
    background: var(--edge);
  }
  .ev {
    position: relative;
    margin: 12px 0;
  }
  .ev::before {
    content: '';
    position: absolute;
    left: -25px;
    top: 15px;
    width: 11px;
    height: 11px;
    border-radius: 50%;
    background: var(--muted);
    border: 3px solid var(--bg);
    box-shadow: 0 0 0 1px var(--edge);
  }
  .ev.r::before {
    background: var(--accent2);
  }
  .ev.f::before {
    background: var(--accent);
  }
  .ev .c {
    background: rgba(255, 255, 255, 0.03);
    backdrop-filter: blur(14px);
    border: 1px solid var(--edge);
    border-radius: calc(var(--radius) - 2px);
    padding: 11px 14px;
  }
  .ev .eh {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 5px;
  }
  .ev .eh .k {
    font: 700 9.5px/1 var(--fmono);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .ev .eh .tm {
    margin-left: auto;
    font: 500 10px/1 var(--fmono);
    color: var(--dim);
  }
  .ev .tt {
    font: 600 12.5px/1.45 var(--fui);
    color: var(--ink);
  }
  .ev .tt.mono {
    font-family: var(--fmono);
  }
  .ev .sub {
    margin-top: 3px;
    font: 500 11px/1.4 var(--fmono);
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
  .sev.crit {
    color: var(--crit);
  }
  .sev.high {
    color: var(--high);
  }
  .sev.med {
    color: var(--med);
  }
</style>
