<div class="top">
  <span class="bd"><span class="mk">◎</span> TRAPLINE</span>
  <div class="launch">
    <span class="k">⌘K</span>
    <span class="q">shodan domain app.acme.com <span class="mut">· enrich · leakcheck</span></span>
    <span class="cur"></span>
  </div>
  <span class="acts"><b>🧰</b><b>💰</b><b>⚙</b></span>
</div>

<style>
  @keyframes bl {
    50% {
      opacity: 0;
    }
  }

  .top {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 0 14px;
    border-bottom: var(--bordw) solid var(--edge);
    background: rgba(255, 255, 255, 0.022);
    backdrop-filter: blur(18px);
    position: relative;
    z-index: 3;
    height: 100%;
  }
  .top .bd {
    display: flex;
    align-items: center;
    gap: 8px;
    font: 700 13px/1 var(--fdisp);
    letter-spacing: 0.14em;
    white-space: nowrap;
  }
  .top .bd .mk {
    color: var(--accent);
    font-size: 16px;
    text-shadow: 0 0 10px rgba(255, 191, 71, 0.5);
  }
  .launch {
    flex: 1;
    max-width: 560px;
    margin: 0 auto;
    display: flex;
    align-items: center;
    gap: 11px;
    background: rgba(0, 0, 0, 0.28);
    border: 1px solid var(--edge2);
    border-radius: calc(var(--radius) - 1px);
    padding: 10px 13px;
    box-shadow: 0 0 0 3px var(--glowa);
  }
  .launch .k {
    font: 700 10px/1 var(--fmono);
    color: var(--dim);
    border: 1px solid var(--edge2);
    border-radius: 5px;
    padding: 4px 6px;
  }
  .launch .q {
    flex: 1;
    font: 500 13.5px/1 var(--fmono);
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .launch .q .mut {
    color: var(--dim);
  }
  .launch .cur {
    width: 8px;
    height: 16px;
    background: var(--accent);
    box-shadow: 0 0 10px rgba(255, 191, 71, 0.7);
    animation: bl 1.1s steps(1) infinite;
  }
  .top .acts {
    display: flex;
    gap: 7px;
  }
  .top .acts b {
    width: 32px;
    height: 32px;
    border-radius: calc(var(--radius) - 4px);
    border: 1px solid var(--edge);
    background: rgba(0, 0, 0, 0.2);
    display: grid;
    place-items: center;
    font-size: 14px;
    font-weight: normal;
    color: var(--muted);
  }
</style>
