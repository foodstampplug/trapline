//! LeakRadar client (api.leakradar.io). Auth is `Authorization: Bearer <key>`.
//! Email/username search: `POST /search/email` (body `{email}`). Domain search:
//! `GET /search/domain/{domain}/all`. `auto_unlock=true` returns plaintext
//! (costs LeakRadar credits). Records are info-stealer creds (url = login site).
//! Parses into the shared `breach::LeakResult`; defensive over the container key.

use serde::Deserialize;

use super::breach::{LeakResult, LeakRow};

const BASE: &str = "https://api.leakradar.io";

pub const KINDS: &[&str] = &["email", "domain", "raw"];

pub fn valid_kind(k: &str) -> bool {
    KINDS.contains(&k)
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
    // Tolerate the likely container keys across the email/domain endpoints.
    #[serde(default)]
    leaks: Vec<RawRow>,
    #[serde(default)]
    results: Vec<RawRow>,
    #[serde(default)]
    data: Vec<RawRow>,
    #[serde(default)]
    items: Vec<RawRow>,
}

pub fn parse(body: &str) -> LeakResult {
    let raw: RawResponse = serde_json::from_str(body).unwrap_or_default();
    let rows: Vec<RawRow> = [raw.leaks, raw.results, raw.data, raw.items]
        .into_iter()
        .find(|v| !v.is_empty())
        .unwrap_or_default();
    let mut out = LeakResult { found: raw.total.max(raw.count), ..Default::default() };
    for row in rows {
        let email = if !row.email.is_empty() {
            row.email
        } else if !row.email_host.is_empty() && !row.email_domain.is_empty() {
            format!("{}@{}", row.email_host, row.email_domain)
        } else {
            String::new()
        };
        // `source` = the breach name if given, else the login URL (stealer logs).
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
    let client = super::client();
    // `auto_unlock=true` reveals plaintext creds (charges credits per record).
    let req = if kind == "domain" {
        client
            .get(format!("{BASE}/search/domain/{value}/all"))
            .query(&[("auto_unlock", "true"), ("page_size", "100")])
    } else {
        // email or raw → email/username search
        client
            .post(format!("{BASE}/search/email"))
            .query(&[("auto_unlock", "true"), ("page_size", "100")])
            .json(&serde_json::json!({ "email": value }))
    };
    let text = req
        .header("Authorization", format!("Bearer {key}"))
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
    fn kinds() {
        assert!(valid_kind("email") && valid_kind("domain") && valid_kind("raw"));
        assert!(!valid_kind("bogus"));
    }

    #[test]
    fn parse_maps_leakdetails_fields() {
        // LeakDetails: username/password/email/url/source (docs.leakradar.io).
        let sample = r#"{"total":1,"leaks":[
          {"username":"neo","password":"hunter2","email":"neo@acme.com","url":"https://portal.acme.com","source":"StealerLog-2024"}
        ]}"#;
        let r = parse(sample);
        assert_eq!(r.found, 1);
        let row = &r.results[0];
        assert_eq!(row.username, "neo");
        assert_eq!(row.password, "hunter2");
        assert_eq!(row.email, "neo@acme.com");
        assert_eq!(row.source, "StealerLog-2024"); // breach name preferred over url
        assert!(!redacted_json(&r).contains("hunter2"));
    }

    #[test]
    fn parse_tolerates_empty_and_alt_containers() {
        assert_eq!(parse("{}").found, 0);
        assert_eq!(parse(r#"{"results":[{"username":"x","password":"p","url":"http://x"}]}"#).results.len(), 1);
    }
}
