import { describe, it, expect, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/svelte';
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

  it('renders a LeakCheck result: found count + a source + a redacted summary, never a plaintext password', () => {
    const data: LeakResult = {
      found: 3,
      sources: [{ name: 'BreachCo 2019', date: '2019-06-01' }],
      results: [
        { email: 'user@acme.com', usernamePresent: true, passwordPresent: true, source: 'BreachCo 2019' },
        { email: 'user@acme.com', usernamePresent: false, passwordPresent: false, source: 'OtherLeak' },
        { email: 'user@acme.com', usernamePresent: true, passwordPresent: true, source: 'BreachCo 2019' },
      ],
    };

    render(IntegrationCard, { props: { card: { kind: 'leak', data }, arg: 'user@acme.com' } });

    expect(screen.getByText('3')).toBeInTheDocument();
    expect(screen.getByText('BreachCo 2019')).toBeInTheDocument();
    expect(screen.getByText(/2 rows? with password present/i)).toBeInTheDocument();

    // No raw password field or value should ever render — LeakResult carries
    // no plaintext password anywhere in its type, and this must stay a
    // redacted aggregate (count only), never a per-row dump.
    expect(screen.queryByText(/password123|hunter2/i)).not.toBeInTheDocument();
    const html = document.body.innerHTML.toLowerCase();
    expect(html).not.toContain('"password"');
    expect(html).not.toContain('passwordpresent');
  });
});
