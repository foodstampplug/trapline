import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/svelte';

vi.mock('$lib/stores/tools', async () => {
  const { writable } = await import('svelte/store');
  return {
    tools: writable([]),
    loadTools: vi.fn(),
    isMissing: (name: string) => name === 'amass',
  };
});

import Playbook from './Playbook.svelte';

// This project's vite.config.ts doesn't set `test.globals`, so
// @testing-library/svelte's built-in auto-cleanup (which only fires when it
// finds a *global* beforeEach/afterEach) never registers — every other test
// file in the repo works around this by keeping one `it`/`render` per file.
// This file has two, so clean up explicitly between them.
afterEach(cleanup);

describe('Playbook', () => {
  it('renders a category header, a known item, and ⚠ on a missing-tool item', () => {
    render(Playbook, { props: { open: true } });

    expect(screen.getByText('Quickfire')).toBeInTheDocument();
    expect(screen.getByText('subfinder')).toBeInTheDocument();

    const amassRow = screen.getByText('amass (passive)').closest('button');
    expect(amassRow).not.toBeNull();
    expect(amassRow?.textContent).toContain('⚠');
    expect(amassRow?.textContent).toContain('amass');
  });

  it('clicking an item calls onPick with the template + its category', async () => {
    const onPick = vi.fn();
    render(Playbook, { props: { open: true, onPick } });

    const row = screen.getByText('subfinder').closest('button')!;
    await fireEvent.click(row);

    expect(onPick).toHaveBeenCalledTimes(1);
    const arg = onPick.mock.calls[0][0];
    expect(arg.name).toBe('subfinder');
    expect(arg.cat).toBe('Subdomains');
  });
});
