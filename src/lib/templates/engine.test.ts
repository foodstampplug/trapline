import { describe, it, expect } from 'vitest';
import { extractVars, resolveCmd } from './engine';

describe('template engine', () => {
  it('extracts unique vars in first-seen order', () => {
    expect(extractVars('curl {{url}}/{{path}} then {{url}}')).toEqual(['url', 'path']);
  });
  it('returns [] when there are no vars', () => {
    expect(extractVars('subfinder -d example.com')).toEqual([]);
  });
  it('resolves provided vars and leaves the rest as literal placeholders', () => {
    expect(resolveCmd('curl {{url}}/{{path}}', { url: 'https://x' })).toBe('curl https://x/{{path}}');
  });
  it('treats an empty-string value as unfilled', () => {
    expect(resolveCmd('curl {{url}}', { url: '' })).toBe('curl {{url}}');
  });
});
