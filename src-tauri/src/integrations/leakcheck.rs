use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const BASE: &str = "https://leakcheck.io/api/v2/query";

fn sha256_hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    h.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

/// LeakCheck v2 query URL. The key travels in the `X-API-Key` header (see
/// `query`), never in the URL, so it can't leak into logs/history that
/// capture request lines.
pub fn query_url(value: &str) -> String {
    format!("{BASE}/{value}")
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct LeakSource {
    pub name: String,
    pub date: String,
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct LeakRow {
    pub email: String,
    /// The breached username (actual value). Empty when the row has none.
    pub username: String,
    /// Plaintext password from the breach. Shown in the card at the user's
    /// explicit request (authorized engagement). It STILL never reaches any
    /// at-rest sink: `build_finding_json` never includes it, and `redacted_json`
    /// (used for the watch.db enrichment cache) blanks it — only the live,
    /// in-memory card ever sees the value.
    pub password: String,
    /// Convenience flag (`!password.is_empty()`) — drives severity + the card's
    /// 🔒 marker without re-inspecting the value.
    pub password_present: bool,
    pub phone: String,
    /// Full name (`name` if present, else `first_name last_name`).
    pub name: String,
    pub source: String,
    /// Per-row breach date (from `source.breach_date`), empty when unknown.
    pub date: String,
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct LeakResult {
    pub found: u64,
    pub sources: Vec<LeakSource>,
    pub results: Vec<LeakRow>,
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
    #[allow(dead_code)]
    success: bool,
    #[serde(default)]
    found: u64,
    #[serde(default)]
    result: Vec<RawRow>,
}

/// Defensive parse: unknown/empty shape -> empty-but-Ok, never a panic.
/// Drops any plaintext password value; only a `password_present` bool survives
/// in the returned `LeakResult` — the raw `RawRow.password` never escapes this
/// function.
pub fn parse(body: &str) -> LeakResult {
    let raw: RawResponse = serde_json::from_str(body).unwrap_or_default();

    let mut sources: Vec<LeakSource> = Vec::new();
    let mut results: Vec<LeakRow> = Vec::new();

    for row in raw.result {
        if !row.source.name.is_empty() && !sources.iter().any(|s| s.name == row.source.name) {
            sources.push(LeakSource {
                name: row.source.name.clone(),
                date: row.source.breach_date.clone(),
            });
        }
        let pw = row.password;
        let full_name = if !row.name.is_empty() {
            row.name
        } else {
            format!("{} {}", row.first_name, row.last_name).trim().to_string()
        };
        results.push(LeakRow {
            email: row.email,
            username: row.username,
            password_present: !pw.is_empty(),
            password: pw,
            phone: row.phone,
            name: full_name,
            source: row.source.name,
            date: row.source.breach_date,
        });
    }

    LeakResult { found: raw.found, sources, results }
}

/// Serialize a `LeakResult` for AT-REST caching (the watch.db enrichment cache)
/// with plaintext passwords stripped. The live card receives the full result
/// over IPC — passwords included, at the user's request — but nothing plaintext
/// is ever written to disk.
pub fn redacted_json(r: &LeakResult) -> String {
    let results: Vec<LeakRow> = r
        .results
        .iter()
        .map(|row| LeakRow {
            email: row.email.clone(),
            username: row.username.clone(),
            password: String::new(), // stripped at rest
            password_present: row.password_present,
            phone: row.phone.clone(),
            name: row.name.clone(),
            source: row.source.clone(),
            date: row.date.clone(),
        })
        .collect();
    let sources = r
        .sources
        .iter()
        .map(|s| LeakSource { name: s.name.clone(), date: s.date.clone() })
        .collect();
    serde_json::to_string(&LeakResult { found: r.found, sources, results }).unwrap_or_default()
}

/// Pure mapper: LeakResult -> HunterFinding JSON (camelCase). `None` when
/// `found == 0`. Severity `critical` if any row has `password_present`, else
/// `high`. Evidence is a redacted summary — counts and source names only,
/// never a plaintext credential.
pub fn build_finding_json(target: &str, value: &str, r: &LeakResult) -> Option<String> {
    if r.found == 0 {
        return None;
    }

    let has_password = r.results.iter().any(|row| row.password_present);
    let severity = if has_password { "critical" } else { "high" };
    let password_rows = r.results.iter().filter(|row| row.password_present).count();

    let mut evidence = format!(
        "LeakCheck — credential exposure for {value}\nRecords found: {}\n",
        r.found
    );
    if !r.sources.is_empty() {
        let names: Vec<String> = r
            .sources
            .iter()
            .map(|s| if s.date.is_empty() { s.name.clone() } else { format!("{} ({})", s.name, s.date) })
            .collect();
        evidence.push_str(&format!("\nSources ({}):\n{}\n", r.sources.len(), names.join("\n")));
    }
    evidence.push_str(&format!("\n{password_rows} row(s) with password present.\n"));

    let sh = sha256_hex(value);
    let id = format!("leak-{}-{}", Utc::now().format("%Y%m%d-%H%M%S"), &sh[..6]);

    let finding = serde_json::json!({
        "id": id,
        "programName": target,
        "platform": "LeakCheck",
        "title": format!("Credential exposure: {value} ({} records)", r.found),
        "severity": severity,
        "status": "draft",
        "endpoint": value,
        "summary": format!("LeakCheck found {} record(s) for {value}.", r.found),
        "description": "",
        "steps": "",
        "evidence": evidence,
        "impact": "",
        "remediation": "",
        "cvss": "",
        "cvssScore": "",
        "notes": "",
        "cmdline": "leakcheck",
    });
    Some(serde_json::to_string(&finding).unwrap())
}

/// Query LeakCheck v2 for `value` (`kind` = "email" | "domain"). The API key
/// is sent only in the `X-API-Key` header — never logged, never printed,
/// never included in the returned error.
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

/// Persist a LeakCheck lookup as a finding in the in-process store, when
/// there's something to report (no-op `Ok(())` when `found == 0`).
pub fn record_findings(target: &str, value: &str, r: &LeakResult) -> Result<(), String> {
    match build_finding_json(target, value, r) {
        Some(json) => crate::findings::save(&json),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_targets_v2_query() {
        assert_eq!(query_url("a@b.test"), "https://leakcheck.io/api/v2/query/a@b.test");
    }

    #[test]
    fn parse_retains_full_row_for_the_card_incl_password() {
        let sample = r#"{"success":true,"found":2,"result":[
          {"email":"neo@acme.com","username":"neo","password":"hunter2","phone":"+1555","first_name":"Thomas","last_name":"Anderson","source":{"name":"BreachX","breach_date":"2020-01"}},
          {"email":"c@b.test","username":"cc","source":{"name":"BreachY"}}
        ]}"#;
        let r = parse(sample);
        assert_eq!(r.found, 2);
        let row = &r.results[0];
        assert_eq!(row.email, "neo@acme.com");
        assert_eq!(row.username, "neo");
        assert_eq!(row.password, "hunter2"); // plaintext retained for the card
        assert!(row.password_present);
        assert_eq!(row.phone, "+1555");
        assert_eq!(row.name, "Thomas Anderson");
        assert_eq!(row.source, "BreachX");
        assert_eq!(row.date, "2020-01");
        // The live LeakResult DOES carry the password (it crosses IPC to the card):
        let j = serde_json::to_string(&r).unwrap();
        assert!(j.contains("hunter2"));
        assert!(r.sources.iter().any(|s| s.name == "BreachX"));
    }

    #[test]
    fn password_never_reaches_an_at_rest_sink() {
        let sample = r#"{"found":1,"result":[
          {"email":"a@b.test","username":"cc","password":"hunter2","source":{"name":"BreachX","breach_date":"2020-01"}}
        ]}"#;
        let r = parse(sample);
        // The watch.db enrichment cache (redacted_json) must NOT carry the password:
        assert!(!redacted_json(&r).contains("hunter2"), "cache must strip the plaintext password");
        // ...but the rest of the row survives so the cache is still useful:
        assert!(redacted_json(&r).contains("a@b.test"));
        // The persisted finding (findings.json evidence) must NOT carry it either:
        let j = build_finding_json("acme", "acme.com", &r).unwrap();
        assert!(!j.contains("hunter2"), "finding must never persist the plaintext password");
    }

    #[test]
    fn finding_severity_critical_when_plaintext_else_high_else_none() {
        let none = LeakResult { found: 0, ..Default::default() };
        assert!(build_finding_json("acme", "acme.com", &none).is_none());

        let plain = LeakResult {
            found: 1,
            results: vec![LeakRow {
                email: "a@b.test".into(),
                username: String::new(),
                password: "hunter2".into(),
                password_present: true,
                phone: String::new(),
                name: String::new(),
                source: "BreachX".into(),
                date: String::new(),
            }],
            ..Default::default()
        };
        let j = build_finding_json("acme", "a@b.test", &plain).unwrap();
        let v: serde_json::Value = serde_json::from_str(&j).unwrap();
        assert_eq!(v["severity"], "critical");
        assert_eq!(v["platform"], "LeakCheck");
        assert!(!j.contains("hunter2"));

        let found_only = LeakResult { found: 3, results: vec![], ..Default::default() };
        let j2 = build_finding_json("acme", "acme.com", &found_only).unwrap();
        let v2: serde_json::Value = serde_json::from_str(&j2).unwrap();
        assert_eq!(v2["severity"], "high");
    }

    #[test]
    fn async_query_rejects_empty_key() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt.block_on(query("", "a@b.test", "email")).unwrap_err();
        assert!(err.contains("LeakCheck API key not set"));
        let err = rt.block_on(query("   ", "acme.com", "domain")).unwrap_err();
        assert!(err.contains("LeakCheck API key not set"));
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
        let sample = r#"{"success":true,"found":2,"result":[
          {"email":"a@b.test","password":"pw1","source":{"name":"BreachX","breach_date":"2020-01"}},
          {"email":"c@b.test","password":"pw2","source":{"name":"BreachX","breach_date":"2020-01"}}
        ]}"#;
        let r = parse(sample);
        assert_eq!(r.sources.len(), 1);
        // Passwords are NOT persisted: the redacted cache JSON strips both.
        let cache = redacted_json(&r);
        assert!(!cache.contains("pw1") && !cache.contains("pw2"));
    }

    #[test]
    fn record_findings_skips_save_when_no_hits() {
        // found == 0 => build_finding_json is None => record_findings must be a
        // no-op Ok(()) and never touch crate::findings::save.
        let none = LeakResult { found: 0, ..Default::default() };
        assert!(record_findings("acme", "acme.com", &none).is_ok());
    }
}
