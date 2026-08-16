import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import type { Finding } from '$lib/types';

function seededFinding(): Finding {
  return {
    id: 'a',
    programName: 'Acme',
    platform: 'h1',
    title: 'IDOR in /api/users',
    severity: 'high',
    status: 'draft',
    endpoint: 'https://x/api/users/1',
    summary: '',
    description: '',
    steps: '',
    evidence: '',
    impact: '',
    remediation: '',
    cvss: '',
    cvssScore: '',
    notes: '',
    cmdline: '',
    createdAt: '2026-01-01T00:00:00.000Z',
    updatedAt: '2026-01-01T00:00:00.000Z',
  };
}

// Async factory + dynamic import (rather than require()) so this stays valid
// ESM in a project with no @types/node — same pattern Cockpit.test.ts uses
// for its store mocks.
vi.mock('$lib/stores/findings', async () => {
  const { writable } = await import('svelte/store');
  return {
    findings: writable([seededFinding()]),
    loadFindings: vi.fn(),
    saveFinding: vi.fn(),
    deleteFinding: vi.fn(),
    newFinding: () => ({ ...seededFinding(), id: 'new-1', title: '' }),
  };
});

import FindingsPanel from './FindingsPanel.svelte';

describe('FindingsPanel', () => {
  it('renders the seeded finding title and a + New finding control', () => {
    render(FindingsPanel, { props: { open: true } });

    expect(screen.getByText('IDOR in /api/users')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /\+ New finding/i })).toBeInTheDocument();
  });
});
