<script lang="ts">
  // Shows a generated report as plain text — never {@html}/innerHTML, since
  // the markdown embeds untrusted finding content (title/summary/evidence/etc).
  let {
    markdown,
    onClose,
  }: {
    markdown: string;
    onClose?: () => void;
  } = $props();

  let copied = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | undefined;

  async function copy(): Promise<void> {
    try {
      await navigator.clipboard.writeText(markdown);
      copied = true;
      clearTimeout(copyTimer);
      copyTimer = setTimeout(() => (copied = false), 1500);
    } catch (e) {
      console.error(e);
    }
  }

  function onWindowKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape') {
      e.preventDefault();
      onClose?.();
    }
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

<div class="overlay">
  <button type="button" class="backdrop" aria-label="Close report" onclick={() => onClose?.()}></button>
  <div class="modal-card wide" role="dialog" aria-modal="true" aria-label="HackerOne / Bugcrowd report">
    <div class="modal-head">
      <h2>HackerOne / Bugcrowd Report</h2>
      <button type="button" class="icon-btn sm" title="Close" onclick={() => onClose?.()}>✕</button>
    </div>
    <p class="modal-sub">
      Review and fill in any <b>[brackets]</b> before submitting. The impact section is where payout is decided —
      make it count.
    </p>
    <pre class="report">{markdown}</pre>
    <div class="modal-actions">
      <span class="spacer"></span>
      <button type="button" class="run-btn" onclick={copy}>{copied ? 'Copied ✓' : 'Copy to Clipboard'}</button>
      <button type="button" class="ghost-btn" onclick={() => onClose?.()}>Close</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 60;
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
    margin-bottom: 8px;
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
  .modal-sub {
    font: 500 12px/1.5 var(--fui);
    color: var(--dim);
    margin: 0 0 14px;
  }
  .modal-sub b {
    color: var(--muted);
  }
  .report {
    background: rgba(0, 0, 0, 0.28);
    border: var(--bordw) solid var(--edge2);
    border-radius: calc(var(--radius) - 6px);
    padding: 16px;
    color: var(--ink);
    font: 500 12px/1.6 var(--fmono);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 48vh;
    overflow-y: auto;
    margin: 0;
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
  .run-btn {
    font: 700 13px/1 var(--fdisp);
    /* Dark-on-amber text — same hex TargetsPanel.svelte/FindingEditor.svelte
       already use for buttons on var(--accent); reused, not a new hex. */
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
</style>
