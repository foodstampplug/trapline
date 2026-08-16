<div class="status">
  <span class="p"></span> monitoring 1 · shodan+leakcheck linked · rate-limit 60/min
  <span class="push">4 findings · webhook linked</span>
</div>

<style>
  .status {
    display: flex;
    align-items: center;
    gap: 15px;
    padding: 0 14px;
    border-top: var(--bordw) solid var(--edge);
    background: rgba(255, 255, 255, 0.022);
    backdrop-filter: blur(14px);
    font: 600 10.5px/1 var(--fmono);
    color: var(--muted);
    position: relative;
    z-index: 3;
    height: 100%;
  }
  .status .p {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--ok);
  }
  .status .push {
    margin-left: auto;
  }
</style>
