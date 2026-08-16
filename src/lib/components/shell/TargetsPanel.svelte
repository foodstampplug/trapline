<div class="targets">
  <div class="ph">Targets <span class="c">4</span></div>
  <div class="tb">
    <div class="ti on">▾ app.acme.com</div>
    <div class="ti sub">
      api <span class="b"><span class="badge">2</span></span>
    </div>
    <div class="ti sub hot">admin ⚠</div>
    <div class="ti sub">staging</div>
    <div class="ti sub">assets</div>
    <div class="ti">▸ shop.acme.com</div>
  </div>
</div>

<style>
  .targets {
    border-right: var(--bordw) solid var(--edge);
    background: rgba(255, 255, 255, 0.02);
    backdrop-filter: blur(16px);
    display: flex;
    flex-direction: column;
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
  .badge {
    min-width: 17px;
    height: 17px;
    padding: 0 5px;
    border-radius: 9px;
    background: var(--accent);
    color: #221a06;
    font: 800 10px/17px var(--fmono);
    text-align: center;
    display: inline-block;
  }
  .targets .tb {
    padding: 7px 6px;
    overflow: auto;
    font: 600 12px/1 var(--fmono);
  }
  .targets .ti {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 7px 9px;
    border-radius: calc(var(--radius) - 6px);
    color: var(--muted);
    cursor: pointer;
  }
  .targets .ti.sub {
    padding-left: 22px;
    font-size: 11.5px;
  }
  .targets .ti.on {
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    color: var(--ink);
  }
  .targets .ti.hot {
    color: var(--accent);
  }
  .targets .ti .b {
    margin-left: auto;
  }
</style>
