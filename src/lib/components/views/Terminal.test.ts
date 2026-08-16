import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
// Async factory + dynamic import mirrors the pattern used in Cockpit.test.ts —
// stays valid ESM without needing require() in this @types/node-less project.
vi.mock('$lib/stores/runs', async () => {
  const { writable } = await import('svelte/store');
  return {
    runs: writable([
      {
        id: 'r1',
        cmdline: 'shodan domain app.acme.com',
        lines: [
          {
            text: 'admin.app.acme.com 443 OpenSSH 8.2 CVE-2023-38408',
            stream: 'out',
            spans: [{ s: 0, e: 18, cat: 'recon', sev: 'critical', label: 'admin subdomain' }],
          },
        ],
        status: 'running',
        findings: [],
      },
    ]),
  };
});
import Terminal from './Terminal.svelte';

describe('Terminal view', () => {
  it('renders the run command line and the flagged substring as a <mark>', () => {
    render(Terminal);
    expect(screen.getByText(/shodan domain app\.acme\.com/)).toBeInTheDocument();
    const mark = screen.getByText('admin.app.acme.com');
    expect(mark.tagName).toBe('MARK');
    expect(mark.classList.contains('sev-critical')).toBe(true);
  });
});
