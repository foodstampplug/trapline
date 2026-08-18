<script lang="ts">
  import { surface } from '$lib/stores/surface';
  import type { Scope, SurfaceNode } from '$lib/stores/surface';
  import { enrichment, applyShodanHost } from '$lib/stores/enrichment';
  import { shodanHost, shodanDomain } from '$lib/bridge';
  import { toast } from '$lib/stores/toasts';

  let { onCreateFinding }: { onCreateFinding?: (host: string) => void } = $props();

  // Display-time noise filter. `surface.ts`'s host regex is a general-purpose
  // "find dotted hostnames in text" matcher — it has no way to tell
  // `admin.acme.com` from a file mention like `config.json` or `bundle.js`
  // inside recon output, so those slip through as pseudo-hosts. Rather than
  // teach the store about file extensions (and risk under/over-filtering
  // upstream data other views may want raw), the noise call is made here,
  // display-only: drop any node whose final dot-segment is a common file
  // extension, and drop a scope entirely if that empties it.
  const NOISE_EXTENSIONS = new Set([
    'json', 'js', 'mjs', 'cjs', 'ts', 'css', 'html', 'htm', 'xml', 'yml', 'yaml', 'txt', 'md',
    'php', 'asp', 'aspx', 'jsp', 'png', 'jpg', 'jpeg', 'gif', 'svg', 'ico', 'map', 'env', 'lock',
    'cfg', 'conf', 'ini', 'sh', 'ps1', 'py',
  ]);

  function isNoiseHost(host: string): boolean {
    const last = host.split('.').pop()?.toLowerCase() ?? '';
    return NOISE_EXTENSIONS.has(last);
  }

  function filterScopes(scopes: Scope[]): Scope[] {
    return scopes
      .map((s) => ({ ...s, nodes: s.nodes.filter((n) => !isNoiseHost(n.host)) }))
      .filter((s) => s.nodes.length > 0);
  }

  let scopes = $derived(filterScopes($surface));

  // Active scope defaults to the first one; a small selector appears only
  // when recon has mapped more than one registrable domain.
  let selectedIdx = $state(0);
  let activeScope: Scope | null = $derived(
    scopes.length > 0 ? (scopes[Math.min(selectedIdx, scopes.length - 1)] ?? null) : null
  );

  let inspected = $state<SurfaceNode | null>(null);

  function selectScope(i: number): void {
    selectedIdx = i;
    inspected = null;
  }

  // Radial layout — children evenly spaced on a circle around the center
  // node, starting straight up and going clockwise.
  const RADIUS = 34;
  function angle(i: number, n: number): number {
    return (i / n) * Math.PI * 2 - Math.PI / 2;
  }
  function posX(i: number, n: number): number {
    return 50 + RADIUS * Math.cos(angle(i, n));
  }
  function posY(i: number, n: number): number {
    return 50 + RADIUS * Math.sin(angle(i, n));
  }

  /** Flagged nodes get a severity class (`sev-critical`/`sev-high`/everything
   * else buckets to `sev-medium`); unflagged nodes get no class and fall
   * back to the base `.node` styling (edge2 border, ok dot). */
  function severityClass(node: SurfaceNode): string {
    if (!node.flagged) return '';
    const sev = node.severity ?? 'medium';
    return sev === 'critical' || sev === 'high' ? `sev-${sev}` : 'sev-medium';
  }

  function openNode(node: SurfaceNode): void {
    inspected = node;
  }

  function closeInspector(): void {
    inspected = null;
  }

  function createFinding(): void {
    if (!inspected) return;
    onCreateFinding?.(inspected.host);
  }

  // Host-vs-IP heuristic for the Enrich action below. Deliberately naive
  // (dotted-quad shape only, no octet-range validation) — recon-derived
  // node hosts are either plain hostnames or literal IPv4 strings, never
  // anything more exotic than that.
  const IPV4_RE = /^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$/;
  function isIpHost(host: string): boolean {
    return IPV4_RE.test(host);
  }

  /** CVE-count -> severity-tinted badge class, using the existing
   * --high/--crit tokens (same pair severityClass() above draws from):
   * any CVE at all is at least a "high" concern, 2+ escalates to "crit". */
  function cveBadgeClass(count: number): string {
    return count >= 2 ? 'crit' : 'high';
  }

  let enriching = $state(false);

  // Enrich action: IP-shaped host -> shodanHost() -> applyShodanHost() folds
  // the result into the enrichment store (an exact ShodanHost -> EnrichEntry
  // type match, same path Launcher.svelte's callIntegration() uses for its
  // shodanHost integration entry). A domain-shaped host -> shodanDomain()
  // instead — but per Task 8's confirmed resolution (see enrichment.ts /
  // Launcher.svelte comments), that result (domain/subdomains/records) is
  // NOT forced through applyShodanHost: it carries none of the required
  // ports/services/cves/org fields, and faking them would silently blank
  // out any real host enrichment already held for that host. The domain
  // lookup still runs (so a future domain-shaped surface can consume it)
  // but the node/CVE badges below only ever populate from the host path.
  async function enrichNode(): Promise<void> {
    if (!inspected || enriching) return;
    const host = inspected.host;
    enriching = true;
    try {
      if (isIpHost(host)) {
        const data = await shodanHost(host);
        applyShodanHost(host, data);
      } else {
        await shodanDomain(host);
      }
    } catch (e) {
      // Same channel as every other async action in this app (Launcher.svelte's
      // runIntegration) — only the backend's own message is ever shown. Shodan
      // network-error strings could otherwise embed the API key (reqwest's
      // Error Display includes the request URL, which carries `?key=...`);
      // integrations::shodan::fetch() scrubs the key from those errors on the
      // backend before they ever reach this catch.
      toast(e instanceof Error ? e.message : String(e), 'err');
    } finally {
      enriching = false;
    }
  }
</script>

<div class="smap">
  {#if activeScope}
    {#if scopes.length > 1}
      <div class="scope-sel">
        {#each scopes as s, i (s.domain)}
          <button type="button" class:on={i === selectedIdx} onclick={() => selectScope(i)}>{s.domain}</button>
        {/each}
      </div>
    {/if}

    <div class="graph">
      <svg preserveAspectRatio="none">
        {#each activeScope.nodes as node, i (node.host)}
          <line
            x1="50%"
            y1="50%"
            x2="{posX(i, activeScope.nodes.length)}%"
            y2="{posY(i, activeScope.nodes.length)}%"
            class:hot={node.flagged}
          />
        {/each}
      </svg>

      <div class="node center" style="left:50%;top:50%">
        <span class="d"></span>{activeScope.domain}
      </div>

      {#each activeScope.nodes as node, i (node.host)}
        {@const enrich = $enrichment.get(node.host)}
        <button
          type="button"
          class="node {severityClass(node)}"
          style="left:{posX(i, activeScope.nodes.length)}%;top:{posY(i, activeScope.nodes.length)}%"
          onclick={() => openNode(node)}
        >
          <span class="d"></span>{node.host}
          {#if enrich}
            <span class="node-badges">
              {#if enrich.ports.length > 0}
                <span class="badge-ports" title="Shodan-reported open ports">{enrich.ports.join(', ')}</span>
              {/if}
              {#if enrich.cves.length > 0}
                <span class="badge-cve {cveBadgeClass(enrich.cves.length)}" title="Shodan-reported CVEs">
                  {enrich.cves.length} CVE
                </span>
              {/if}
            </span>
          {/if}
        </button>
      {/each}

      {#if inspected}
        <div class="inspector" role="dialog" aria-label="Node inspector">
          <div class="insp-head">
            <span class="insp-host">{inspected.host}</span>
            <button type="button" class="icon-btn sm" title="Close" onclick={closeInspector}>✕</button>
          </div>
          {#if inspected.flagged}
            <div class="insp-sev {severityClass(inspected)}">{inspected.severity ?? 'flagged'}</div>
          {/if}
          <button type="button" class="insp-finding-btn" onclick={createFinding}>→ Finding</button>
          <button
            type="button"
            class="insp-enrich-btn"
            onclick={() => void enrichNode()}
            disabled={enriching}
          >
            {enriching ? 'Enriching…' : '⚑ Enrich'}
          </button>
        </div>
      {/if}
    </div>
  {:else}
    <div class="empty">Run recon to map the surface.</div>
  {/if}
</div>

<style>
  .smap {
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background:
      radial-gradient(900px 520px at 42% 44%, rgba(122, 162, 255, 0.08), transparent 60%),
      repeating-linear-gradient(0deg, rgba(255, 255, 255, 0.03) 0 1px, transparent 1px 34px),
      repeating-linear-gradient(90deg, rgba(255, 255, 255, 0.03) 0 1px, transparent 1px 34px);
  }
  .empty {
    margin: auto;
    color: var(--dim);
    font: 500 12px/1.6 var(--fui);
    padding: 8px 2px;
  }

  .scope-sel {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    padding: 8px 10px;
    border-bottom: var(--bordw) solid var(--edge);
  }
  .scope-sel button {
    font: 700 10.5px/1 var(--fmono);
    color: var(--muted);
    background: rgba(255, 255, 255, 0.03);
    border: var(--bordw) solid var(--edge);
    border-radius: calc(var(--radius) - 8px);
    padding: 6px 10px;
    cursor: pointer;
  }
  .scope-sel button.on {
    color: var(--ink);
    border-color: var(--accent2);
    background: color-mix(in srgb, var(--accent2) 14%, transparent);
  }

  .graph {
    position: relative;
    flex: 1;
    min-height: 0;
  }
  .graph svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .graph svg line {
    stroke: var(--edge2);
    stroke-width: 1.5;
  }
  .graph svg line.hot {
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
    color: var(--ink);
  }
  button.node {
    cursor: pointer;
  }
  button.node:hover {
    border-color: var(--accent2);
  }
  .node .d {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--ok);
  }
  .node-badges {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-left: 2px;
  }
  .badge-ports {
    font: 700 9px/1 var(--fmono);
    color: var(--muted);
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--edge2);
    border-radius: 4px;
    padding: 3px 5px;
    white-space: nowrap;
  }
  .badge-cve {
    font: 700 9px/1 var(--fmono);
    border: 1.5px solid currentColor;
    border-radius: 4px;
    padding: 3px 5px;
    white-space: nowrap;
  }
  .badge-cve.high {
    color: var(--high);
  }
  .badge-cve.crit {
    color: var(--crit);
  }
  .node.center {
    border-color: var(--accent2);
    font-weight: 700;
    font-size: 12.5px;
    cursor: default;
  }
  .node.center .d {
    background: var(--accent2);
    box-shadow: 0 0 8px var(--accent2);
  }
  .node.sev-critical {
    border-color: var(--crit);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--crit) 18%, transparent), var(--shadow);
  }
  .node.sev-critical .d {
    background: var(--crit);
    box-shadow: 0 0 8px var(--crit);
  }
  .node.sev-high {
    border-color: var(--high);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--high) 18%, transparent), var(--shadow);
  }
  .node.sev-high .d {
    background: var(--high);
    box-shadow: 0 0 8px var(--high);
  }
  .node.sev-medium {
    border-color: var(--med);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--med) 18%, transparent), var(--shadow);
  }
  .node.sev-medium .d {
    background: var(--med);
    box-shadow: 0 0 8px var(--med);
  }

  .inspector {
    position: absolute;
    right: 14px;
    bottom: 14px;
    z-index: 5;
    width: 220px;
    background: var(--card-glass);
    backdrop-filter: blur(20px);
    border: var(--bordw) solid var(--edge2);
    border-radius: var(--radius);
    padding: 14px;
    box-shadow: var(--shadow);
  }
  .insp-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 8px;
  }
  .insp-host {
    font: 700 12.5px/1.3 var(--fmono);
    color: var(--ink);
    word-break: break-all;
  }
  .icon-btn.sm {
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    border-radius: calc(var(--radius) - 8px);
    border: var(--bordw) solid var(--edge);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
    font-size: 11px;
    flex-shrink: 0;
  }
  .icon-btn.sm:hover {
    color: var(--ink);
    border-color: var(--accent);
  }
  .insp-sev {
    display: inline-block;
    font: 800 9.5px/1 var(--fmono);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    border: 1.5px solid currentColor;
    border-radius: 5px;
    padding: 3px 6px;
    margin-bottom: 10px;
  }
  .insp-sev.sev-critical {
    color: var(--crit);
  }
  .insp-sev.sev-high {
    color: var(--high);
  }
  .insp-sev.sev-medium {
    color: var(--med);
  }
  .insp-finding-btn {
    width: 100%;
    font: 700 12px/1 var(--fdisp);
    /* Dark-on-amber text — same hex TargetsPanel.svelte/FindingEditor.svelte
       already use for buttons on var(--accent); reused rather than a new
       orphan hex. */
    color: #221a06;
    background: var(--accent);
    border: none;
    border-radius: calc(var(--radius) - 6px);
    padding: 9px 12px;
    cursor: pointer;
  }
  .insp-finding-btn:hover {
    filter: brightness(1.09);
  }
  .insp-enrich-btn {
    width: 100%;
    margin-top: 8px;
    font: 700 12px/1 var(--fdisp);
    color: var(--ink);
    background: rgba(255, 255, 255, 0.04);
    border: var(--bordw) solid var(--edge2);
    border-radius: calc(var(--radius) - 6px);
    padding: 9px 12px;
    cursor: pointer;
  }
  .insp-enrich-btn:hover {
    border-color: var(--accent2);
  }
  .insp-enrich-btn:disabled {
    opacity: 0.6;
    cursor: default;
  }
</style>
