<div class="right">
  <div class="wbox">
    <div class="ph" style="border:0;padding:0 0 9px">
      Watch <span class="wlive"><span class="p"></span>live</span>
    </div>
    <div class="wl">
      <span class="sev high">new</span>
      <div class="t">Route <b>/api/v2/internal/export</b><span>not in baseline</span></div>
    </div>
    <div class="wm">
      <div>Last<b>2m</b></div>
      <div>Assets<b>18</b></div>
      <div>New<b>3</b></div>
      <div>Every<b>30m</b></div>
    </div>
  </div>
  <div class="findbox">
    <div class="ph">Findings <span class="c">4</span></div>
    <div class="fl">
      <div class="fi">
        <span class="sev crit">C</span>
        <div class="t">Creds in breach dump<span>leakcheck · 6 plaintext</span></div>
        <span class="go">›</span>
      </div>
      <div class="fi">
        <span class="sev high">H</span>
        <div class="t">Admin OpenSSH CVE<span>admin.app.acme.com</span></div>
        <span class="go">›</span>
      </div>
      <div class="fi">
        <span class="sev high">H</span>
        <div class="t">Admin panel exposed<span>admin.app.acme.com</span></div>
        <span class="go">›</span>
      </div>
    </div>
  </div>
</div>

<style>
  @keyframes bl {
    50% {
      opacity: 0;
    }
  }

  .right {
    border-left: var(--bordw) solid var(--edge);
    background: rgba(255, 255, 255, 0.02);
    backdrop-filter: blur(16px);
    display: grid;
    grid-template-rows: auto 1fr;
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
  .sev {
    font: 800 8.5px/1 var(--fmono);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    border: 1.5px solid currentColor;
    border-radius: 5px;
    padding: 3px 5px;
    white-space: nowrap;
  }
  .sev.high {
    color: var(--high);
  }
  .sev.crit {
    color: var(--crit);
  }
  .wbox {
    padding: 11px;
    border-bottom: var(--bordw) solid var(--edge);
  }
  .wl {
    display: flex;
    align-items: center;
    gap: 9px;
    background: color-mix(in srgb, var(--high) 12%, rgba(255, 255, 255, 0.03));
    border: 1px solid color-mix(in srgb, var(--high) 40%, transparent);
    border-radius: calc(var(--radius) - 3px);
    padding: 10px;
  }
  .wl .t {
    font: 600 11.5px/1.35 var(--fui);
  }
  .wl .t b {
    font-family: var(--fmono);
  }
  .wl .t span {
    display: block;
    color: var(--dim);
    font: 500 10px/1 var(--fmono);
    margin-top: 2px;
  }
  .wm {
    display: flex;
    gap: 13px;
    margin-top: 10px;
  }
  .wm div {
    font: 600 10px/1.2 var(--fui);
    color: var(--muted);
  }
  .wm b {
    display: block;
    color: var(--ink);
    font: 700 14px/1.2 var(--fmono);
    margin-top: 3px;
    font-variant-numeric: tabular-nums;
  }
  .wlive {
    display: flex;
    align-items: center;
    gap: 6px;
    font: 700 9px/1 var(--fmono);
    letter-spacing: 0.1em;
    color: var(--ok);
    text-transform: uppercase;
    margin-left: auto;
  }
  .wlive .p {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--ok);
    animation: bl 1.4s infinite;
  }
  .findbox {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .fl {
    padding: 9px;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .fi {
    display: flex;
    align-items: center;
    gap: 9px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--edge);
    border-radius: calc(var(--radius) - 3px);
    padding: 9px 10px;
    cursor: pointer;
  }
  .fi .t {
    font: 600 11.5px/1.35 var(--fui);
  }
  .fi .t span {
    display: block;
    color: var(--dim);
    font: 500 10px/1 var(--fmono);
    margin-top: 2px;
  }
  .fi .go {
    margin-left: auto;
    color: var(--dim);
  }
</style>
