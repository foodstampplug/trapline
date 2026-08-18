//! DeHashed v2 client. `POST https://api.dehashed.com/v2/search`, key in the
//! `Dehashed-Api-Key` header. v2 returns each entry's fields as arrays (or
//! strings) — the `field()` helper tolerates both. `database_name` → source.
//! Passwords are cleartext. Parses into the shared `breach::LeakResult`.
//!
//! NOTE: the exact v2 field names / array-vs-string shape are confirmed against
//! a live key at test time (their docs are behind auth); parsing is defensive
//! so an unexpected shape degrades to empty rather than failing.

use serde::Deserialize;

use super::breach::{LeakResult, LeakRow};

const URL: &str = "https://api.dehashed.com/v2/search";

/// DeHashed searchable fields (used to build `field:value` queries).
pub const FIELDS: &[&str] =
    &["email", "username", "password", "hashed_password", "name", "phone", "ip_address", "domain"];

pub fn valid_kind(kind: &str) -> bool {
    kind == "raw" || FIELDS.contains(&kind)
}

/// Build the DeHashed query string. `raw`/empty → the value verbatim (the user
/// may pass their own `field:value`); otherwise `kind:"value"`.
pub fn build_query(value: &str, kind: &str) -> String {
    if kind == "raw" || kind.is_empty() {
        value.to_string()
    } else if value.contains(' ') {
        // Quote multi-word values (e.g. name:"John Doe") so the space isn't AND.
        format!("{kind}:\"{value}\"")
    } else {
        // DeHashed v2 wants UNQUOTED field:value (e.g. domain:example.com —
        // quoting a domain returned nothing).
        format!("{kind}:{value}")
    }
}

#[derive(Deserialize, Default)]
struct RawResponse {
    #[serde(default)]
    total: u64,
    #[serde(default)]
    entries: Vec<serde_json::Value>,
}

/// Extract a field that may be a string, a number, or an array of strings.
fn field(entry: &serde_json::Value, key: &str) -> String {
    match &entry[key] {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Array(a) => {
            a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect::<Vec<_>>().join(", ")
        }
        _ => String::new(),
    }
}

pub fn parse(body: &str) -> LeakResult {
    let raw: RawResponse = serde_json::from_str(body).unwrap_or_default();
    let mut out = LeakResult { found: raw.total, ..Default::default() };
    for e in &raw.entries {
        let db = field(e, "database_name");
        out.add_source(&db, "");
        let pw = field(e, "password");
        out.results.push(LeakRow {
            email: field(e, "email"),
            username: field(e, "username"),
            password_present: !pw.is_empty(),
            password: pw,
            phone: field(e, "phone"),
            name: field(e, "name"),
            hash: field(e, "hashed_password"),
            ip: field(e, "ip_address"),
            source: db,
            date: String::new(),
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
        return Err("DeHashed API key not set — add it in Settings".into());
    }
    let payload = serde_json::json!({ "query": build_query(value, kind), "page": 1, "size": 100 });
    let text = super::client()
        .post(URL)
        .header("Dehashed-Api-Key", key)
        .header("Accept", "application/json")
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
    fn query_builder() {
        assert_eq!(build_query("a@b.test", "email"), "email:a@b.test");
        assert_eq!(build_query("example.com", "domain"), "domain:example.com");
        assert_eq!(build_query("John Doe", "name"), "name:\"John Doe\""); // multi-word quoted
        assert_eq!(build_query("email:x", "raw"), "email:x");
        assert!(valid_kind("email") && valid_kind("raw"));
        assert!(!valid_kind("bogus"));
    }

    #[test]
    fn parse_handles_array_and_string_fields() {
        // v2 typically returns arrays; tolerate a plain string too.
        let sample = r#"{"total":1,"entries":[
          {"id":"1","email":["neo@acme.com"],"username":["neo"],"password":["hunter2"],
           "hashed_password":["abc"],"ip_address":["1.2.3.4"],"name":["Thomas Anderson"],
           "phone":"5551234","database_name":"BreachX"}
        ]}"#;
        let r = parse(sample);
        assert_eq!(r.found, 1);
        let row = &r.results[0];
        assert_eq!(row.email, "neo@acme.com");
        assert_eq!(row.username, "neo");
        assert_eq!(row.password, "hunter2");
        assert!(row.password_present);
        assert_eq!(row.hash, "abc");
        assert_eq!(row.ip, "1.2.3.4");
        assert_eq!(row.phone, "5551234"); // string form tolerated
        assert_eq!(row.source, "BreachX");
        assert!(!redacted_json(&r).contains("hunter2"));
    }

    #[test]
    fn parse_tolerates_empty() {
        assert_eq!(parse("{}").found, 0);
        assert!(parse("garbage").results.is_empty());
    }
}
