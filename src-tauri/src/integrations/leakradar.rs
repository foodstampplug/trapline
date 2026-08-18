//! LeakRadar client — DEFENSIVE / BEST-EFFORT. Base `https://api.leakradar.io`.
//! ⚠️ The exact endpoint paths and auth-header name are best-effort — LeakRadar's
//! docs (docs.leakradar.io) wouldn't load during integration, so CONFIRM these
//! against your key and tweak the constants below if a call errors. Parsing is
//! defensive: an unexpected shape degrades to an empty result, never a crash.
//! Records are info-stealer credentials (url = the site the login belongs to).

use serde::Deserialize;

use super::breach::{LeakResult, LeakRow};

const BASE: &str = "https://api.leakradar.io";
/// ⚠️ CONFIRM: LeakRadar auth header (best guess — swap if the API rejects it).
const AUTH_HEADER: &str = "X-API-Key";

pub const KINDS: &[&str] = &["email", "domain", "raw"];

pub fn valid_kind(k: &str) -> bool {
    KINDS.contains(&k)
}

/// ⚠️ CONFIRM endpoint paths — map a kind to its search path.
pub fn endpoint(kind: &str) -> &'static str {
    match kind {
        "email" => "/emails/search",
        "domain" => "/domains/search",
        _ => "/search",
    }
}

#[derive(Deserialize, Default)]
struct RawRow {
    #[serde(default)]
    email: String,
    #[serde(default)]
    email_host: String,
    #[serde(default)]
    email_domain: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    source: String,
    #[serde(default)]
    date: String,
}

#[derive(Deserialize, Default)]
struct RawResponse {
    #[serde(default)]
    total: u64,
    #[serde(default)]
    count: u64,
    // Tolerate a few likely container keys.
    #[serde(default)]
    results: Vec<RawRow>,
    #[serde(default)]
    data: Vec<RawRow>,
    #[serde(default)]
    items: Vec<RawRow>,
}

pub fn parse(body: &str) -> LeakResult {
    let raw: RawResponse = serde_json::from_str(body).unwrap_or_default();
    let rows: Vec<RawRow> = if !raw.results.is_empty() {
        raw.results
    } else if !raw.data.is_empty() {
        raw.data
    } else {
        raw.items
    };
    let mut out = LeakResult { found: raw.total.max(raw.count), ..Default::default() };
    for row in rows {
        let email = if !row.email.is_empty() {
            row.email
        } else if !row.email_host.is_empty() && !row.email_domain.is_empty() {
            format!("{}@{}", row.email_host, row.email_domain)
        } else {
            String::new()
        };
        let src = if !row.source.is_empty() { row.source } else { row.url.clone() };
        out.add_source(&src, "");
        let pw = row.password;
        out.results.push(LeakRow {
            email,
            username: row.username,
            password_present: !pw.is_empty(),
            password: pw,
            source: src,
            date: row.date,
            ..Default::default()
        });
    }
    if out.found == 0 {
        out.found = out.results.len() as u64;
    }
    out
}

pub async fn query(key: &str, value: &str, kind: &str) -> Result<LeakResult, String> {
    if key.trim().is_empty() {
        return Err("LeakRadar API key not set — add it in Settings".into());
    }
    let url = format!("{BASE}{}", endpoint(kind));
    let text = super::client()
        .get(url)
        .header(AUTH_HEADER, key)
        // `reveal=true` asks for plaintext (LeakRadar charges credits per reveal).
        .query(&[("query", value), ("reveal", "true")])
        .send()
        .await
        .map_err(|e| e.to_string().replace(key, "***"))?
        .text()
        .await
        .map_err(|e| e.to_string().replace(key, "***"))?;
    Ok(parse(&text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::breach::redacted_json;

    #[test]
    fn endpoints_and_kinds() {
        assert_eq!(endpoint("email"), "/emails/search");
        assert_eq!(endpoint("domain"), "/domains/search");
        assert_eq!(endpoint("raw"), "/search");
        assert!(valid_kind("email") && valid_kind("raw"));
        assert!(!valid_kind("bogus"));
    }

    #[test]
    fn parse_maps_common_fields_defensively() {
        let sample = r#"{"total":1,"results":[
          {"username":"neo","password":"hunter2","email_host":"neo","email_domain":"acme.com","url":"https://portal.acme.com"}
        ]}"#;
        let r = parse(sample);
        assert_eq!(r.found, 1);
        let row = &r.results[0];
        assert_eq!(row.username, "neo");
        assert_eq!(row.password, "hunter2");
        assert_eq!(row.email, "neo@acme.com"); // composed from host+domain
        assert_eq!(row.source, "https://portal.acme.com"); // url = the login's site
        assert!(!redacted_json(&r).contains("hunter2"));
    }

    #[test]
    fn parse_tolerates_empty_and_alt_containers() {
        assert_eq!(parse("{}").found, 0);
        let alt = r#"{"count":1,"data":[{"username":"x","password":"p"}]}"#;
        assert_eq!(parse(alt).results.len(), 1);
    }
}
