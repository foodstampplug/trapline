const VAR_RE = /\{\{(\w+)\}\}/g;

export function extractVars(cmd: string): string[] {
  const seen = new Set<string>();
  for (const m of cmd.matchAll(VAR_RE)) seen.add(m[1]);
  return [...seen];
}

export function resolveCmd(cmd: string, vars: Record<string, string>): string {
  return cmd.replace(VAR_RE, (m, name) => (vars[name] ? vars[name] : m));
}
