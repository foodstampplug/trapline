import { describe, it, expect } from 'vitest';
import { CVSS_DEFAULTS } from './cvss';
describe('CVSS defaults', () => {
  it('gives critical a 9.x score and an AV:N vector', () => {
    expect(CVSS_DEFAULTS.critical.score.startsWith('9')).toBe(true);
    expect(CVSS_DEFAULTS.critical.vector).toContain('AV:N');
  });
  it('has an entry for every severity', () => {
    for (const s of ['critical','high','medium','low','info'] as const) {
      expect(CVSS_DEFAULTS[s].vector).toMatch(/^CVSS:3\.1/);
    }
  });
});
