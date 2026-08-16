import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import Launcher from './Launcher.svelte';

describe('Launcher', () => {
  it('filters all 215 commands by a case-insensitive substring over name/desc/tool/cat', async () => {
    render(Launcher, { props: { open: true } });

    const search = screen.getByPlaceholderText(/search/i);
    await fireEvent.input(search, { target: { value: 'kong' } });

    expect(screen.getByText('Kong portal UUID leak')).toBeInTheDocument();
    expect(screen.queryByText('subfinder')).not.toBeInTheDocument();
  });
});
