<script lang="ts">
  import { untrack } from 'svelte';
  import type { Finding } from '$lib/types';
  import { CVSS_DEFAULTS, type Severity } from '$lib/data/cvss';
  import { saveFinding, deleteFinding } from '$lib/stores/findings';

  let {
    finding,
    existing = false,
    onSaved,
    onClose,
  }: {
    finding: Finding;
    existing?: boolean;
    onSaved?: () => void;
    onClose?: () => void;
  } = $props();

  // Local working copy — editing this never mutates the `finding` prop.
  // Deliberately a one-time snapshot (not a $derived): the form needs
  // mutable local state, decoupled from the prop after mount.
  let local = $state<Finding>(untrack(() => ({ ...finding })));

  const SEVERITIES: { value: Severity; label: string }[] = [
    { value: 'critical', label: '🔴 Critical' },
    { value: 'high', label: '🟠 High' },
    { value: 'medium', label: '🟡 Medium' },
    { value: 'low', label: '🟢 Low' },
    { value: 'info', label: '⚪ Info' },
  ];

  const STATUSES: { value: string; label: string }[] = [
    { value: 'draft', label: 'Draft' },
    { value: 'ready', label: 'Ready to Submit' },
    { value: 'submitted', label: 'Submitted' },
    { value: 'triaged', label: 'Triaged' },
    { value: 'resolved', label: 'Resolved ✓' },
    { value: 'na', label: 'N/A' },
    { value: 'duplicate', label: 'Duplicate' },
  ];

  const PLATFORMS: { value: string; label: string }[] = [
    { value: 'h1', label: 'HackerOne' },
    { value: 'bc', label: 'Bugcrowd' },
    { value: 'synack', label: 'Synack' },
    { value: 'intigriti', label: 'Intigriti' },
    { value: 'yeswehack', label: 'YesWeHack' },
    { value: 'other', label: 'Other / Private' },
  ];

  /** True when `v` is exactly some severity's canned default vector — i.e. it
   * hasn't been hand-edited away from whatever auto-fill last put there. */
  function isDefaultVector(v: string): boolean {
    return (Object.keys(CVSS_DEFAULTS) as Severity[]).some((s) => CVSS_DEFAULTS[s].vector === v);
  }

  // Severity → CVSS auto-fill. Reacts only to `local.severity`; the read/write
  // of cvss/cvssScore is untracked so this can't loop on its own writes.
  $effect(() => {
    const sev = local.severity as Severity;
    const def = CVSS_DEFAULTS[sev];
    if (!def) return;
    untrack(() => {
      if (!local.cvss || isDefaultVector(local.cvss)) {
        local.cvss = def.vector;
        local.cvssScore = def.score;
      }
    });
  });

  const canSave = $derived(local.title.trim().length > 0);

  async function save(): Promise<void> {
    if (!canSave) return;
    try {
      await saveFinding(local);
    } catch (e) {
      console.error(e);
    }
    onSaved?.();
  }

  async function remove(): Promise<void> {
    try {
      await deleteFinding(local.id);
    } catch (e) {
      console.error(e);
    }
    onClose?.();
  }
</script>

<div class="overlay">
  <button type="button" class="backdrop" aria-label="Close finding editor" onclick={() => onClose?.()}></button>
  <div class="modal-card finding-card" role="dialog" aria-modal="true" aria-label="Finding editor">
    <div class="modal-head">
      <h2>{local.title ? 'Edit Finding' : 'New Finding'}</h2>
      <button type="button" class="icon-btn sm" title="Close" onclick={() => onClose?.()}>✕</button>
    </div>

    <div class="finding-form">
      <div class="ff-row">
        <div class="ff-col ff-full">
          <label for="ffTitle">Title <span class="lbl-req">*</span></label>
          <input
            id="ffTitle"
            type="text"
            spellcheck="false"
            placeholder="e.g. IDOR in /api/v1/users/{'{id}'} allows horizontal privilege escalation"
            bind:value={local.title}
          />
        </div>
      </div>

      <div class="ff-row">
        <div class="ff-col">
          <label for="ffSeverity">Severity</label>
          <select id="ffSeverity" bind:value={local.severity}>
            {#each SEVERITIES as s (s.value)}
              <option value={s.value}>{s.label}</option>
            {/each}
          </select>
        </div>
        <div class="ff-col">
          <label for="ffStatus">Status</label>
          <select id="ffStatus" bind:value={local.status}>
            {#each STATUSES as s (s.value)}
              <option value={s.value}>{s.label}</option>
            {/each}
          </select>
        </div>
        <div class="ff-col">
          <label for="ffPlatform">Platform</label>
          <select id="ffPlatform" bind:value={local.platform}>
            {#each PLATFORMS as p (p.value)}
              <option value={p.value}>{p.label}</option>
            {/each}
          </select>
        </div>
      </div>

      <div class="ff-row">
        <div class="ff-col ff-wide">
          <label for="ffProgram">Program name</label>
          <input id="ffProgram" type="text" placeholder="e.g. LPL Financial" bind:value={local.programName} />
        </div>
        <div class="ff-col ff-wide">
          <label for="ffEndpoint">Vulnerable endpoint</label>
          <input
            id="ffEndpoint"
            type="text"
            placeholder="https://api.target.com/v1/users/123"
            bind:value={local.endpoint}
          />
        </div>
      </div>

      <div class="ff-row">
        <div class="ff-col ff-full">
          <label for="ffSummary">Summary <span class="lbl-note">(2–3 sentences)</span></label>
          <textarea
            id="ffSummary"
            rows="3"
            placeholder="What is the vulnerability, where is it, and how severe is it?"
            bind:value={local.summary}
          ></textarea>
        </div>
      </div>

      <div class="ff-row">
        <div class="ff-col ff-full">
          <label for="ffSteps">Steps to reproduce</label>
          <textarea
            id="ffSteps"
            rows="5"
            placeholder={"1. Log in as a normal user\n2. Send the following request, changing the id parameter...\n3. Observe that the response contains another user's data"}
            bind:value={local.steps}
          ></textarea>
        </div>
      </div>

      <div class="ff-row">
        <div class="ff-col ff-full">
          <label for="ffEvidence">Proof of concept / Evidence <span class="lbl-note">(auto-filled from command output)</span></label>
          <textarea
            id="ffEvidence"
            rows="7"
            class="mono"
            placeholder={"curl.exe -s https://api.target.com/v1/users/124 -H 'Authorization: Bearer ...'\n\n{\"id\":124,\"email\":\"victim@example.com\",...}"}
            bind:value={local.evidence}
          ></textarea>
        </div>
      </div>

      <div class="ff-row">
        <div class="ff-col ff-full">
          <label for="ffImpact">Impact</label>
          <textarea
            id="ffImpact"
            rows="3"
            placeholder="An unauthenticated attacker can [action] by [method], allowing [consequence] affecting [scope], exposing [company] to [regulatory/financial risk]."
            bind:value={local.impact}
          ></textarea>
        </div>
      </div>

      <div class="ff-row">
        <div class="ff-col ff-full">
          <label for="ffRemediation">Remediation</label>
          <textarea
            id="ffRemediation"
            rows="2"
            placeholder="Specific fix — not generic advice. E.g. Enforce object-level authorization: verify the requesting user owns the resource before returning it."
            bind:value={local.remediation}
          ></textarea>
        </div>
      </div>

      <div class="ff-row">
        <div class="ff-col ff-wide">
          <label for="ffCVSS">CVSS vector <span class="lbl-note">(auto-suggested from severity)</span></label>
          <input
            id="ffCVSS"
            type="text"
            class="mono"
            placeholder="CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H"
            bind:value={local.cvss}
          />
        </div>
        <div class="ff-col ff-score">
          <label for="ffCVSSScore">Score</label>
          <input id="ffCVSSScore" type="text" placeholder="9.8" bind:value={local.cvssScore} />
        </div>
      </div>

      <div class="ff-row">
        <div class="ff-col ff-full">
          <label for="ffNotes">Private notes <span class="lbl-note">(not included in report)</span></label>
          <textarea
            id="ffNotes"
            rows="2"
            placeholder="Hunt notes, follow-up ideas, escalation paths..."
            bind:value={local.notes}
          ></textarea>
        </div>
      </div>
    </div>

    <div class="modal-actions">
      {#if existing}
        <button type="button" class="ghost-btn danger" onclick={remove}>Delete</button>
      {/if}
      <span class="spacer"></span>
      <button type="button" class="run-btn" disabled={!canSave} onclick={save}>Save Finding</button>
    </div>
  </div>
</div>

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
    justify-content: space-between;
    margin-bottom: 14px;
  }
  .modal-head h2 {
    margin: 0;
    font: 700 18px/1 var(--fdisp);
    color: var(--ink);
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
  }
  .icon-btn:hover {
    color: var(--ink);
    border-color: var(--accent);
  }

  .ff-row {
    display: flex;
    gap: 12px;
    margin-bottom: 12px;
    flex-wrap: wrap;
  }
  .ff-col {
    display: flex;
    flex-direction: column;
    gap: 5px;
    flex: 1;
    min-width: 160px;
  }
  .ff-col.ff-wide {
    flex: 2;
  }
  .ff-col.ff-full {
    flex: 1 1 100%;
  }
  .ff-col.ff-score {
    max-width: 100px;
    flex: 0 0 auto;
  }
  .ff-col label {
    font: 700 11px/1 var(--fui);
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .lbl-req {
    color: var(--crit);
  }
  .lbl-note {
    text-transform: none;
    letter-spacing: 0;
    color: var(--dim);
    font-weight: 400;
  }
  .ff-col input,
  .ff-col select,
  .ff-col textarea {
    width: 100%;
    background: rgba(0, 0, 0, 0.28);
    border: var(--bordw) solid var(--edge2);
    border-radius: calc(var(--radius) - 6px);
    color: var(--ink);
    padding: 9px 11px;
    outline: none;
    font: 500 13px/1.4 var(--fui);
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .ff-col input:focus,
  .ff-col select:focus,
  .ff-col textarea:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--glowa);
  }
  .ff-col select {
    cursor: pointer;
  }
  .ff-col select option {
    background: var(--bg);
  }
  .ff-col textarea {
    resize: vertical;
    line-height: 1.55;
  }
  .ff-col textarea.mono,
  .ff-col input.mono {
    font-family: var(--fmono);
    font-size: 12px;
  }

  .modal-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 18px;
    padding-top: 14px;
    border-top: var(--bordw) solid var(--edge);
  }
  .modal-actions .spacer {
    flex: 1;
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
  }
  .ghost-btn:hover {
    color: var(--ink);
    border-color: var(--accent);
  }
  .ghost-btn.danger {
    color: var(--crit);
    border-color: color-mix(in srgb, var(--crit) 35%, transparent);
  }
  .ghost-btn.danger:hover {
    border-color: var(--crit);
    background: color-mix(in srgb, var(--crit) 10%, transparent);
  }
  .run-btn {
    font: 700 13px/1 var(--fdisp);
    /* Dark-on-amber text — same hex TargetsPanel.svelte already uses for its
       .badge on var(--accent); reused rather than adding a new orphan hex. */
    color: #221a06;
    background: var(--accent);
    border: none;
    border-radius: calc(var(--radius) - 6px);
    padding: 10px 16px;
    cursor: pointer;
  }
  .run-btn:hover {
    filter: brightness(1.09);
  }
  .run-btn:active {
    transform: translateY(1px);
  }
  .run-btn:disabled {
    background: var(--panel3);
    color: var(--dim);
    cursor: default;
  }
</style>
