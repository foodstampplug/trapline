import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/svelte';
import type { Finding } from '$lib/types';

const { saveFinding, deleteFinding } = vi.hoisted(() => ({
  saveFinding: vi.fn(),
  deleteFinding: vi.fn(),
}));

function blankFinding(): Finding {
  return {
    id: 'new-1',
    programName: '',
    platform: '',
    title: '',
    severity: 'medium',
    status: 'draft',
    endpoint: '',
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
    createdAt: '',
    updatedAt: '',
  };
}

vi.mock('$lib/stores/findings', () => ({
  saveFinding,
  deleteFinding,
  newFinding: blankFinding,
}));

import FindingEditor from './FindingEditor.svelte';
import { newFinding } from '$lib/stores/findings';
import { CVSS_DEFAULTS } from '$lib/data/cvss';

// This project's vite.config.ts doesn't set `test.globals`, so
// @testing-library/svelte's built-in auto-cleanup never registers — every
// file with more than one `it`/`render` cleans up explicitly between tests.
afterEach(cleanup);

describe('FindingEditor', () => {
  it('renders the Title field, auto-fills CVSS on severity change, and saves the edited finding', async () => {
    const blank = newFinding();
    render(FindingEditor, { props: { finding: blank } });

    const titleInput = screen.getByLabelText(/title/i) as HTMLInputElement;
    expect(titleInput).toBeInTheDocument();

    // Change severity → critical
    const severitySelect = screen.getByLabelText('Severity') as HTMLSelectElement;
    await fireEvent.change(severitySelect, { target: { value: 'critical' } });

    // CVSS vector field should now show the critical default
    const cvssInput = screen.getByLabelText(/cvss vector/i) as HTMLInputElement;
    expect(cvssInput.value).toBe(CVSS_DEFAULTS.critical.vector);

    // Fill in a title and save
    await fireEvent.input(titleInput, { target: { value: 'IDOR in /api/v1/users/{id}' } });

    const saveBtn = screen.getByRole('button', { name: /save finding/i });
    await fireEvent.click(saveBtn);

    expect(saveFinding).toHaveBeenCalledTimes(1);
    const saved = saveFinding.mock.calls[0][0] as Finding;
    expect(saved.severity).toBe('critical');
    expect(saved.title).toBe('IDOR in /api/v1/users/{id}');
    expect(saved.cvss).toBe(CVSS_DEFAULTS.critical.vector);
  });

  it('leaves a hand-edited CVSS vector alone when severity changes again', async () => {
    const blank = newFinding();
    render(FindingEditor, { props: { finding: blank } });

    const cvssInput = screen.getByLabelText(/cvss vector/i) as HTMLInputElement;
    const custom = 'CVSS:3.1/AV:L/AC:H/PR:H/UI:R/S:U/C:L/I:N/A:N';
    await fireEvent.input(cvssInput, { target: { value: custom } });

    const severitySelect = screen.getByLabelText('Severity') as HTMLSelectElement;
    await fireEvent.change(severitySelect, { target: { value: 'low' } });

    expect(cvssInput.value).toBe(custom);
  });

  it('delete calls deleteFinding with the finding id and onClose (existing finding)', async () => {
    const blank = newFinding();
    const onClose = vi.fn();
    render(FindingEditor, { props: { finding: blank, existing: true, onClose } });

    const deleteBtn = screen.getByRole('button', { name: /delete/i });
    await fireEvent.click(deleteBtn);

    expect(deleteFinding).toHaveBeenCalledWith(blank.id);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('hides the Delete control for a new (not-yet-saved) finding, shows it when existing', () => {
    const blank = newFinding();

    const { unmount } = render(FindingEditor, { props: { finding: blank } });
    expect(screen.queryByRole('button', { name: /delete/i })).not.toBeInTheDocument();
    unmount();

    render(FindingEditor, { props: { finding: blank, existing: true } });
    expect(screen.getByRole('button', { name: /delete/i })).toBeInTheDocument();
  });
});
