import { describe, it, expect, vi, beforeEach } from 'vitest';
const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invoke(...a) }));
import * as bridge from './bridge';

beforeEach(() => invoke.mockReset());

describe('command bridge', () => {
  // ── Anchor tests (from task-2-brief.md) ──────────────────────────────────

  it('getConfig calls invoke("get_config")', async () => {
    invoke.mockResolvedValue({ webhookUrl: '' });
    await bridge.getConfig();
    expect(invoke).toHaveBeenCalledWith('get_config');
  });

  // NOTE: the brief's anchor test assumed `run_command` took { id, command, shell }.
  // The actual Rust signature (src-tauri/src/commands.rs:92-110) is
  // `run_command(app, id: String, cmdline: String, state)` — the shell is read
  // server-side from AppState, not passed by the caller. Updated to match reality.
  it('runCommand forwards args to invoke("run_command")', async () => {
    invoke.mockResolvedValue(undefined);
    await bridge.runCommand({ id: 'j1', cmdline: 'whoami' });
    expect(invoke).toHaveBeenCalledWith('run_command', { id: 'j1', cmdline: 'whoami' });
  });

  it('deckSetFolder forwards the path', async () => {
    invoke.mockResolvedValue(undefined);
    await bridge.deckSetFolder('C:/x/trapline-deck');
    expect(invoke).toHaveBeenCalledWith('deck_set_folder', { path: 'C:/x/trapline-deck' });
  });

  // ── Additional coverage for commands whose arg shapes deviated from the
  // brief's assumed shape once checked against the real Rust signatures ──────

  it('cancelCommand forwards the id', async () => {
    invoke.mockResolvedValue(undefined);
    await bridge.cancelCommand('j1');
    expect(invoke).toHaveBeenCalledWith('cancel_command', { id: 'j1' });
  });

  // src-tauri/src/commands.rs:153-184 — send_card(id: String, title: String, state)
  it('sendCard forwards id + title (not the brief-assumed title/desc/filename/data/color)', async () => {
    invoke.mockResolvedValue(undefined);
    await bridge.sendCard({ id: 'j1', title: 'Recon complete' });
    expect(invoke).toHaveBeenCalledWith('send_card', { id: 'j1', title: 'Recon complete' });
  });

  // src-tauri/src/commands.rs:193-223 — send_loot(markdown: String, state)
  it('sendLoot forwards a markdown string (not the brief-assumed title/items)', async () => {
    invoke.mockResolvedValue(undefined);
    await bridge.sendLoot({ markdown: '- **finding one**' });
    expect(invoke).toHaveBeenCalledWith('send_loot', { markdown: '- **finding one**' });
  });

  // src-tauri/src/commands.rs:287-290 + findings.rs:84-101 — save_finding(data: String)
  // parses `data` as JSON server-side, so the bridge must stringify the Finding first.
  it('saveFinding stringifies the finding into the "data" field', async () => {
    invoke.mockResolvedValue(undefined);
    const finding = { id: 'f1', title: 'IDOR', severity: 'high' } as unknown as Awaited<
      ReturnType<typeof bridge.loadFindings>
    >[number];
    await bridge.saveFinding(finding);
    expect(invoke).toHaveBeenCalledWith('save_finding', { data: JSON.stringify(finding) });
  });

  // src-tauri/src/commands.rs:292-295 — load_findings() -> Result<String, String>
  // returns a JSON-encoded array as a string; the bridge must parse it back out.
  it('loadFindings parses the JSON string the backend returns', async () => {
    const findings = [{ id: 'f1', title: 'IDOR' }];
    invoke.mockResolvedValue(JSON.stringify(findings));
    const result = await bridge.loadFindings();
    expect(invoke).toHaveBeenCalledWith('load_findings');
    expect(result).toEqual(findings);
  });

  it('deleteFinding forwards the id', async () => {
    invoke.mockResolvedValue(undefined);
    await bridge.deleteFinding('f1');
    expect(invoke).toHaveBeenCalledWith('delete_finding', { id: 'f1' });
  });

  // src-tauri/src/commands.rs:270-273 + session.rs — save_session(data: String)
  it('saveSession stringifies the session into the "data" field', async () => {
    invoke.mockResolvedValue(undefined);
    const session = { target: 'example.com' };
    await bridge.saveSession(session);
    expect(invoke).toHaveBeenCalledWith('save_session', { data: JSON.stringify(session) });
  });

  // src-tauri/src/commands.rs:275-278 — load_session() -> Result<String, String>
  it('loadSession parses the JSON string the backend returns', async () => {
    const session = { target: 'example.com' };
    invoke.mockResolvedValue(JSON.stringify(session));
    const result = await bridge.loadSession();
    expect(invoke).toHaveBeenCalledWith('load_session');
    expect(result).toEqual(session);
  });

  it('clearSession calls invoke("clear_session") with no args', async () => {
    invoke.mockResolvedValue(undefined);
    await bridge.clearSession();
    expect(invoke).toHaveBeenCalledWith('clear_session');
  });

  it('toolCheck calls invoke("tool_check")', async () => {
    invoke.mockResolvedValue([]);
    await bridge.toolCheck();
    expect(invoke).toHaveBeenCalledWith('tool_check');
  });

  it('setConfig forwards the config object', async () => {
    invoke.mockResolvedValue(undefined);
    const config = {
      webhookUrl: 'https://discord.com/api/webhooks/x',
      username: 'Trapline',
      shell: '',
      communityDiscord: '',
      deckPath: '',
      deckPort: 8787,
      deckToken: '',
      watchTargets: [],
      watchIntervalSecs: 1800,
      watchAlertThreshold: 50,
      watchMaxRpm: 30,
      watchEnabled: false,
    };
    await bridge.setConfig(config);
    expect(invoke).toHaveBeenCalledWith('set_config', { config });
  });

  it('testWebhook forwards the url', async () => {
    invoke.mockResolvedValue(undefined);
    await bridge.testWebhook('https://discord.com/api/webhooks/x');
    expect(invoke).toHaveBeenCalledWith('test_webhook', { url: 'https://discord.com/api/webhooks/x' });
  });

  it('openUrl forwards the url', async () => {
    invoke.mockResolvedValue(undefined);
    await bridge.openUrl('https://trapline.xyz');
    expect(invoke).toHaveBeenCalledWith('open_url', { url: 'https://trapline.xyz' });
  });

  it('deckStart calls invoke("deck_start") with no args', async () => {
    invoke.mockResolvedValue({ running: true });
    await bridge.deckStart();
    expect(invoke).toHaveBeenCalledWith('deck_start');
  });

  it('deckStop calls invoke("deck_stop") with no args', async () => {
    invoke.mockResolvedValue(undefined);
    await bridge.deckStop();
    expect(invoke).toHaveBeenCalledWith('deck_stop');
  });

  it('deckStatus calls invoke("deck_status") with no args', async () => {
    invoke.mockResolvedValue({ running: false });
    await bridge.deckStatus();
    expect(invoke).toHaveBeenCalledWith('deck_status');
  });
});
