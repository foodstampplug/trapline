import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup, within } from '@testing-library/svelte';

// Async factory + dynamic import (rather than require()) so this stays valid
// ESM in a project with no @types/node — same pattern Cockpit.test.ts /
// FindingsPanel.test.ts use for their store mocks. Seeded with 2 findings and
// 3 loot rows so the rail badges have something non-zero to render.
vi.mock('$lib/stores/findings', async () => {
  const { writable } = await import('svelte/store');
  return { findings: writable([{ id: 'f1' }, { id: 'f2' }]) };
});
vi.mock('$lib/stores/loot', async () => {
  const { writable } = await import('svelte/store');
  return {
    loot: writable([
      { cat: 'secret', sev: 'high', name: 'a', value: '1', cmd: 'x' },
      { cat: 'secret', sev: 'high', name: 'b', value: '2', cmd: 'x' },
      { cat: 'secret', sev: 'high', name: 'c', value: '3', cmd: 'x' },
    ]),
  };
});

import AppRail from './AppRail.svelte';
import { findings } from '$lib/stores/findings';
import { loot } from '$lib/stores/loot';

const noopProps = {
  onOpenPlaybook: vi.fn(),
  onOpenTools: vi.fn(),
  onOpenFindings: vi.fn(),
  onOpenLoot: vi.fn(),
  onOpenSettings: vi.fn(),
  onOpenDeck: vi.fn(),
};

afterEach(() => {
  cleanup();
});

describe('AppRail count badges', () => {
  it('shows a count badge of 2 on Findings and 3 on Loot when the stores are non-empty', () => {
    render(AppRail, { props: noopProps });

    const findingsBtn = screen.getByTitle('Findings');
    const lootBtn = screen.getByTitle('Loot');
    expect(within(findingsBtn).getByText('2')).toBeInTheDocument();
    expect(within(lootBtn).getByText('3')).toBeInTheDocument();
  });

  it('hides both badges when the findings and loot stores are empty', () => {
    findings.set([]);
    loot.set([]);

    render(AppRail, { props: noopProps });

    const findingsBtn = screen.getByTitle('Findings');
    const lootBtn = screen.getByTitle('Loot');
    expect(within(findingsBtn).queryByText(/^\d+$/)).not.toBeInTheDocument();
    expect(within(lootBtn).queryByText(/^\d+$/)).not.toBeInTheDocument();
  });
});
