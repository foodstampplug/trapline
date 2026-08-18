//! Render a `breach::LeakResult` into a shareable report in several formats
//! (markdown / html / csv / json / txt), organized by breach source and led by
//! a "what's leaked" summary. Every report is footered `found by the plug
//! @foodstampplug`. These are explicit, user-initiated exports for authorized
//! client work — unlike the persisted finding, they include cleartext passwords.

use std::collections::BTreeMap;

use chrono::Utc;

use super::breach::LeakResult;

pub const FOOTER: &str = "found by the plug @foodstampplug";

/// Every supported export format → its file extension.
pub fn ext_for(format: &str) -> Option<&'static str> {
    match format {
        "md" => Some("md"),
        "html" => Some("html"),
        "csv" => Some("csv"),
        "json" => Some("json"),
        "txt" => Some("txt"),
        _ => None,
    }
}

struct Counts {
    emails: usize,
    usernames: usize,
    passwords: usize,
    hashes: usize,
    phones: usize,
    ips: usize,
    names: usize,
}

fn counts(r: &LeakResult) -> Counts {
    let n = |f: &dyn Fn(&super::breach::LeakRow) -> bool| r.results.iter().filter(|x| f(x)).count();
    Counts {
        emails: n(&|x| !x.email.is_empty()),
        usernames: n(&|x| !x.username.is_empty()),
        passwords: n(&|x| x.password_present),
        hashes: n(&|x| !x.hash.is_empty()),
        phones: n(&|x| !x.phone.is_empty()),
        ips: n(&|x| !x.ip.is_empty()),
        names: n(&|x| !x.name.is_empty()),
    }
}

/// Group rows by source (breach/table), stable-ordered.
fn by_source(r: &LeakResult) -> BTreeMap<String, Vec<&super::breach::LeakRow>> {
    let mut m: BTreeMap<String, Vec<&super::breach::LeakRow>> = BTreeMap::new();
    for row in &r.results {
        let key = if row.source.is_empty() { "(unknown source)".to_string() } else { row.source.clone() };
        m.entry(key).or_default().push(row);
    }
    m
}

const HEADERS: [&str; 8] = ["Email", "Username", "Password", "Hash", "Phone", "IP", "Name", "Date"];
fn cells(row: &super::breach::LeakRow) -> [String; 8] {
    [
        row.email.clone(),
        row.username.clone(),
        row.password.clone(),
        row.hash.clone(),
        row.phone.clone(),
        row.ip.clone(),
        row.name.clone(),
        row.date.clone(),
    ]
}

fn md_cell(s: &str) -> String {
    let t = s.replace('|', "\\|").replace(['\n', '\r'], " ");
    if t.is_empty() { "—".to_string() } else { t }
}
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}
fn csv_escape(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Render `r` to `(content, extension)`. Unknown format → markdown.
pub fn export(provider: &str, query: &str, r: &LeakResult, format: &str) -> (String, String) {
    let generated = Utc::now().format("%Y-%m-%d %H:%M UTC").to_string();
    let c = counts(r);
    let content = match format {
        "csv" => render_csv(provider, r),
        "json" => render_json(provider, query, &generated, r, &c),
        "html" => render_html(provider, query, &generated, r, &c),
        "txt" => render_txt(provider, query, &generated, r, &c),
        _ => render_md(provider, query, &generated, r, &c),
    };
    (content, ext_for(format).unwrap_or("md").to_string())
}

fn summary_lines(c: &Counts) -> Vec<(&'static str, usize)> {
    vec![
        ("Emails", c.emails),
        ("Usernames", c.usernames),
        ("Passwords (cleartext)", c.passwords),
        ("Hashes", c.hashes),
        ("Phone numbers", c.phones),
        ("IP addresses", c.ips),
        ("Names", c.names),
    ]
}

fn render_md(provider: &str, query: &str, generated: &str, r: &LeakResult, c: &Counts) -> String {
    let mut s = String::new();
    s.push_str(&format!("# Breach Exposure Report — {provider}\n\n"));
    s.push_str(&format!("- **Query:** `{query}`\n- **Records found:** {}\n- **Generated:** {generated}\n\n", r.found));
    s.push_str("> ⚠️ Authorized use only — contains third-party breach credentials.\n\n");
    s.push_str("## What's leaked\n");
    for (label, n) in summary_lines(c) {
        s.push_str(&format!("- **{label}:** {n}\n"));
    }
    s.push_str("\n## Records by source\n\n");
    for (source, rows) in by_source(r) {
        s.push_str(&format!("### {} — {} record(s)\n\n", md_cell(&source), rows.len()));
        s.push_str(&format!("| {} |\n", HEADERS.join(" | ")));
        s.push_str("|---|---|---|---|---|---|---|---|\n");
        for row in rows {
            let cs: Vec<String> = cells(row).iter().map(|x| md_cell(x)).collect();
            s.push_str(&format!("| {} |\n", cs.join(" | ")));
        }
        s.push('\n');
    }
    s.push_str(&format!("---\n_{FOOTER}_\n"));
    s
}

fn render_txt(provider: &str, query: &str, generated: &str, r: &LeakResult, c: &Counts) -> String {
    let mut s = String::new();
    s.push_str(&format!("BREACH EXPOSURE REPORT — {provider}\n"));
    s.push_str(&format!("Query: {query}\nRecords found: {}\nGenerated: {generated}\n", r.found));
    s.push_str("Authorized use only — contains third-party breach credentials.\n\n");
    s.push_str("WHAT'S LEAKED\n");
    for (label, n) in summary_lines(c) {
        s.push_str(&format!("  {label}: {n}\n"));
    }
    s.push_str("\nRECORDS BY SOURCE\n");
    for (source, rows) in by_source(r) {
        s.push_str(&format!("\n== {} ({}) ==\n", source, rows.len()));
        for row in rows {
            let cs = cells(row);
            let mut parts = Vec::new();
            for (h, v) in HEADERS.iter().zip(cs.iter()) {
                if !v.is_empty() {
                    parts.push(format!("{h}: {v}"));
                }
            }
            s.push_str(&format!("  - {}\n", parts.join("  |  ")));
        }
    }
    s.push_str(&format!("\n---\n{FOOTER}\n"));
    s
}

fn render_csv(_provider: &str, r: &LeakResult) -> String {
    let mut s = String::from("source,email,username,password,hash,phone,ip,name,date\n");
    for row in &r.results {
        let cs = [
            &row.source, &row.email, &row.username, &row.password, &row.hash, &row.phone, &row.ip, &row.name, &row.date,
        ];
        let line: Vec<String> = cs.iter().map(|x| csv_escape(x)).collect();
        s.push_str(&line.join(","));
        s.push('\n');
    }
    s.push_str(&format!("\n# {FOOTER}\n"));
    s
}

fn render_json(provider: &str, query: &str, generated: &str, r: &LeakResult, c: &Counts) -> String {
    let whats_leaked: serde_json::Map<String, serde_json::Value> = summary_lines(c)
        .into_iter()
        .map(|(k, n)| (k.to_string(), serde_json::json!(n)))
        .collect();
    let doc = serde_json::json!({
        "provider": provider,
        "query": query,
        "generated": generated,
        "found": r.found,
        "authorizedUseOnly": true,
        "whatsLeaked": whats_leaked,
        "sources": r.sources,
        "records": r.results,
        "footer": FOOTER,
    });
    serde_json::to_string_pretty(&doc).unwrap_or_default()
}

fn render_html(provider: &str, query: &str, generated: &str, r: &LeakResult, c: &Counts) -> String {
    let mut body = String::new();
    body.push_str(&format!("<h1>Breach Exposure Report — {}</h1>", html_escape(provider)));
    body.push_str(&format!(
        "<p class=\"meta\"><b>Query:</b> <code>{}</code> &nbsp;·&nbsp; <b>Records:</b> {} &nbsp;·&nbsp; <b>Generated:</b> {}</p>",
        html_escape(query), r.found, html_escape(generated)
    ));
    body.push_str("<p class=\"warn\">⚠️ Authorized use only — contains third-party breach credentials.</p>");
    body.push_str("<h2>What's leaked</h2><ul class=\"summary\">");
    for (label, n) in summary_lines(c) {
        body.push_str(&format!("<li><b>{label}:</b> {n}</li>"));
    }
    body.push_str("</ul><h2>Records by source</h2>");
    for (source, rows) in by_source(r) {
        body.push_str(&format!("<h3>{} <span class=\"cnt\">{}</span></h3>", html_escape(&source), rows.len()));
        body.push_str("<div class=\"tw\"><table><thead><tr>");
        for h in HEADERS {
            body.push_str(&format!("<th>{h}</th>"));
        }
        body.push_str("</tr></thead><tbody>");
        for row in rows {
            body.push_str("<tr>");
            for v in cells(row) {
                let cell = if v.is_empty() { "—".to_string() } else { html_escape(&v) };
                body.push_str(&format!("<td>{cell}</td>"));
            }
            body.push_str("</tr>");
        }
        body.push_str("</tbody></table></div>");
    }
    body.push_str(&format!("<footer>{}</footer>", html_escape(FOOTER)));

    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Breach Report — {}</title><style>\
body{{font:14px/1.5 -apple-system,Segoe UI,Roboto,sans-serif;color:#1a1c22;background:#f6f7f9;margin:0;padding:28px;}}\
h1{{font-size:22px;margin:0 0 4px;}}h2{{font-size:16px;margin:22px 0 8px;border-bottom:2px solid #e6e8ec;padding-bottom:4px;}}\
h3{{font-size:13px;margin:16px 0 6px;color:#3a3f4b;}}h3 .cnt{{color:#8a90a0;font-weight:400;}}\
.meta{{color:#5f6773;}}code{{background:#eceef2;padding:2px 5px;border-radius:4px;}}\
.warn{{color:#b00020;background:#fdecef;border:1px solid #f3b6c0;border-radius:6px;padding:8px 11px;font-weight:600;}}\
ul.summary{{columns:2;}}.tw{{overflow-x:auto;}}table{{border-collapse:collapse;width:100%;font-size:12px;background:#fff;}}\
th{{text-align:left;background:#f0f2f5;padding:6px 9px;border:1px solid #e2e5ea;white-space:nowrap;}}\
td{{padding:5px 9px;border:1px solid #eceef2;white-space:nowrap;}}\
footer{{margin-top:26px;color:#8a90a0;font-style:italic;font-size:12px;}}\
</style></head><body>{body}</body></html>",
        html_escape(provider)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::breach::{LeakResult, LeakRow};

    fn sample() -> LeakResult {
        LeakResult {
            found: 2,
            sources: vec![],
            results: vec![
                LeakRow { email: "neo@acme.com".into(), username: "neo".into(), password: "hunter2".into(), password_present: true, source: "BreachX".into(), ..Default::default() },
                LeakRow { email: "trin@acme.com".into(), source: "BreachY".into(), ..Default::default() },
            ],
        }
    }

    #[test]
    fn every_format_has_footer_and_data() {
        for fmt in ["md", "html", "csv", "json", "txt"] {
            let (content, ext) = export("Snusbase", "acme.com", &sample(), fmt);
            assert_eq!(ext, fmt);
            assert!(content.contains(FOOTER), "{fmt} missing footer");
            assert!(content.contains("neo@acme.com"), "{fmt} missing data");
            // exports DO include cleartext passwords (explicit user action):
            assert!(content.contains("hunter2"), "{fmt} should include the password");
        }
    }

    #[test]
    fn csv_escapes_commas_and_quotes() {
        let mut r = sample();
        r.results[0].name = "Doe, \"John\"".into();
        let (csv, _) = export("X", "q", &r, "csv");
        assert!(csv.contains("\"Doe, \"\"John\"\"\""));
    }

    #[test]
    fn markdown_groups_by_source_and_summarizes() {
        let (md, _) = export("DeHashed", "acme.com", &sample(), "md");
        assert!(md.contains("### BreachX"));
        assert!(md.contains("### BreachY"));
        assert!(md.contains("What's leaked"));
        assert!(md.contains("Passwords (cleartext):** 1"));
    }

    #[test]
    fn html_escapes_untrusted_values() {
        let mut r = sample();
        r.results[0].username = "<script>x</script>".into();
        let (html, _) = export("X", "q", &r, "html");
        assert!(!html.contains("<script>x</script>"));
        assert!(html.contains("&lt;script&gt;"));
    }
}
