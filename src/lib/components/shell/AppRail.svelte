<div class="rail">
  <button class="on" title="Recon">▚</button>
  <button title="Watch">👁<span class="dt"></span></button>
  <button title="Deck">📡</button>
  <button title="Findings">🐛</button>
  <button class="sp" title="Settings">⚙</button>
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
  .rail button .dt {
    position: absolute;
    top: 5px;
    right: 6px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--high);
  }
  .rail .sp {
    margin-top: auto;
  }
</style>
