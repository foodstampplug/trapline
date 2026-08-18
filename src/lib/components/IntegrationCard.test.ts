import { describe, it, expect, afterEach } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/svelte';
import IntegrationCard from './IntegrationCard.svelte';
import type { ShodanHost, LeakResult } from '$lib/types';

// This project's vite.config.ts doesn't set `test.globals`, so
// @testing-library/svelte's built-in auto-cleanup never registers — every
// file with more than one `it`/`render` cleans up explicitly between tests
// (same reason as Settings.test.ts/Loot.test.ts).
afterEach(cleanup);

describe('IntegrationCard', () => {
  it('renders a Shodan host result: org, a port, and a CVE chip', () => {
    const data: ShodanHost = {
      ip: '1.2.3.4',
      org: 'Acme Corp',
      hostnames: ['a.acme.com'],
      ports: [22, 443, 8080],
      services: [
        { port: 443, product: 'nginx', version: '1.20' },
        { port: 22, product: 'OpenSSH', version: '8.2' },
      ],
      cves: ['CVE-2021-41773', 'CVE-2020-1234'],
    };

    render(IntegrationCard, { props: { card: { kind: 'shodanHost', data }, arg: '1.2.3.4' } });

    expect(screen.getByText('Acme Corp')).toBeInTheDocument();
    // 8080 only appears in the ports list (443/22 also appear in Services),
    // so this unambiguously asserts a port chip rendered.
    expect(screen.getByText('8080')).toBeInTheDocument();
    expect(screen.getByText('CVE-2021-41773')).toBeInTheDocument();
  });

  it('renders the LeakCheck table with per-row intel; password masked by default, reveal shows plaintext', async () => {
    const data: LeakResult = {
      found: 2,
      sources: [{ name: 'BreachCo 2019', date: '2019-06-01' }],
      results: [
        {
          email: 'neo@acme.com',
          username: 'neo',
          password: 'hunter2',
          passwordPresent: true,
          phone: '+1555',
          name: 'Thomas Anderson',
          source: 'BreachCo 2019',
          date: '2019-06-01',
        },
        { email: 'trin@acme.com', username: '', password: '', passwordPresent: false, phone: '', name: '', source: 'OtherLeak', date: '' },
      ],
    };

    render(IntegrationCard, { props: { card: { kind: 'leak', data }, arg: 'acme.com' } });

    // Real per-row intel renders in the table:
    expect(screen.getByText('neo@acme.com')).toBeInTheDocument();
    expect(screen.getByText('trin@acme.com')).toBeInTheDocument();
    expect(screen.getByText('neo')).toBeInTheDocument();
    expect(screen.getByText('Thomas Anderson')).toBeInTheDocument();

    // Password is masked by default — the plaintext is NOT visible yet:
    expect(screen.queryByText('hunter2')).not.toBeInTheDocument();
    expect(screen.getAllByText('••••••').length).toBeGreaterThan(0);

    // Reveal shows the plaintext (user-authorized, live/in-memory only):
    await fireEvent.click(screen.getByText(/reveal passwords/i));
    expect(screen.getByText('hunter2')).toBeInTheDocument();
  });
});
