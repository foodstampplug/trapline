<script lang="ts">
  import { findings } from '$lib/stores/findings';
  import { loot } from '$lib/stores/loot';
  import { watch } from '$lib/stores/watch';

  let {
    onOpenPlaybook,
    onOpenTools,
    onOpenFindings,
    onOpenLoot,
    onOpenSettings,
    onOpenDeck,
  }: {
    onOpenPlaybook?: () => void;
    onOpenTools?: () => void;
    onOpenFindings?: () => void;
    onOpenLoot?: () => void;
    onOpenSettings?: () => void;
    onOpenDeck?: () => void;
  } = $props();
</script>

<div class="rail">
  <button class="on" title="Recon">▚</button>
  <button title="Playbook" onclick={() => onOpenPlaybook?.()}>📑</button>
  <button title="Tools" onclick={() => onOpenTools?.()}>🧰</button>
  <button title="Watch" class:live={$watch.running}
    >👁{#if $watch.running}<span class="dt"></span>{/if}</button
  >
  <button title="Deck" onclick={() => onOpenDeck?.()}>📡</button>
  <button title="Loot" onclick={() => onOpenLoot?.()}
    >💰{#if $loot.length}<span class="count">{$loot.length}</span>{/if}</button
  >
  <button title="Findings" onclick={() => onOpenFindings?.()}
    >🐛{#if $findings.length}<span class="count">{$findings.length}</span>{/if}</button
  >
  <button class="sp" title="Settings" onclick={() => onOpenSettings?.()}>⚙</button>
</div>

<style>
  .rail {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    padding: 12px 0;
    border-right: var(--bordw) solid var(--edge);
    background: rgba(255, 255, 255, 0.02);
    backdrop-filter: blur(16px);
  }
  .rail button {
    width: 36px;
    height: 36px;
    border-radius: calc(var(--radius) - 3px);
    display: grid;
    place-items: center;
    font-size: 16px;
    color: var(--muted);
    position: relative;
    cursor: pointer;
    background: transparent;
    border: none;
    font-family: inherit;
  }
  .rail button.on {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    color: var(--ink);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 40%, transparent);
  }
  .rail button.live {
    color: var(--ok);
  }
  .rail button .dt {
    position: absolute;
    top: 5px;
    right: 6px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--high);
  }
  .rail button .count {
    position: absolute;
    top: -3px;
    right: -3px;
    min-width: 14px;
    height: 14px;
    padding: 0 3px;
    border-radius: 999px;
    background: var(--accent);
    /* Dark-on-amber text — same established on-accent pattern as
       FindingsPanel.svelte/Loot.svelte's .run-btn (#221a06 on var(--accent)),
       reused here, not a new hex. */
    color: #221a06;
    font: 700 10px/14px var(--fui);
    text-align: center;
  }
  .rail .sp {
    margin-top: auto;
  }
</style>
