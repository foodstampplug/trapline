<script lang="ts">
  import { onMount } from 'svelte';
  import TopBar from './TopBar.svelte';
  import AppRail from './AppRail.svelte';
  import TargetsPanel from './TargetsPanel.svelte';
  import RightDock from './RightDock.svelte';
  import StatusBar from './StatusBar.svelte';
  import Terminal from '../views/Terminal.svelte';
  import SurfaceMap from '../views/SurfaceMap.svelte';
  import ActivityFeed from '../views/ActivityFeed.svelte';
  import Launcher from '../Launcher.svelte';
  import CommandBar from '../CommandBar.svelte';
  import Playbook from '../Playbook.svelte';
  import Tools from '../Tools.svelte';
  import FindingsPanel from '../FindingsPanel.svelte';
  import FindingEditor from '../FindingEditor.svelte';
  import Loot from '../Loot.svelte';
  import Settings from '../Settings.svelte';
  import Deck from '../Deck.svelte';
  import Toaster from '../Toaster.svelte';
  import { loadTools } from '$lib/stores/tools';
  import { loadFindings, newFinding } from '$lib/stores/findings';
  import { initWatch } from '$lib/stores/watch';
  import type { Finding } from '$lib/types';
  import type { Template } from '$lib/data/templates';

  type ViewId = 'output' | 'map' | 'feed';

  const viewMeta: Record<ViewId, string> = {
    output: 'shodan enrich · 42 hosts · admin: 3 CVEs',
    map: '18 nodes · CVE + new route flagged',
    feed: 'live · recon · watch · leakcheck · shodan',
  };

  let activeView = $state<ViewId>('output');
  let launcherOpen = $state(false);
  let playbookOpen = $state(false);
  let toolsOpen = $state(false);
  let findingsOpen = $state(false);
  let lootOpen = $state(false);
  let settingsOpen = $state(false);
  let deckOpen = $state(false);
  let commandBar: CommandBar;

  // Surface Map's node → finding path: a separate, top-level FindingEditor
  // host (mirrors FindingsPanel's editor-overlay pattern) so opening it from
  // a map node doesn't require FindingsPanel itself to be open.
  let mapFinding = $state<Finding | null>(null);

  function openFindingForHost(host: string): void {
    mapFinding = { ...newFinding(), endpoint: 'https://' + host };
  }

  function closeMapFinding(): void {
    mapFinding = null;
  }

  // Shared load-into-command-bar path — both the ⌘K launcher (T5) and the
  // Playbook drawer (T6) pick a template through this same function, they
  // just differ in which panel they close afterward.
  function loadPicked(t: Template & { cat: string }): void {
    commandBar?.loadTemplate(t);
  }

  function handlePick(t: Template & { cat: string }): void {
    loadPicked(t);
    launcherOpen = false;
  }

  function handlePlaybookPick(t: Template & { cat: string }): void {
    loadPicked(t);
    playbookOpen = false;
  }

  function handleRun(): void {
    activeView = 'output';
  }

  onMount(() => {
    void loadTools();
    void loadFindings();
    void initWatch();
  });
</script>

<div class="cockpit">
  <TopBar onOpenLauncher={() => (launcherOpen = true)} />
  <div class="body">
    <AppRail
      onOpenPlaybook={() => (playbookOpen = true)}
      onOpenTools={() => (toolsOpen = true)}
      onOpenFindings={() => (findingsOpen = true)}
      onOpenLoot={() => (lootOpen = true)}
      onOpenSettings={() => (settingsOpen = true)}
      onOpenDeck={() => (deckOpen = true)}
    />
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
        <SurfaceMap onCreateFinding={openFindingForHost} />
      </div>

      <div class="view v-feed" class:on={activeView === 'feed'}>
        <ActivityFeed />
      </div>
    </main>
    <RightDock />
  </div>
  <StatusBar />
  <Toaster />
  <Launcher bind:open={launcherOpen} onPick={handlePick} />
  <Playbook bind:open={playbookOpen} onPick={handlePlaybookPick} />
  <Tools bind:open={toolsOpen} />
  <FindingsPanel bind:open={findingsOpen} />
  <Loot bind:open={lootOpen} />
  <Settings bind:open={settingsOpen} />
  <Deck bind:open={deckOpen} />
  {#if mapFinding}
    {#key mapFinding.id}
      <FindingEditor finding={mapFinding} existing={false} onSaved={closeMapFinding} onClose={closeMapFinding} />
    {/key}
  {/if}
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

  .v-feed {
    overflow: auto;
    padding: 6px 16px 16px;
  }
</style>
