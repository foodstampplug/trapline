use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Endpoint,
    Route,
    Secret,
    Flag,
}

impl Kind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::Endpoint => "endpoint",
            Kind::Route => "route",
            Kind::Secret => "secret",
            Kind::Flag => "flag",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Artifact {
    pub kind: Kind,
    pub value: String,
}

// ---- secret patterns (the high-value catches) -------------------------------
static SECRETS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"sk_live_[0-9a-zA-Z]{16,}").unwrap(),
        Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
        Regex::new(r"AIza[0-9A-Za-z_\-]{35}").unwrap(),
        Regex::new(r"gh[pousr]_[0-9A-Za-z]{36,}").unwrap(),
        Regex::new(r"xox[baprs]-[0-9A-Za-z-]{10,}").unwrap(),
        Regex::new(r"eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}").unwrap(),
        Regex::new(r"AC[0-9a-fA-F]{32}").unwrap(),
        Regex::new(r"SG\.[0-9A-Za-z_\-]{16,}\.[0-9A-Za-z_\-]{16,}").unwrap(),
        Regex::new(
            r#"(?i)(?:api[_-]?key|apikey|secret|access[_-]?token|auth[_-]?token|id[_-]?token|client[_-]?secret|password)["']?\s*[:=]\s*["'][0-9A-Za-z\-_\.]{8,}["']"#,
        )
        .unwrap(),
    ]
});

// ---- surface patterns -------------------------------------------------------
static URL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"https?://[A-Za-z0-9.\-]+(?:/[^\s"'`<>()]*)?"#).unwrap());
static PATH_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"["'`](/[A-Za-z0-9_\-./{}:]{2,160})["'`]"#).unwrap());
static CALL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?:fetch|\.(?:get|post|put|patch|delete)|axios|XMLHttpRequest|\.open)\(\s*["'`]([^"'`<>()]{2,200})["'`]"#).unwrap()
});
static ROUTE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"path\s*:\s*["'`]([^"'`<>()]{1,160})["'`]"#).unwrap());
static FLAG_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"["'`]?((?:enable|feature|flag|is[A-Z])[A-Za-z0-9_]{2,40})["'`]?\s*[:=]"#).unwrap()
});

const BORING_EXT: [&str; 15] = [
    ".png", ".jpg", ".jpeg", ".gif", ".svg", ".css", ".woff", ".woff2", ".ttf", ".ico", ".map",
    ".mp4", ".webp", ".scss", ".less",
];

fn boring_path(v: &str) -> bool {
    let lv = v.to_lowercase();
    let q = lv.split(['?', '#']).next().unwrap_or(&lv);
    BORING_EXT.iter().any(|e| q.ends_with(e))
}

fn add(kind: Kind, value: &str, seen: &mut HashSet<String>, out: &mut Vec<Artifact>) {
    let v = value.trim().to_string();
    if v.len() < 2 || v.len() > 400 {
        return;
    }
    // Defense-in-depth: never let HTML metacharacters from untrusted target JS
    // into a finding field — prevents seeding a stored-XSS payload downstream.
    if v.contains('<') || v.contains('>') {
        return;
    }
    if matches!(kind, Kind::Endpoint | Kind::Route) && boring_path(&v) {
        return;
    }
    let key = format!("{}|{}", kind.as_str(), v);
    if seen.insert(key) {
        out.push(Artifact { kind, value: v });
    }
}

/// Pull every interesting artifact out of a chunk of JavaScript.
/// We diff the SET of these across time, not the source text — minification and
/// identifier mangling never reach this layer, which is what keeps noise down.
pub fn extract(js: &str) -> Vec<Artifact> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<Artifact> = Vec::new();

    for re in SECRETS.iter() {
        for m in re.find_iter(js) {
            add(Kind::Secret, m.as_str(), &mut seen, &mut out);
        }
    }
    for m in URL_RE.find_iter(js) {
        add(Kind::Endpoint, m.as_str(), &mut seen, &mut out);
    }
    for c in PATH_RE.captures_iter(js) {
        if let Some(g) = c.get(1) {
            add(Kind::Endpoint, g.as_str(), &mut seen, &mut out);
        }
    }
    for c in CALL_RE.captures_iter(js) {
        if let Some(g) = c.get(1) {
            add(Kind::Endpoint, g.as_str(), &mut seen, &mut out);
        }
    }
    for c in ROUTE_RE.captures_iter(js) {
        if let Some(g) = c.get(1) {
            add(Kind::Route, g.as_str(), &mut seen, &mut out);
        }
    }
    for c in FLAG_RE.captures_iter(js) {
        if let Some(g) = c.get(1) {
            add(Kind::Flag, g.as_str(), &mut seen, &mut out);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_endpoint_and_secret() {
        let js = r#"fetch("/api/v2/users"); const k="sk_live_abcdefghijklmnop1234";"#;
        let arts = extract(js);
        assert!(arts.iter().any(|a| a.kind == Kind::Endpoint && a.value.contains("/api/v2/users")));
        assert!(arts.iter().any(|a| a.kind == Kind::Secret));
    }

    #[test]
    fn ignores_image_paths() {
        let js = r#"const logo = "/assets/logo.svg";"#;
        let arts = extract(js);
        assert!(arts.is_empty());
    }
}
