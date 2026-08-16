import { describe, it, expect } from 'vitest';
import { generateReport } from './generate';
const f = { id:'1', title:'IDOR in /api/users', severity:'high', endpoint:'https://x/api/users/1',
  summary:'S', steps:'1. do', evidence:'curl ...', impact:'bad', remediation:'fix',
  cvss:'CVSS:3.1/AV:N', cvssScore:'8.1', programName:'Acme', platform:'h1', status:'draft',
  description:'', notes:'', cmdline:'', createdAt:'', updatedAt:'' } as any;
describe('generateReport', () => {
  it('includes title, severity, CVSS, and the impact section', () => {
    const md = generateReport(f);
    expect(md).toContain('IDOR in /api/users');
    expect(md).toMatch(/SEVERITY|Severity/);
    expect(md).toContain('8.1');
    expect(md).toContain('bad'); // impact
  });
});
