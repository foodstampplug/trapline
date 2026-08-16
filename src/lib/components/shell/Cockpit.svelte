<script lang="ts">
  import TopBar from './TopBar.svelte';
  import AppRail from './AppRail.svelte';
  import TargetsPanel from './TargetsPanel.svelte';
  import RightDock from './RightDock.svelte';
  import StatusBar from './StatusBar.svelte';
  import Terminal from '../views/Terminal.svelte';
  import Launcher from '../Launcher.svelte';
  import CommandBar from '../CommandBar.svelte';
  import type { Template } from '$lib/data/templates';

  type ViewId = 'output' | 'map' | 'feed';

  const viewMeta: Record<ViewId, string> = {
    output: 'shodan enrich · 42 hosts · admin: 3 CVEs',
    map: '18 nodes · CVE + new route flagged',
    feed: 'live · recon · watch · leakcheck · shodan',
  };

  let activeView = $state<ViewId>('output');
  let launcherOpen = $state(false);
  let commandBar: CommandBar;

  function handlePick(t: Template & { cat: string }): void {
    commandBar?.loadTemplate(t);
    launcherOpen = false;
  }

  function handleRun(): void {
    activeView = 'output';
  }
</script>

<div class="cockpit">
  <TopBar onOpenLauncher={() => (launcherOpen = true)} />
  <div class="body">
    <AppRail />
    <TargetsPanel />
    <main class="main">
      <div class="views">
        <button class:on={activeView === 'output'} onclick={() => (activeView = 'output')}>
          ▚ Terminal
        </button>
        <button class:on={activeView === 'map'} onclick={() => (activeView = 'map')}>
          ⬡ Surface Map
        </button>
        <button class:on={activeView === 'feed'} onclick={() => (activeView = 'feed')}>
          ≋ Activity
        </button>
        <span class="meta">{viewMeta[activeView]}</span>
      </div>

      <CommandBar bind:this={commandBar} onRun={handleRun} />

      <div class="view v-output" class:on={activeView === 'output'}>
        <Terminal />
      </div>

      <div class="view v-map" class:on={activeView === 'map'}>
        <svg preserveAspectRatio="none">
          <line x1="44%" y1="52%" x2="22%" y2="28%" />
          <line x1="44%" y1="52%" x2="24%" y2="76%" />
          <line x1="44%" y1="52%" x2="68%" y2="30%" class="hot" />
          <line x1="44%" y1="52%" x2="70%" y2="72%" />
          <line x1="68%" y1="30%" x2="86%" y2="15%" class="hot" />
        </svg>
        <div class="node center" style="left:44%;top:52%"><span class="d"></span>app.acme.com</div>
        <div class="node" style="left:22%;top:28%"><span class="d"></span>api :443</div>
        <div class="node" style="left:24%;top:76%"><span class="d"></span>assets</div>
        <div class="node flag" style="left:68%;top:30%">
          <span class="d"></span>admin <span class="sev crit">CVE</span>
        </div>
        <div class="node" style="left:70%;top:72%">
          <span class="d" style="background:var(--med)"></span>staging
        </div>
        <div class="node watch" style="left:86%;top:15%">
          <span class="d"></span>/api/v2/internal <span class="sev high">new</span>
        </div>
      </div>

      <div class="view v-feed" class:on={activeView === 'feed'}>
        <div class="fd-day">Today</div>
        <div class="tl">
          <div class="ev w">
            <div class="c">
              <div class="eh">
                <span class="k">👁 Watch</span><span class="sev high">high</span><span class="tm">2m</span>
              </div>
              <div class="tt">New route: <b>/api/v2/internal/export</b> <span>— not in baseline</span></div>
            </div>
          </div>
          <div class="ev f">
            <div class="c">
              <div class="eh">
                <span class="k">🔓 LeakCheck</span><span class="sev crit">crit</span><span class="tm">5m</span>
              </div>
              <div class="tt"><b>6 plaintext creds</b> for app.acme.com <span>— 3 breaches</span></div>
            </div>
          </div>
          <div class="ev r">
            <div class="c">
              <div class="eh"><span class="k">🛰 Shodan</span><span class="tm">8m</span></div>
              <div class="tt">
                admin.app.acme.com · <b>CVE-2023-38408</b> <span>— OpenSSH RCE candidate</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </main>
    <RightDock />
  </div>
  <StatusBar />
  <Launcher bind:open={launcherOpen} onPick={handlePick} />
</div>

<style>
  .cockpit {
    display: grid;
    grid-template-rows: 58px 1fr 30px;
    height: 100vh;
    color: var(--ink);
    font-family: var(--fui);
    position: relative;
    overflow: hidden;
    background:
      radial-gradient(1150px 540px at 84% -12%, rgba(255, 191, 71, 0.13), transparent 60%),
      radial-gradient(780px 470px at 4% 114%, rgba(122, 162, 255, 0.1), transparent 60%),
      var(--bg);
  }

  .body {
    display: grid;
    grid-template-columns: 52px 166px 1fr 292px;
    min-height: 0;
    position: relative;
    z-index: 2;
  }

  .main {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .views {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 8px 10px;
    border-bottom: var(--bordw) solid var(--edge);
    background: rgba(255, 255, 255, 0.022);
    backdrop-filter: blur(14px);
  }
  .views button {
    font: 700 11px/1 var(--fdisp);
    color: var(--muted);
    background: transparent;
    border: 1px solid transparent;
    border-radius: calc(var(--radius) - 4px);
    padding: 8px 12px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .views button.on {
    color: var(--ink);
    background: rgba(255, 255, 255, 0.05);
    border-color: var(--edge2);
  }
  .views .meta {
    margin-left: auto;
    font: 600 10.5px/1 var(--fmono);
    color: var(--dim);
  }

  .view {
    flex: 1;
    min-height: 0;
    position: relative;
    display: none;
  }
  .view.on {
    display: block;
  }

  .v-map {
    overflow: hidden;
    background:
      radial-gradient(900px 520px at 42% 44%, rgba(122, 162, 255, 0.08), transparent 60%),
      repeating-linear-gradient(0deg, rgba(255, 255, 255, 0.03) 0 1px, transparent 1px 34px),
      repeating-linear-gradient(90deg, rgba(255, 255, 255, 0.03) 0 1px, transparent 1px 34px);
  }
  .v-map svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .v-map svg line {
    stroke: var(--edge2);
    stroke-width: 1.5;
  }
  .v-map svg line.hot {
    stroke: var(--accent);
    stroke-dasharray: 4 3;
  }
  .node {
    position: absolute;
    transform: translate(-50%, -50%);
    background: rgba(20, 22, 28, 0.72);
    backdrop-filter: blur(12px);
    border: 1px solid var(--edge2);
    border-radius: calc(var(--radius) - 2px);
    padding: 9px 12px;
    font: 600 11.5px/1 var(--fmono);
    white-space: nowrap;
    box-shadow: var(--shadow);
    display: flex;
    align-items: center;
    gap: 7px;
    z-index: 2;
  }
  .node .d {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--ok);
  }
  .node.center {
    border-color: var(--accent2);
    font-weight: 700;
    font-size: 12.5px;
  }
  .node.center .d {
    background: var(--accent2);
    box-shadow: 0 0 8px var(--accent2);
  }
  .node.flag {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(255, 191, 71, 0.18), var(--shadow);
  }
  .node.flag .d {
    background: var(--accent);
    box-shadow: 0 0 8px var(--accent);
  }
  .node.watch {
    border-color: color-mix(in srgb, var(--high) 55%, transparent);
  }
  .node.watch .d {
    background: var(--high);
    animation: bl 1.4s infinite;
  }

  .v-feed {
    overflow: auto;
    padding: 6px 16px 16px;
  }
  .fd-day {
    font: 700 10px/1 var(--fmono);
    letter-spacing: 0.11em;
    text-transform: uppercase;
    color: var(--dim);
    margin: 14px 0 2px 30px;
  }
  .tl {
    position: relative;
    padding-left: 30px;
  }
  .tl::before {
    content: '';
    position: absolute;
    left: 9px;
    top: 6px;
    bottom: 6px;
    width: 2px;
    background: var(--edge);
  }
  .ev {
    position: relative;
    margin: 12px 0;
  }
  .ev::before {
    content: '';
    position: absolute;
    left: -25px;
    top: 15px;
    width: 11px;
    height: 11px;
    border-radius: 50%;
    background: var(--muted);
    border: 3px solid var(--bg);
    box-shadow: 0 0 0 1px var(--edge);
  }
  .ev.w::before {
    background: var(--high);
  }
  .ev.r::before {
    background: var(--accent2);
  }
  .ev.f::before {
    background: var(--accent);
  }
  .ev .c {
    background: rgba(255, 255, 255, 0.03);
    backdrop-filter: blur(14px);
    border: 1px solid var(--edge);
    border-radius: calc(var(--radius) - 2px);
    padding: 11px 14px;
  }
  .ev.w .c {
    border-color: color-mix(in srgb, var(--high) 38%, transparent);
    background: color-mix(in srgb, var(--high) 8%, rgba(255, 255, 255, 0.03));
  }
  .ev .eh {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 5px;
  }
  .ev .eh .k {
    font: 700 9.5px/1 var(--fmono);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .ev .eh .tm {
    margin-left: auto;
    font: 500 10px/1 var(--fmono);
    color: var(--dim);
  }
  .ev .tt {
    font: 600 12.5px/1.45 var(--fui);
  }
  .ev .tt b {
    font-family: var(--fmono);
  }
  .ev .tt span {
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
  .flag {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    color: #ffe6b0;
    border-radius: 3px;
    padding: 1px 5px;
    font-weight: 700;
  }

  @keyframes bl {
    50% {
      opacity: 0;
    }
  }
</style>
