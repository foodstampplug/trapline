<script lang="ts">
  // Bottom-right toast stack — the app's only user-facing feedback channel
  // for async success/error branches (Discord sends, config saves, run
  // kickoffs, clipboard copies, deck start/stop) that would otherwise just
  // log to the console and leave the user guessing. Mounted once at the top
  // of Cockpit.svelte; every call site just imports `toast(...)` from
  // $lib/stores/toasts. Each card auto-dismisses via the store's own 3.5s
  // timer, or click-to-dismiss.
  import { toasts, dismiss } from '$lib/stores/toasts';
</script>

<div class="toaster" role="status" aria-live="polite">
  {#each $toasts as t (t.id)}
    <button type="button" class="toast {t.kind}" onclick={() => dismiss(t.id)}>
      {t.message}
    </button>
  {/each}
</div>

<style>
  .toaster {
    position: fixed;
    right: 16px;
    bottom: 16px;
    z-index: 100;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: min(92vw, 340px);
    pointer-events: none;
  }
  .toast {
    display: block;
    width: 100%;
    text-align: left;
    background: rgba(12, 13, 16, 0.92);
    backdrop-filter: blur(20px);
    border: var(--bordw) solid var(--edge2);
    border-left: 3px solid var(--muted);
    border-radius: calc(var(--radius) - 6px);
    padding: 10px 14px;
    color: var(--ink);
    font: 600 12.5px/1.4 var(--fui);
    cursor: pointer;
    box-shadow: var(--shadow);
    pointer-events: auto;
    animation: toast-in 0.18s ease-out;
  }
  .toast.ok {
    border-left-color: var(--ok);
  }
  .toast.err {
    border-left-color: var(--crit);
  }
  .toast.info {
    border-left-color: var(--muted);
  }

  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  /* app.css already sets `* { animation: none !important }` globally under
     reduced motion, so this is belt-and-suspenders — kept local + explicit
     so the guard is obvious right next to the motion it covers. */
  @media (prefers-reduced-motion: reduce) {
    .toast {
      animation: none;
    }
  }
</style>
