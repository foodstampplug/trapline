<script lang="ts">
  import type { Span } from '$lib/events';

  let { text, spans }: { text: string; spans: Span[] } = $props();

  interface Segment {
    text: string;
    mark: boolean;
    cat?: string;
    sev?: string;
    label?: string;
  }

  // Sort spans by start, walk the text emitting alternating plain/marked
  // segments for [s,e) ranges. Spans are clamped to the text bounds and to
  // the current cursor position, so out-of-range or overlapping spans never
  // produce a negative-length or duplicated slice — they're just skipped.
  function buildSegments(t: string, sp: Span[]): Segment[] {
    const sorted = [...sp].sort((a, b) => a.s - b.s);
    const out: Segment[] = [];
    let cursor = 0;
    for (const span of sorted) {
      const s = Math.max(span.s, cursor);
      const e = Math.min(span.e, t.length);
      if (e <= s) continue;
      if (s > cursor) out.push({ text: t.slice(cursor, s), mark: false });
      out.push({ text: t.slice(s, e), mark: true, cat: span.cat, sev: span.sev, label: span.label });
      cursor = e;
    }
    if (cursor < t.length) out.push({ text: t.slice(cursor), mark: false });
    return out;
  }

  let segments = $derived(buildSegments(text, spans));
</script>

<span class="out-line">
  {#each segments as seg, i (i)}
    {#if seg.mark}
      <mark class="flag cat-{seg.cat} sev-{seg.sev}" title={seg.label}>{seg.text}</mark>
    {:else}{seg.text}{/if}
  {/each}
</span>

<style>
  .out-line {
    white-space: pre-wrap;
    word-break: break-word;
  }
  mark.flag {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    color: #ffe6b0;
    border-radius: 3px;
    padding: 1px 5px;
    font-weight: 700;
  }
  mark.flag.sev-crit {
    background: color-mix(in srgb, var(--crit) 24%, transparent);
    color: var(--crit);
  }
  mark.flag.sev-high {
    background: color-mix(in srgb, var(--high) 22%, transparent);
    color: var(--high);
  }
  mark.flag.sev-med {
    background: color-mix(in srgb, var(--med) 20%, transparent);
    color: var(--med);
  }
</style>
