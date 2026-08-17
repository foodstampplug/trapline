import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, cleanup } from '@testing-library/svelte';
import { writable } from 'svelte/store';

vi.mock('$lib/stores/watch', () => ({
  watch: writable({ running: true, targets: 2, intervalSecs: 1800, lastRunMs: Date.now(), lastAssets: 18, lastNew: 3 }),
}));
vi.mock('$lib/stores/findings', () => ({
  findings: writable([
    { id: 'a', severity: 'critical', title: 'Creds in breach dump', programName: 'acme', endpoint: 'x', createdAt: '2026-08-16T00:00:00Z' },
    { id: 'b', severity: 'high', title: 'Admin panel exposed', programName: 'acme', endpoint: 'y', createdAt: '2026-08-16T00:00:00Z' },
  ]),
}));

import RightDock from './RightDock.svelte';
import { watch } from '$lib/stores/watch';
import { findings } from '$lib/stores/findings';

afterEach(() => {
  cleanup();
});

describe('RightDock', () => {
  it('renders live watch counts and findings from the stores', () => {
    const { getByText, container } = render(RightDock);
    expect(getByText('18')).toBeTruthy(); // lastAssets
    expect(getByText('3')).toBeTruthy(); // lastNew
    expect(getByText('Creds in breach dump')).toBeTruthy();
    expect(getByText('Admin panel exposed')).toBeTruthy();
    // findings count in the header reflects the store length (2), not the old mock "4"
    expect(container.querySelector('.findbox .c')?.textContent?.trim()).toBe('2');
    // exactly the store's findings render as rows — not the old hardcoded 3-row mock
    expect(container.querySelectorAll('.fl .fi').length).toBe(2);
    expect(container.textContent).not.toContain('Admin OpenSSH CVE');
  });

  it('shows the live indicator only when watch is running', () => {
    watch.set({ running: true, targets: 1, intervalSecs: 900, lastRunMs: Date.now(), lastAssets: 5, lastNew: 0 });
    const running = render(RightDock);
    expect(running.container.querySelector('.wlive')?.textContent?.toLowerCase()).toContain('live');
    running.unmount();

    watch.set({ running: false, targets: 1, intervalSecs: 900, lastRunMs: 0, lastAssets: 5, lastNew: 0 });
    const idle = render(RightDock);
    expect(idle.container.querySelector('.wlive')?.textContent?.toLowerCase() ?? '').not.toContain('live');
  });

  it('reflects an empty findings store with a zero header count and no rows', () => {
    findings.set([]);
    const { container } = render(RightDock);
    expect(container.querySelector('.findbox .c')?.textContent?.trim()).toBe('0');
    expect(container.querySelectorAll('.fl .fi').length).toBe(0);
  });
});
