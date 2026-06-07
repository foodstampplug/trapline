use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Secret,
    Recon,
    Http,
    Id,
}

/// One highlighted match within a line (byte offsets)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub s: usize,
    pub e: usize,
    pub cat: Category,
    pub sev: Severity,
    pub label: String,
    pub value: String,
}

/// De-duplicated finding shown in command summary + Discord embed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub cat: Category,
    pub sev: Severity,
    pub name: String,
    pub value: String,
}

#[allow(dead_code)]
pub struct Rule {
    pub name: &'static str,
    pub cat: Category,
    pub sev: Severity,
    pub re: Lazy<Regex>,
}

fn sev_weight(s: &Severity) -> u8 {
    match s {
        Severity::Critical => 4,
        Severity::High => 3,
        Severity::Medium => 2,
        Severity::Info => 1,
    }
}

// All 94 detection rules, ported from flags.go
// Rust regex crate does not support look-around or backreferences (same as Go RE2)
pub static RULES: &[(&str, Category, Severity, &str)] = &[
    // ── Secrets & keys ─────────────────────────────────────────────────────────
    ("JWT", Category::Secret, Severity::High,
        r"eyJ[A-Za-z0-9_\-]{8,}\.[A-Za-z0-9_\-]{8,}\.[A-Za-z0-9_\-]{6,}"),
    ("AWS access key", Category::Secret, Severity::Critical,
        r"AKIA[0-9A-Z]{16}"),
    ("AWS temp credentials", Category::Secret, Severity::Critical,
        r"ASIA[0-9A-Z]{16}"),
    ("Google API key", Category::Secret, Severity::High,
        r"AIza[0-9A-Za-z_\-]{35}"),
    ("Slack token", Category::Secret, Severity::Critical,
        r"xox[baprs]-[0-9A-Za-z\-]{10,48}"),
    ("Stripe live key", Category::Secret, Severity::Critical,
        r"sk_live_[0-9A-Za-z]{16,}"),
    ("Stripe pub key", Category::Secret, Severity::Info,
        r"pk_live_[0-9A-Za-z]{16,}"),
    ("GitHub token", Category::Secret, Severity::Critical,
        r"gh[pousr]_[0-9A-Za-z]{36,}"),
    ("GitHub App token", Category::Secret, Severity::Critical,
        r"ghs_[0-9A-Za-z]{36}"),
    ("Sentry DSN", Category::Secret, Severity::High,
        r"https://[0-9a-f]{32}@[A-Za-z0-9.\-]+sentry[A-Za-z0-9.\-]*\.io/[0-9]+"),
    ("Private key block", Category::Secret, Severity::Critical,
        r"-----BEGIN (?:RSA |EC |OPENSSH |DSA |PGP )?PRIVATE KEY-----"),
    ("Bearer token", Category::Secret, Severity::High,
        r"(?i)bearer\s+[A-Za-z0-9._\-]{12,}"),
    ("Authorization header", Category::Secret, Severity::High,
        r"(?i)authorization:\s*[A-Za-z0-9._\-]+\s+\S+"),
    ("Secret assignment", Category::Secret, Severity::Medium,
        r#"(?i)(?:password|passwd|secret|api[_\-]?key|client[_\-]?secret|access[_\-]?token|auth[_\-]?token)["']?\s*[:=]\s*["']?[^\s"',}{)]{4,}"#),
    ("GitLab token", Category::Secret, Severity::Critical,
        r"glpat-[0-9A-Za-z_\-]{20}"),
    ("npm token", Category::Secret, Severity::High,
        r"npm_[0-9A-Za-z]{36}"),
    ("SendGrid key", Category::Secret, Severity::Critical,
        r"SG\.[A-Za-z0-9_\-]{22}\.[A-Za-z0-9_\-]{43}"),
    ("Twilio API key", Category::Secret, Severity::High,
        r"\bSK[0-9a-f]{32}\b"),
    ("Mailgun key", Category::Secret, Severity::High,
        r"\bkey-[0-9a-f]{32}\b"),
    ("GCP service account", Category::Secret, Severity::Critical,
        r#"(?i)"type"\s*:\s*"service_account""#),
    ("Azure storage key", Category::Secret, Severity::Critical,
        r"(?i)AccountKey=[A-Za-z0-9+/]{40,}={0,2}"),
    ("Slack webhook", Category::Secret, Severity::High,
        r"https://hooks\.slack\.com/services/[A-Za-z0-9/]{40,}"),
    ("Discord webhook", Category::Secret, Severity::Medium,
        r"https://(?:ptb\.|canary\.)?discord(?:app)?\.com/api/webhooks/\d+/[A-Za-z0-9_\-]+"),
    ("OpenAI key", Category::Secret, Severity::Critical,
        r"sk-(?:proj-[A-Za-z0-9_\-]{80,}|[A-Za-z0-9]{48})"),
    ("HuggingFace token", Category::Secret, Severity::High,
        r"\bhf_[A-Za-z0-9]{30,}\b"),
    ("HashiCorp Vault token", Category::Secret, Severity::Critical,
        r"\bhv[sbr]\.[A-Za-z0-9_\-]{24,}\b"),
    ("DigitalOcean PAT", Category::Secret, Severity::Critical,
        r"dop_v1_[A-Za-z0-9]{64}"),
    ("Mapbox token", Category::Secret, Severity::High,
        r"[ps]k\.eyJ1Ijoi[A-Za-z0-9_\-]+\.[A-Za-z0-9_\-]+"),
    ("Stripe test key", Category::Secret, Severity::Info,
        r"sk_test_[0-9A-Za-z]{16,}"),
    ("Stripe webhook secret", Category::Secret, Severity::High,
        r"whsec_[A-Za-z0-9]{32,}"),
    ("Stripe restricted key", Category::Secret, Severity::High,
        r"rk_live_[0-9A-Za-z]{16,}"),
    ("PlanetScale token", Category::Secret, Severity::Critical,
        r"pscale_tkn_[A-Za-z0-9_\-]{32,}"),
    ("Doppler token", Category::Secret, Severity::Critical,
        r"dp\.pt\.[A-Za-z0-9]{40,}"),
    ("Google OAuth token", Category::Secret, Severity::Critical,
        r"ya29\.[A-Za-z0-9_\-]{20,}"),
    ("MongoDB connection string", Category::Secret, Severity::Critical,
        r"mongodb(?:\+srv)?://[A-Za-z0-9._~:@%!\-]*/[A-Za-z0-9_\-]+"),
    ("PostgreSQL connection string", Category::Secret, Severity::Critical,
        r"postgres(?:ql)?://[A-Za-z0-9._~:@%!\-]*/[A-Za-z0-9_\-]+"),
    ("MySQL connection string", Category::Secret, Severity::Critical,
        r"mysql://[A-Za-z0-9._~:@%!\-]*/[A-Za-z0-9_\-]+"),
    ("Redis connection string", Category::Secret, Severity::Critical,
        r"redis://[A-Za-z0-9._~:@%!\-]+(?:/\d+)?"),
    ("Twilio account SID", Category::Secret, Severity::High,
        r"AC[0-9a-f]{32}"),
    ("AWS ARN", Category::Id, Severity::High,
        r"arn:aws:[a-z0-9\-]*:[a-z0-9\-]*:\d{12}:"),

    // ── ATO token fields (Skill 17) ─────────────────────────────────────────────
    ("ATO token — idToken", Category::Id, Severity::Critical,
        r#"(?i)"idToken"\s*:\s*"[A-Za-z0-9._\-]{20,}"#),
    ("ATO token — id_token", Category::Id, Severity::Critical,
        r#"(?i)"id_token"\s*:\s*"[A-Za-z0-9._\-]{20,}"#),
    ("ATO token — oauth_token", Category::Id, Severity::Critical,
        r#"(?i)"oauth_token"\s*:\s*"[A-Za-z0-9._\-]{20,}"#),
    ("ATO token — access_token", Category::Id, Severity::Critical,
        r#"(?i)"access_token"\s*:\s*"[A-Za-z0-9._\-]{20,}"#),
    ("ATO token — auth_data", Category::Id, Severity::Critical,
        r#"(?i)"auth_?data"\s*:\s*"[A-Za-z0-9._\-]{20,}"#),
    ("ATO token — bearer_token", Category::Id, Severity::Critical,
        r#"(?i)"bearer_?token"\s*:\s*"[A-Za-z0-9._\-]{20,}"#),
    ("ATO token — authData", Category::Id, Severity::Critical,
        r#"(?i)"authData"\s*:\s*"[A-Za-z0-9._\-]{20,}"#),
    ("ATO token — refresh_token", Category::Id, Severity::Critical,
        r#"(?i)"refresh_?token"\s*:\s*"[A-Za-z0-9._\-]{20,}"#),
    ("ATO token — session_token", Category::Id, Severity::Critical,
        r#"(?i)"session_?token"\s*:\s*"[A-Za-z0-9._\-]{20,}"#),
    ("ATO token — generic token field", Category::Id, Severity::High,
        r#"(?i)"token"\s*:\s*"[A-Za-z0-9._\-]{20,}"#),
    ("Password reset token in response", Category::Id, Severity::Critical,
        r#"(?i)"(?:reset_?token|password_?token|confirm_?token|verification_?token|verify_?token)"\s*:\s*"[A-Za-z0-9._\-]{10,}""#),

    // ── Recon & cloud ───────────────────────────────────────────────────────────
    ("Cloud metadata IP", Category::Recon, Severity::Critical,
        r"169\.254\.169\.254"),
    ("GCP metadata hostname", Category::Recon, Severity::Critical,
        r"metadata\.google\.internal"),
    ("Private IP — 10.x", Category::Recon, Severity::High,
        r"\b10\.\d{1,3}\.\d{1,3}\.\d{1,3}\b"),
    ("Private IP — 192.168", Category::Recon, Severity::High,
        r"\b192\.168\.\d{1,3}\.\d{1,3}\b"),
    ("Private IP — 172.16-31", Category::Recon, Severity::High,
        r"\b172\.(?:1[6-9]|2\d|3[01])\.\d{1,3}\.\d{1,3}\b"),
    ("Internal hostname (.internal)", Category::Recon, Severity::Medium,
        r"\b[a-z0-9\-]+\.internal\b"),
    ("Internal hostname (.local)", Category::Recon, Severity::Medium,
        r"\b[a-z0-9\-]+\.local\b"),
    ("Internal hostname (.corp)", Category::Recon, Severity::Medium,
        r"\b[a-z0-9\-]+\.corp\b"),
    ("Internal hostname (.intranet)", Category::Recon, Severity::Medium,
        r"\b[a-z0-9\-]+\.intranet\b"),
    ("S3 bucket", Category::Recon, Severity::Medium,
        r"(?:[a-z0-9.\-]+\.s3[a-z0-9.\-]*\.amazonaws\.com|s3://[a-z0-9.\-]{3,})"),
    ("GCS bucket", Category::Recon, Severity::Medium,
        r"storage\.googleapis\.com/[A-Za-z0-9._\-]+"),
    ("Exposed VCS/env path", Category::Recon, Severity::High,
        r#"(?i)/\.(?:git|env|svn|aws|ssh|htpasswd)(?:[/."'\s]|$)"#),
    ("Firebase DB", Category::Recon, Severity::High,
        r"https://[a-z0-9\-]+\.firebaseio\.com"),
    ("Firebase app", Category::Recon, Severity::Medium,
        r"\b[a-z0-9\-]+\.firebaseapp\.com\b"),
    ("Azure blob", Category::Recon, Severity::Medium,
        r"\b[a-z0-9]+\.blob\.core\.windows\.net\b"),
    ("DigitalOcean Spaces", Category::Recon, Severity::Medium,
        r"\b[a-z0-9.\-]+\.digitaloceanspaces\.com\b"),
    ("GraphQL introspection", Category::Recon, Severity::High,
        r#"(?i)"(?:__schema|queryType|mutationType)"\s*:"#),
    ("K8s internal svc", Category::Recon, Severity::Medium,
        r"\b[a-z0-9\-.]+\.svc\.cluster\.local\b"),
    ("Open directory listing", Category::Recon, Severity::Medium,
        r"(?i)<title>\s*Index of /"),
    ("Git HEAD ref", Category::Recon, Severity::High,
        r"(?i)^ref:\s*refs/heads/\S+"),
    ("Spring Actuator endpoint", Category::Recon, Severity::High,
        r"(?i)/actuator/(?:env|beans|mappings|configprops|loggers|heapdump|threaddump|httptrace|auditevents|shutdown|metrics)"),
    ("Swagger / OpenAPI", Category::Recon, Severity::Medium,
        r#"(?i)(?:swagger(?:-ui)?\.html|/v[23]/api-docs|/openapi\.(?:json|yaml)|"swagger"\s*:\s*"2\.|"openapi"\s*:\s*"3\.")"#),
    ("GraphQL IDE exposed", Category::Recon, Severity::Medium,
        r"(?i)graphiql|graphql-playground|apollo(?:\s+studio|\s+sandbox)"),
    ("Prometheus metrics", Category::Recon, Severity::Medium,
        r"# (?:HELP|TYPE) [a-z_]+"),
    ("Elasticsearch response", Category::Recon, Severity::High,
        r#"(?i)"_shards"\s*:\s*\{|"_index"\s*:\s*"[^"]+"|/_cat/indices"#),
    ("Kong Admin API", Category::Recon, Severity::Critical,
        r#"(?i)(?::8001/|x-kong-admin-latency\s*:|"kong_version"\s*:)"#),
    ("Kong portal path", Category::Recon, Severity::High,
        r"(?i)/api/v[23]/portals?/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}"),
    ("Jenkins exposed", Category::Recon, Severity::High,
        r#"(?i)(?:x-jenkins\s*:|jenkins-version\s*:|"_class"\s*:\s*"hudson\.")"#),
    ("K8s API response", Category::Recon, Severity::Critical,
        r#"(?i)"apiVersion"\s*:\s*"(?:v1|apps/v1|rbac\.authorization\.k8s\.io/v1)""#),
    ("GraphQL error (field/type leak)", Category::Recon, Severity::Medium,
        r#"(?i)"errors"\s*:.*"(?:Cannot query field|Field .* doesn't exist|Unknown argument|Did you mean|Expected type)""#),
    ("Stack trace (JS)", Category::Recon, Severity::Medium,
        r"at Object\.[A-Za-z]+ \("),
    ("Stack trace (Python)", Category::Recon, Severity::Medium,
        r"Traceback \(most recent call last\)"),
    ("Stack trace (Java)", Category::Recon, Severity::Medium,
        r"Exception in thread|at [a-z][a-zA-Z0-9]+\.[a-zA-Z0-9]+\("),
    ("Stack trace (.NET)", Category::Recon, Severity::Medium,
        r"NullReferenceException|System\.(?:InvalidOperation|ArgumentNull|NullReference)Exception"),
    ("SQL error in response", Category::Recon, Severity::Medium,
        r"(?i)(?:SQLException|SQLSTATE\[|You have an error in your SQL syntax|ORA-\d{5}|pg_query\(\)|near .* syntax error)"),

    // ── HTTP signals ────────────────────────────────────────────────────────────
    ("CORS null origin allowed", Category::Http, Severity::High,
        r"(?i)access-control-allow-origin:\s*null"),
    ("CORS origin reflected", Category::Http, Severity::High,
        r"(?i)access-control-allow-origin:\s*https?://\S+"),
    ("CORS wildcard", Category::Http, Severity::Medium,
        r"(?i)access-control-allow-origin:\s*\*"),
    ("CORS credentials", Category::Http, Severity::High,
        r"(?i)access-control-allow-credentials:\s*true"),
    ("CORS unsafe methods", Category::Http, Severity::High,
        r"(?i)access-control-allow-methods:.*\b(?:DELETE|PUT|PATCH)\b"),
    ("Server banner", Category::Http, Severity::Info,
        r"(?i)^\s*server:\s*\S.*"),
    ("Apache version banner", Category::Http, Severity::Medium,
        r"(?i)server:\s*Apache/\d+\.\d+\.\d+"),
    ("nginx version banner", Category::Http, Severity::Medium,
        r"(?i)server:\s*nginx/\d+\.\d+\.\d+"),
    ("IIS version banner", Category::Http, Severity::Medium,
        r"(?i)server:\s*Microsoft-IIS/\d+\.\d+"),
    ("Tech disclosure", Category::Http, Severity::Info,
        r"(?i)^\s*x-(?:powered-by|aspnet-version|aspnetmvc-version|generator):\s*\S.*"),
    ("Set-Cookie", Category::Http, Severity::Info,
        r"(?i)set-cookie:\s*\S+"),
    ("Server error 5xx", Category::Http, Severity::Medium,
        r"\bHTTP/\d(?:\.\d)?\s+5\d{2}\b|\b5\d{2}\s+(?:Internal Server Error|Bad Gateway|Service Unavailable)\b"),
    ("Auth gate 401/403", Category::Http, Severity::Medium,
        r"\bHTTP/\d(?:\.\d)?\s+(?:401|403)\b"),
    ("Stack trace / error", Category::Http, Severity::Medium,
        r"(?i)(?:exception in|stack trace|traceback \(most recent|fatal error|undefined index|SQLSTATE\[|ORA-\d{5})"),
    ("Debug mode exposed", Category::Http, Severity::Medium,
        r"(?i)(?:Werkzeug Debugger|Flask Debugger|Symfony Profiler|Whoops\b|DEBUG\s*=\s*true)"),
    ("Redirect Location", Category::Http, Severity::Info,
        r"(?i)^\s*location:\s*\S+"),
    ("WWW-Authenticate", Category::Http, Severity::Medium,
        r"(?i)^\s*www-authenticate:\s*\S.*"),
    ("Kong proxy header", Category::Http, Severity::Info,
        r"(?i)x-kong-(?:upstream-latency|proxy-latency|request-id|upstream-status)\s*:"),
    ("AWS infra header", Category::Http, Severity::Info,
        r"(?i)x-amzn-(?:requestid|trace-id|remapped-ip|errortype)\s*:"),
    ("Distributed trace header", Category::Http, Severity::Info,
        r"(?i)(?:x-b3-traceid|x-datadog-trace-id|uber-trace-id|traceparent)\s*:"),

    // ── IDs & PII ───────────────────────────────────────────────────────────────
    ("UUID", Category::Id, Severity::Info,
        r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-5][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}"),
    ("Email", Category::Id, Severity::Info,
        r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}"),
    ("Token field in JSON", Category::Id, Severity::High,
        r#"(?i)"(?:id_?token|idToken|access_?token|oauth_?token|auth_?(?:data|token)|authData|refresh_?token|session_?(?:id|token)?|sessionToken|bearer_?token|bearerToken|api_?token|apiToken|user_?token|userToken|sid)"\s*:\s*"[^"]{6,}""#),
    ("Credit card", Category::Id, Severity::Medium,
        r"\b(?:4[0-9]{12}(?:[0-9]{3})?|5[1-5][0-9]{14}|3[47][0-9]{13}|6(?:011|5[0-9]{2})[0-9]{12})\b"),
    ("US SSN", Category::Id, Severity::Medium,
        r"\b\d{3}-\d{2}-\d{4}\b"),
];

/// Compiled regex cache — built once at first use
static COMPILED: Lazy<Vec<(&'static str, Category, Severity, Regex)>> = Lazy::new(|| {
    RULES
        .iter()
        .filter_map(|(name, cat, sev, pattern)| {
            match Regex::new(pattern) {
                Ok(re) => Some((*name, cat.clone(), sev.clone(), re)),
                Err(e) => {
                    eprintln!("Trapline flags.rs: failed to compile rule '{}': {}", name, e);
                    None
                }
            }
        })
        .collect()
});

/// Returns non-overlapping highlight spans for a single output line.
/// When matches overlap, the more severe (then longer) one wins.
pub fn scan_line(line: &str) -> Vec<Span> {
    if line.is_empty() {
        return vec![];
    }
    let compiled = &*COMPILED;
    let mut spans: Vec<Span> = Vec::new();

    for (name, cat, sev, re) in compiled {
        for m in re.find_iter(line) {
            spans.push(Span {
                s: m.start(),
                e: m.end(),
                cat: cat.clone(),
                sev: sev.clone(),
                label: name.to_string(),
                value: m.as_str().to_string(),
            });
        }
    }

    if spans.len() <= 1 {
        return spans;
    }

    // Sort: by start asc, then severity desc, then length desc
    spans.sort_by(|a, b| {
        a.s.cmp(&b.s)
            .then_with(|| sev_weight(&b.sev).cmp(&sev_weight(&a.sev)))
            .then_with(|| (b.e - b.s).cmp(&(a.e - a.s)))
    });

    // Remove overlapping spans — keep the first (highest priority) at each position
    let mut out: Vec<Span> = Vec::new();
    let mut last_end = 0usize;
    for span in spans {
        if span.s >= last_end {
            last_end = span.e;
            out.push(span);
        }
    }
    out
}

/// De-duplicates spans across all lines of a command's output
pub fn aggregate_findings(lines: &[crate::runner::OutLine]) -> Vec<Finding> {
    use std::collections::HashSet;
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<Finding> = Vec::new();

    for ln in lines {
        for span in &ln.spans {
            let key = format!("{}|{}|{}", serde_json::to_string(&span.cat).unwrap_or_default(),
                span.label, span.value);
            if seen.contains(&key) {
                continue;
            }
            seen.insert(key);
            out.push(Finding {
                cat: span.cat.clone(),
                sev: span.sev.clone(),
                name: span.label.clone(),
                value: span.value.clone(),
            });
        }
    }

    // Sort most severe first
    out.sort_by(|a, b| sev_weight(&b.sev).cmp(&sev_weight(&a.sev)));
    out
}

/// Renders the findings body for a Discord embed
pub fn describe_findings(findings: &[Finding]) -> String {
    if findings.is_empty() {
        return "No flags detected — clean output.".to_string();
    }
    let mut buf = String::new();
    let max = 12;
    for (i, f) in findings.iter().enumerate() {
        if i >= max {
            buf.push_str(&format!("…and {} more\n", findings.len() - max));
            break;
        }
        let cat = serde_json::to_string(&f.cat).unwrap_or_default().trim_matches('"').to_string();
        let sev = serde_json::to_string(&f.sev).unwrap_or_default().trim_matches('"').to_string();
        let val = truncate_str(&f.value, 90);
        buf.push_str(&format!("• **{}** _({}/{})_: `{}`\n", f.name, cat, sev, val));
    }
    buf
}

/// Returns Discord embed color for the worst severity in the findings list
pub fn color_for_findings(findings: &[Finding]) -> u32 {
    let best = findings.iter().max_by_key(|f| sev_weight(&f.sev));
    match best.map(|f| &f.sev) {
        Some(Severity::Critical) => 0xff4d4d,
        Some(Severity::High) => 0xff8c42,
        Some(Severity::Medium) => 0xe8c468,
        _ => 0xa98e6b,
    }
}

fn truncate_str(s: &str, n: usize) -> String {
    if s.chars().count() > n {
        let mut t: String = s.chars().take(n - 1).collect();
        t.push('…');
        t
    } else {
        s.to_string()
    }
}
