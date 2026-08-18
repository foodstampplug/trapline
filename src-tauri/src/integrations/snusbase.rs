//! Snusbase client. `POST https://api.snusbase.com/data/search`, key in the
//! `Auth` header. Response groups records by breach table; each table becomes a
//! `source`. Passwords are cleartext. Parses into the shared `breach::LeakResult`.

use serde::Deserialize;
use std::collections::HashMap;

use super::breach::{LeakResult, LeakRow};

const URL: &str = "https://api.snusbase.com/data/search";

/// Snusbase search types (docs.snusbase.com).
pub const TYPES: &[&str] = &["email", "username", "lastip", "password", "hash", "name", "_domain"];

pub fn valid_type(t: &str) -> bool {
    TYPES.contains(&t)
}

#[derive(Deserialize, Default)]
struct RawRow {
    #[serde(default)]
    email: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    hash: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    lastip: String,
    #[serde(default)]
    created: String,
    #[serde(default)]
    updated: String,
}

#[derive(Deserialize, Default)]
struct RawResponse {
    #[serde(default)]
    size: u64,
    /// Keyed by breach table name.
    #[serde(default)]
    results: HashMap<String, Vec<RawRow>>,
}

pub fn parse(body: &str) -> LeakResult {
    let raw: RawResponse = serde_json::from_str(body).unwrap_or_default();
    let mut out = LeakResult { found: raw.size, ..Default::default() };
    for (table, rows) in raw.results {
        out.add_source(&table, "");
        for row in rows {
            let pw = row.password;
            let date = if !row.created.is_empty() { row.created } else { row.updated };
            out.results.push(LeakRow {
                email: row.email,
                username: row.username,
                password_present: !pw.is_empty(),
                password: pw,
                name: row.name,
                hash: row.hash,
                ip: row.lastip,
                source: table.clone(),
                date,
                ..Default::default()
            });
        }
    }
    if out.found == 0 {
        out.found = out.results.len() as u64;
    }
    out
}

pub async fn query(key: &str, value: &str, kind: &str) -> Result<LeakResult, String> {
    if key.trim().is_empty() {
        return Err("Snusbase API key not set — add it in Settings".into());
    }
    let payload = serde_json::json!({ "terms": [value], "types": [kind] });
    let text = super::client()
        .post(URL)
        .header("Auth", key)
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    Ok(parse(&text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::breach::redacted_json;

    #[test]
    fn parse_flattens_tables_and_keeps_password_live() {
        let sample = r#"{"took":5,"size":2,"results":{
          "Collection1":[{"email":"a@b.test","username":"neo","password":"hunter2","hash":"abc","lastip":"1.2.3.4","name":"Neo","created":"2019-01"}],
          "SomeForum":[{"email":"c@d.test","password":"","hash":"","username":"trin"}]
        }}"#;
        let r = parse(sample);
        assert_eq!(r.found, 2);
        assert_eq!(r.results.len(), 2);
        let neo = r.results.iter().find(|x| x.username == "neo").unwrap();
        assert_eq!(neo.password, "hunter2");
        assert!(neo.password_present);
        assert_eq!(neo.hash, "abc");
        assert_eq!(neo.ip, "1.2.3.4");
        assert_eq!(neo.name, "Neo");
        assert_eq!(neo.date, "2019-01");
        assert!(r.sources.iter().any(|s| s.name == "Collection1"));
        // the table name is the source:
        assert!(r.results.iter().any(|x| x.source == "SomeForum"));
        // at-rest strips the password:
        assert!(!redacted_json(&r).contains("hunter2"));
        assert!(serde_json::to_string(&r).unwrap().contains("hunter2"));
    }

    #[test]
    fn parse_tolerates_empty() {
        let r = parse("{}");
        assert_eq!(r.found, 0);
        assert!(r.results.is_empty());
    }

    #[test]
    fn type_allowlist() {
        assert!(valid_type("email") && valid_type("_domain") && valid_type("password"));
        assert!(!valid_type("bogus"));
    }
}
