import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/svelte';
import { addLoot, clearLoot } from '$lib/stores/loot';
import Loot from './Loot.svelte';

// @testing-library/svelte's built-in auto-cleanup never registers here
// (same reason as FindingEditor.test.ts/Playbook.test.ts) — unmount
// explicitly so the second test doesn't see the first render's DOM.
beforeEach(() => clearLoot());
afterEach(cleanup);

describe('Loot', () => {
  it('renders a collected item and the Copy Markdown control', () => {
    addLoot([{ cat: 'secret', sev: 'high', name: 'AWS key', value: 'AKIA…' }], 'cmd1');
    render(Loot, { props: { open: true } });

    expect(screen.getByText('AWS key')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Copy Markdown/i })).toBeInTheDocument();
  });

  it('shows the empty state when nothing has been collected', () => {
    render(Loot, { props: { open: true } });
    expect(screen.getByText(/No flags collected yet/i)).toBeInTheDocument();
  });
});
