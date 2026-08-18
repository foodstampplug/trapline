//! LeakCheck v2 client. Parses the LeakCheck wire format into the shared
//! `breach::LeakResult`. The API key travels only in the `X-API-Key` header,
//! never the URL, so it can't leak into request-line logs.

use serde::Deserialize;

use super::breach::{LeakResult, LeakRow};

const BASE: &str = "https://leakcheck.io/api/v2/query";

pub fn query_url(value: &str) -> String {
    format!("{BASE}/{value}")
}

#[derive(Deserialize, Default)]
struct RawSource {
    #[serde(default)]
    name: String,
    #[serde(default)]
    breach_date: String,
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
    phone: String,
    #[serde(default)]
    first_name: String,
    #[serde(default)]
    last_name: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    source: RawSource,
}

#[derive(Deserialize, Default)]
struct RawResponse {
    #[serde(default)]
    found: u64,
    #[serde(default)]
    result: Vec<RawRow>,
}

/// Defensive parse: unknown/empty shape -> empty-but-Ok, never a panic.
pub fn parse(body: &str) -> LeakResult {
    let raw: RawResponse = serde_json::from_str(body).unwrap_or_default();
    let mut out = LeakResult { found: raw.found, ..Default::default() };
    for row in raw.result {
        out.add_source(&row.source.name, &row.source.breach_date);
        let pw = row.password;
        let full_name = if !row.name.is_empty() {
            row.name
        } else {
            format!("{} {}", row.first_name, row.last_name).trim().to_string()
        };
        out.results.push(LeakRow {
            email: row.email,
            username: row.username,
            password_present: !pw.is_empty(),
            password: pw,
            phone: row.phone,
            name: full_name,
            source: row.source.name,
            date: row.source.breach_date,
            ..Default::default() // hash/ip empty for LeakCheck
        });
    }
    out
}

/// Query LeakCheck v2 for `value` (`kind` = the search type). Key in header only.
pub async fn query(key: &str, value: &str, kind: &str) -> Result<LeakResult, String> {
    if key.trim().is_empty() {
        return Err("LeakCheck API key not set — add it in Settings".into());
    }
    let body = super::client()
        .get(query_url(value))
        .header("X-API-Key", key)
        .query(&[("type", kind)])
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    Ok(parse(&body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::breach::redacted_json;

    #[test]
    fn url_targets_v2_query() {
        assert_eq!(query_url("a@b.test"), "https://leakcheck.io/api/v2/query/a@b.test");
    }

    #[test]
    fn parse_retains_full_row_incl_password_but_at_rest_strips_it() {
        let sample = r#"{"found":1,"result":[{"email":"neo@acme.com","username":"neo","password":"hunter2","phone":"+1555","first_name":"Thomas","last_name":"Anderson","source":{"name":"BreachX","breach_date":"2020-01"}}]}"#;
        let r = parse(sample);
        let row = &r.results[0];
        assert_eq!(row.email, "neo@acme.com");
        assert_eq!(row.username, "neo");
        assert_eq!(row.password, "hunter2"); // live card gets the plaintext
        assert!(row.password_present);
        assert_eq!(row.name, "Thomas Anderson");
        assert_eq!(row.source, "BreachX");
        assert_eq!(row.date, "2020-01");
        assert!(serde_json::to_string(&r).unwrap().contains("hunter2"));
        assert!(!redacted_json(&r).contains("hunter2")); // ...but never at rest
    }

    #[test]
    fn parse_tolerates_missing_fields() {
        let r = parse("{}");
        assert_eq!(r.found, 0);
        assert!(r.results.is_empty());
        assert!(r.sources.is_empty());
    }

    #[test]
    fn parse_dedups_sources_by_name() {
        let sample = r#"{"found":2,"result":[
          {"email":"a@b.test","source":{"name":"BreachX","breach_date":"2020-01"}},
          {"email":"c@b.test","source":{"name":"BreachX","breach_date":"2020-01"}}
        ]}"#;
        assert_eq!(parse(sample).sources.len(), 1);
    }

    #[test]
    fn async_query_rejects_empty_key() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        assert!(rt.block_on(query("", "a@b.test", "email")).unwrap_err().contains("LeakCheck API key not set"));
        assert!(rt.block_on(query("  ", "acme.com", "domain")).unwrap_err().contains("LeakCheck API key not set"));
    }
}
