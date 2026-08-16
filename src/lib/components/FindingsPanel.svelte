<script lang="ts">
  import { findings, newFinding } from '$lib/stores/findings';
  import type { Finding } from '$lib/types';
  import FindingEditor from './FindingEditor.svelte';
  import ReportModal from './ReportModal.svelte';
  import { generateReport } from '$lib/reports/generate';

  let { open = $bindable(false) }: { open?: boolean } = $props();

  // Mirrors main:src/main.js SEV_ORDER (~line 867) — worst-first sort.
  const SEV_ORDER: Record<string, number> = { critical: 0, high: 1, medium: 2, low: 3, info: 4 };
  const STATUS_LABELS: Record<string, string> = {
    draft: 'Draft',
    ready: 'Ready',
    submitted: 'Submitted',
    triaged: 'Triaged',
    resolved: 'Resolved',
    na: 'N/A',
    duplicate: 'Duplicate',
  };

  const sorted = $derived(
    [...$findings].sort(
      (a, b) =>
        (SEV_ORDER[a.severity] ?? 5) - (SEV_ORDER[b.severity] ?? 5) || b.createdAt.localeCompare(a.createdAt)
    )
  );

  // Editor state — a specific finding being created/edited, plus whether it
  // already exists (gates FindingEditor's Delete button).
  let editorFinding = $state<Finding | null>(null);
  let editorExisting = $state(false);

  // Report state — the generated markdown for the selected finding, or null.
  let reportMarkdown = $state<string | null>(null);

  function close(): void {
    open = false;
  }

  function openNew(): void {
    editorFinding = newFinding();
    editorExisting = false;
  }

  function openEdit(f: Finding): void {
    editorFinding = f;
    editorExisting = true;
  }

  function closeEditor(): void {
    editorFinding = null;
  }

  function openReport(f: Finding): void {
    reportMarkdown = generateReport(f);
  }

  function closeReport(): void {
    reportMarkdown = null;
  }

  function onWindowKeydown(e: KeyboardEvent): void {
    if (open && !editorFinding && !reportMarkdown && e.key === 'Escape') {
      e.preventDefault();
      close();
    }
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

{#if open}
  <div class="overlay">
    <button type="button" class="backdrop" aria-label="Close findings" onclick={close}></button>
    <div class="modal-card wide" role="dialog" aria-modal="true" aria-label="Findings">
      <div class="modal-head">
        <h2>Findings <span class="count">{$findings.length ? `(${$findings.length})` : ''}</span></h2>
        <button type="button" class="ghost-btn sm" onclick={openNew}>+ New finding</button>
        <button type="button" class="icon-btn sm" title="Close" onclick={close}>✕</button>
      </div>
      <p class="modal-sub">
        Tracked vulnerabilities. Click a finding to edit it, or <b>Generate Report →</b> for a HackerOne/Bugcrowd-ready
        writeup in one shot.
      </p>
      <div class="list">
        {#if sorted.length === 0}
          <div class="empty">No findings yet. Click <b>+ New finding</b> to start tracking.</div>
        {/if}
        {#each sorted as f (f.id)}
          <div class="row">
            <span class="dot {f.severity}"></span>
            <button type="button" class="info" onclick={() => openEdit(f)}>
              <span class="ftitle">{f.title || '(untitled)'}</span>
              <span class="fmeta"
                >{f.programName}{f.programName && f.endpoint ? ' · ' : ''}{f.endpoint}</span
              >
            </button>
            <span class="status {f.status}">{STATUS_LABELS[f.status] || f.status || 'Draft'}</span>
            <button type="button" class="report-btn" onclick={() => openReport(f)}>Generate Report →</button>
          </div>
        {/each}
      </div>
    </div>
  </div>
{/if}

{#if editorFinding}
  {#key editorFinding.id}
    <FindingEditor finding={editorFinding} existing={editorExisting} onSaved={closeEditor} onClose={closeEditor} />
  {/key}
{/if}

{#if reportMarkdown}
  <ReportModal markdown={reportMarkdown} onClose={closeReport} />
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
  .modal-sub b {
    color: var(--muted);
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
    margin-left: auto;
    flex-shrink: 0;
  }
  .ghost-btn:hover {
    color: var(--ink);
    border-color: var(--accent);
  }
  .ghost-btn.sm {
    padding: 7px 11px;
    font-size: 12px;
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

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 4px;
    border-bottom: var(--bordw) solid var(--edge);
  }
  .row:last-child {
    border-bottom: none;
  }

  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
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
  .dot.low {
    background: var(--ok);
    box-shadow: 0 0 6px color-mix(in srgb, var(--ok) 60%, transparent);
  }
  .dot.info {
    background: var(--dim);
  }

  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    background: transparent;
    border: none;
    padding: 4px 2px;
    cursor: pointer;
    text-align: left;
    font-family: inherit;
    border-radius: calc(var(--radius) - 8px);
  }
  .info:hover {
    background: rgba(255, 255, 255, 0.04);
  }
  .ftitle {
    font: 600 13px/1.3 var(--fui);
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }
  .fmeta {
    font: 500 11px/1.3 var(--fmono);
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  .status {
    flex-shrink: 0;
    font: 700 10px/1 var(--fmono);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--dim);
  }
  .status.ready {
    color: var(--accent);
  }
  .status.submitted {
    color: var(--accent2);
  }
  .status.triaged {
    color: var(--high);
  }
  .status.resolved {
    color: var(--ok);
  }

  .report-btn {
    flex-shrink: 0;
    background: transparent;
    border: var(--bordw) solid var(--edge2);
    color: var(--muted);
    border-radius: calc(var(--radius) - 8px);
    padding: 6px 10px;
    font: 700 11px/1 var(--fdisp);
    cursor: pointer;
    transition: 0.15s;
  }
  .report-btn:hover {
    color: var(--accent);
    border-color: var(--accent);
  }
</style>
