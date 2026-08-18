<script lang="ts">
  // Result card for the five ⌘K integration commands (Shodan host/domain/
  // search, LeakCheck domain/email) — Launcher.svelte opens this instead of
  // streaming through Terminal.svelte, since these call the Shodan/LeakCheck
  // bridge (src-tauri/src/integrations/commands.rs) once and get a single
  // JSON snapshot back rather than a running process.
  //
  // Everything below renders via plain Svelte text bindings — never
  // {@html}/innerHTML — since Shodan/LeakCheck values are untrusted, remote
  // strings (org names, breach source names, emails, passwords, etc).
  // The LeakCheck card shows full per-row intel in a table (email · username ·
  // password · phone · name · source · date). Plaintext PASSWORDS are shown at
  // the user's explicit request (authorized use), hidden behind a reveal toggle
  // by default; they are live/in-memory only — never persisted to disk (the
  // finding + the watch.db cache both strip them; see leakcheck.rs).
  import type { IntegrationCardData } from '$lib/data/integrations';

  let {
    card,
    arg = '',
    onClose,
  }: {
    card: IntegrationCardData;
    arg?: string;
    onClose?: () => void;
  } = $props();

  const TITLES: Record<IntegrationCardData['kind'], string> = {
    shodanHost: 'Shodan · Host',
    shodanDomain: 'Shodan · Domain',
    shodanSearch: 'Shodan · Search',
    leak: 'LeakCheck',
  };

  const passwordCount = $derived(
    card.kind === 'leak' ? card.data.results.filter((r) => r.passwordPresent).length : 0
  );

  // Breach cards title by provider (LeakCheck/Snusbase/DeHashed/LeakRadar).
  const cardTitle = $derived(card.kind === 'leak' ? card.provider : TITLES[card.kind]);

  // All data (incl. plaintext passwords) is shown revealed by default, per the
  // user's request; the toggle can re-mask the password column when needed.
  let revealPw = $state(true);

  function close(): void {
    onClose?.();
  }

  function onWindowKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape') {
      e.preventDefault();
      close();
    }
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

<div class="overlay">
  <button type="button" class="backdrop" aria-label="Close result card" onclick={close}></button>
  <div class="card" class:wide={card.kind === 'leak'} role="dialog" aria-modal="true" aria-label={cardTitle}>
    <div class="head">
      <h2>{cardTitle}</h2>
      {#if arg}<span class="arg">{arg}</span>{/if}
      <button type="button" class="icon-btn" title="Close" onclick={close}>✕</button>
    </div>

    <div class="body">
      {#if card.kind === 'shodanHost'}
        <div class="row2">
          <div class="kv"><span class="k">IP</span><span class="v mono">{card.data.ip}</span></div>
          <div class="kv"><span class="k">Org</span><span class="v">{card.data.org || '—'}</span></div>
        </div>
        {#if card.data.hostnames.length > 0}
          <div class="section">
            <div class="sh">Hostnames <span class="c">{card.data.hostnames.length}</span></div>
            <div class="chips">
              {#each card.data.hostnames as h (h)}<span class="chip">{h}</span>{/each}
            </div>
          </div>
        {/if}
        <div class="section">
          <div class="sh">Ports <span class="c">{card.data.ports.length}</span></div>
          {#if card.data.ports.length === 0}
            <div class="none">none found</div>
          {:else}
            <div class="chips">
              {#each card.data.ports as p (p)}<span class="chip mono">{p}</span>{/each}
            </div>
          {/if}
        </div>
        {#if card.data.services.length > 0}
          <div class="section">
            <div class="sh">Services <span class="c">{card.data.services.length}</span></div>
            <div class="list">
              {#each card.data.services as s (s.port + '|' + s.product)}
                <div class="li">
                  <span class="mono">{s.port}</span>
                  <span>{s.product}</span>
                  <span class="dim">{s.version}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}
        <div class="section">
          <div class="sh">CVEs <span class="c">{card.data.cves.length}</span></div>
          {#if card.data.cves.length === 0}
            <div class="none">none found</div>
          {:else}
            <div class="chips">
              {#each card.data.cves as c (c)}<span class="chip cve">{c}</span>{/each}
            </div>
          {/if}
        </div>
      {:else if card.kind === 'shodanDomain'}
        <div class="kv"><span class="k">Domain</span><span class="v mono">{card.data.domain}</span></div>
        <div class="section">
          <div class="sh">Subdomains <span class="c">{card.data.subdomains.length}</span></div>
          {#if card.data.subdomains.length === 0}
            <div class="none">none found</div>
          {:else}
            <div class="chips">
              {#each card.data.subdomains as s (s)}<span class="chip">{s}</span>{/each}
            </div>
          {/if}
        </div>
        <div class="section">
          <div class="sh">Records <span class="c">{card.data.records.length}</span></div>
          {#if card.data.records.length === 0}
            <div class="none">none found</div>
          {:else}
            <div class="list">
              {#each card.data.records as r (r.kind + '|' + r.value)}
                <div class="li"><span class="tag">{r.kind}</span><span class="mono">{r.value}</span></div>
              {/each}
            </div>
          {/if}
        </div>
      {:else if card.kind === 'shodanSearch'}
        <div class="kv"><span class="k">Total</span><span class="v mono">{card.data.total}</span></div>
        <div class="section">
          <div class="sh">Matches <span class="c">{card.data.matches.length}</span></div>
          {#if card.data.matches.length === 0}
            <div class="none">no matches</div>
          {:else}
            <div class="list">
              {#each card.data.matches as m (m.ip + ':' + m.port)}
                <div class="li match">
                  <div class="mrow">
                    <span class="mono">{m.ip}:{m.port}</span>
                    <span class="dim">{m.org || '—'}</span>
                    <span class="dim">{m.product || '—'}</span>
                  </div>
                  {#if m.cves.length > 0}
                    <div class="chips">
                      {#each m.cves as c (c)}<span class="chip cve">{c}</span>{/each}
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {:else if card.kind === 'leak'}
        <div class="warn">⚠️ Authorized use only — third-party breach credentials, shown in cleartext.</div>
        <div class="row2">
          <div class="kv"><span class="k">Found</span><span class="v mono">{card.data.found}</span></div>
          {#if passwordCount > 0}
            <div class="kv"><span class="k">Passwords</span><span class="v">🔓 {passwordCount} cleartext</span></div>
          {/if}
        </div>
        <div class="section">
          <div class="sh">Sources <span class="c">{card.data.sources.length}</span></div>
          {#if card.data.sources.length === 0}
            <div class="none">none found</div>
          {:else}
            <div class="list">
              {#each card.data.sources as s (s.name + '|' + s.date)}
                <div class="li"><span>{s.name}</span><span class="dim mono">{s.date}</span></div>
              {/each}
            </div>
          {/if}
        </div>
        <div class="section">
          <div class="sh">
            Results <span class="c">{card.data.results.length}</span>
            {#if passwordCount > 0}
              <button type="button" class="reveal" onclick={() => (revealPw = !revealPw)}>
                {revealPw ? '🙈 hide passwords' : '👁 reveal passwords'}
              </button>
            {/if}
          </div>
          {#if card.data.results.length === 0}
            <div class="none">no rows</div>
          {:else}
            <div class="tablewrap">
              <table class="leaktable">
                <thead>
                  <tr>
                    <th>Email</th><th>Username</th><th>Password</th><th>Hash</th><th>Phone</th>
                    <th>IP</th><th>Name</th><th>Source</th><th>Date</th>
                  </tr>
                </thead>
                <tbody>
                  {#each card.data.results as r, i (r.email + '|' + r.source + '|' + i)}
                    <tr>
                      <td class="mono">{r.email || '—'}</td>
                      <td>{r.username || '—'}</td>
                      <td class="pwcell">
                        {#if r.passwordPresent}
                          <span class="mono pwval" class:masked={!revealPw}>{revealPw ? r.password : '••••••'}</span>
                        {:else}—{/if}
                      </td>
                      <td class="mono hashcell">{r.hash || '—'}</td>
                      <td class="mono">{r.phone || '—'}</td>
                      <td class="mono">{r.ip || '—'}</td>
                      <td>{r.name || '—'}</td>
                      <td>{r.source || '—'}</td>
                      <td class="dim mono">{r.date || '—'}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {/if}
        </div>
        <div class="note">
          All data is shown revealed — use “hide passwords” to re-mask. Values are live/in-memory only and are never written to disk.
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 55;
    background: var(--scrim);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }
  .backdrop {
    position: absolute;
    inset: 0;
    z-index: 0;
    width: 100%;
    height: 100%;
    padding: 0;
    margin: 0;
    border: none;
    background: transparent;
    cursor: default;
  }
  .card {
    position: relative;
    z-index: 1;
    width: min(92vw, 560px);
    max-height: 82vh;
    overflow-y: auto;
    background: var(--card-glass);
    backdrop-filter: blur(28px);
    border: var(--bordw) solid var(--edge2);
    border-radius: var(--radius);
    padding: 20px;
    box-shadow: var(--shadow);
  }
  .card.wide {
    width: min(96vw, 940px);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 14px;
  }
  .head h2 {
    margin: 0;
    font: 700 15px/1 var(--fdisp);
    color: var(--ink);
    white-space: nowrap;
  }
  .head .arg {
    font: 600 12px/1 var(--fmono);
    color: var(--muted);
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--edge2);
    border-radius: calc(var(--radius) - 8px);
    padding: 5px 8px;
    word-break: break-all;
  }
  .icon-btn {
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border-radius: calc(var(--radius) - 8px);
    border: var(--bordw) solid var(--edge);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
    font-size: 12px;
    margin-left: auto;
    flex-shrink: 0;
  }
  .icon-btn:hover {
    color: var(--ink);
    border-color: var(--accent);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .row2 {
    display: flex;
    gap: 20px;
    flex-wrap: wrap;
  }
  .kv {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .kv .k {
    font: 700 9.5px/1 var(--fmono);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .kv .v {
    font: 600 13px/1.3 var(--fui);
    color: var(--ink);
    word-break: break-all;
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .sh {
    display: flex;
    align-items: baseline;
    gap: 6px;
    font: 700 10px/1 var(--fmono);
    letter-spacing: 0.09em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .sh .c {
    text-transform: none;
    letter-spacing: normal;
    color: var(--dim);
  }
  .none {
    font: 500 11.5px/1.4 var(--fui);
    color: var(--dim);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    font: 600 11px/1 var(--fmono);
    color: var(--ink);
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--edge2);
    border-radius: calc(var(--radius) - 8px);
    padding: 5px 8px;
  }
  .chip.cve {
    color: var(--crit);
    background: color-mix(in srgb, var(--crit) 12%, transparent);
    border-color: color-mix(in srgb, var(--crit) 45%, transparent);
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .li {
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--edge);
    border-radius: calc(var(--radius) - 8px);
    padding: 7px 9px;
    font: 600 11.5px/1.3 var(--fui);
    color: var(--ink);
  }
  .li.match {
    flex-direction: column;
    align-items: stretch;
    gap: 6px;
  }
  .mrow {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .li .tag {
    font: 700 9.5px/1 var(--fmono);
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--muted);
    border: 1px solid var(--edge2);
    border-radius: 5px;
    padding: 3px 5px;
    flex-shrink: 0;
  }
  .dim {
    color: var(--dim);
  }
  .mono {
    font-family: var(--fmono);
  }

  .reveal {
    margin-left: auto;
    font: 600 10px/1 var(--fmono);
    text-transform: none;
    letter-spacing: normal;
    color: var(--accent);
    background: transparent;
    border: 1px solid color-mix(in srgb, var(--accent) 40%, transparent);
    border-radius: calc(var(--radius) - 8px);
    padding: 4px 8px;
    cursor: pointer;
  }
  .reveal:hover {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }
  .tablewrap {
    overflow-x: auto;
    border: 1px solid var(--edge);
    border-radius: calc(var(--radius) - 6px);
  }
  table.leaktable {
    width: 100%;
    border-collapse: collapse;
    font: 500 11px/1.4 var(--fui);
  }
  table.leaktable th {
    text-align: left;
    font: 700 9px/1 var(--fmono);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--muted);
    padding: 8px 10px;
    border-bottom: 1px solid var(--edge2);
    white-space: nowrap;
  }
  table.leaktable td {
    padding: 7px 10px;
    border-bottom: 1px solid var(--edge);
    color: var(--ink);
    white-space: nowrap;
    vertical-align: top;
  }
  table.leaktable tbody tr:last-child td {
    border-bottom: none;
  }
  .pwval {
    color: var(--crit);
  }
  .pwval.masked {
    color: var(--dim);
    letter-spacing: 0.15em;
  }
  .hashcell {
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .note {
    font: 500 10.5px/1.5 var(--fui);
    color: var(--dim);
  }
  .warn {
    font: 700 11px/1.4 var(--fui);
    color: var(--crit);
    background: color-mix(in srgb, var(--crit) 12%, transparent);
    border: 1px solid color-mix(in srgb, var(--crit) 40%, transparent);
    border-radius: calc(var(--radius) - 6px);
    padding: 8px 11px;
  }
</style>
