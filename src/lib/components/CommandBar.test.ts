import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';

const { startRun } = vi.hoisted(() => ({ startRun: vi.fn() }));
vi.mock('$lib/stores/runs', () => ({ startRun }));

import CommandBar from './CommandBar.svelte';

describe('CommandBar', () => {
  it('renders a fill input per {{var}} and calls startRun with the resolved command', async () => {
    render(CommandBar, { props: { cmd: 'curl {{url}}' } });

    const urlInput = screen.getByLabelText('url');
    await fireEvent.input(urlInput, { target: { value: 'https://x' } });

    const runBtn = screen.getByRole('button', { name: /run/i });
    await fireEvent.click(runBtn);

    expect(startRun).toHaveBeenCalledWith('curl https://x');
  });
});
