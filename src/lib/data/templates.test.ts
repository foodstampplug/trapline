import { describe, it, expect } from 'vitest';
import { TEMPLATES, flattenTemplates } from './templates';

describe('TEMPLATES data', () => {
  it('has all 30 categories', () => {
    expect(TEMPLATES.length).toBe(30);
  });
  it('preserves a known Quickfire command verbatim', () => {
    const all = flattenTemplates();
    const kong = all.find((t) => t.name === 'Kong portal UUID leak');
    expect(kong).toBeDefined();
    expect(kong!.cmd).toContain('/api/v3/portal');
    expect(kong!.cat).toBe('Quickfire');
  });
  it('flatten count equals the sum of category items', () => {
    const sum = TEMPLATES.reduce((n, c) => n + c.items.length, 0);
    expect(flattenTemplates().length).toBe(sum);
    expect(sum).toBeGreaterThanOrEqual(215);
  });
});
