// Ported verbatim from main:src/main.js — CVSS_DEFAULTS (~line 868) and
// OWASP_REFS + owaspRef (~line 881). Vectors, scores, and justifications are
// unchanged; only typed for the SvelteKit front end.

export type Severity = 'critical' | 'high' | 'medium' | 'low' | 'info';

export interface CvssDefault {
  vector: string;
  score: string;
  justification: string;
}

export const CVSS_DEFAULTS: Record<Severity, CvssDefault> = {
  critical: {
    vector: 'CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H',
    score: '9.8',
    justification:
      'AV:Network — exploitable remotely. AC:Low — no special conditions. PR:None — no privileges required. UI:None — no user interaction. Scope:Unchanged. C/I/A:High — full data compromise.',
  },
  high: {
    vector: 'CVSS:3.1/AV:N/AC:L/PR:L/UI:N/S:U/C:H/I:H/A:N',
    score: '8.1',
    justification:
      'AV:Network — exploitable remotely. AC:Low — no special conditions. PR:Low — requires basic authenticated access. UI:None. C/I:High — significant data read/write.',
  },
  medium: {
    vector: 'CVSS:3.1/AV:N/AC:L/PR:L/UI:N/S:U/C:L/I:L/A:N',
    score: '5.4',
    justification:
      'AV:Network — exploitable remotely. PR:Low — requires authenticated access. C/I:Low — partial data impact.',
  },
  low: {
    vector: 'CVSS:3.1/AV:N/AC:H/PR:L/UI:R/S:U/C:L/I:N/A:N',
    score: '2.6',
    justification:
      'AC:High — requires specific conditions. UI:Required — victim must take an action. C:Low — limited information disclosure only.',
  },
  info: {
    vector: 'CVSS:3.1/AV:N/AC:H/PR:N/UI:R/S:U/C:N/I:N/A:N',
    score: '0.0',
    justification: 'Informational — no direct security impact.',
  },
};

export interface OwaspRefEntry {
  k: string[];
  ref: string;
}

export const OWASP_REFS: OwaspRefEntry[] = [
  { k: ['idor', 'object level', 'access control', 'authorization', 'privilege'], ref: 'https://owasp.org/API-Security/editions/2023/en/0xa1-broken-object-level-authorization/' },
  { k: ['cors'], ref: 'https://owasp.org/www-community/attacks/CORS_OriginHeaderScrutiny' },
  { k: ['xss', 'cross-site scripting'], ref: 'https://owasp.org/www-community/attacks/xss/' },
  { k: ['ssrf'], ref: 'https://owasp.org/www-community/attacks/Server_Side_Request_Forgery' },
  { k: ['sql', 'sqli', 'injection'], ref: 'https://owasp.org/www-community/attacks/SQL_Injection' },
  { k: ['ssti', 'template injection'], ref: 'https://owasp.org/www-project-web-security-testing-guide/v42/4-Web_Application_Security_Testing/07-Input_Validation_Testing/18-Testing_for_Server_Side_Template_Injection' },
  { k: ['xxe', 'xml'], ref: 'https://owasp.org/www-community/vulnerabilities/XML_External_Entity_(XXE)_Processing' },
  { k: ['open redirect'], ref: 'https://cheatsheetseries.owasp.org/cheatsheets/Unvalidated_Redirects_and_Forwards_Cheat_Sheet.html' },
  { k: ['race condition', 'race'], ref: 'https://owasp.org/www-community/vulnerabilities/Time_of_check_Time_of_use' },
  { k: ['subdomain takeover', 'takeover'], ref: 'https://owasp.org/www-project-web-security-testing-guide/v42/4-Web_Application_Security_Testing/02-Configuration_and_Deployment_Management_Testing/10-Test_for_Subdomain_Takeover' },
  { k: ['jwt', 'token', 'auth'], ref: 'https://owasp.org/www-project-top-ten/2017/A2_2017-Broken_Authentication' },
  { k: ['disclosure', 'exposure', 'leak', 'secret', 'key'], ref: 'https://owasp.org/www-project-top-ten/2017/A3_2017-Sensitive_Data_Exposure' },
];

/** Best-effort OWASP reference lookup by keyword match against a finding title. */
export function owaspRef(title: string): string {
  const t = (title || '').toLowerCase();
  for (const { k, ref } of OWASP_REFS) {
    if (k.some((kw) => t.includes(kw))) return ref;
  }
  return 'https://owasp.org/www-project-top-ten/';
}
