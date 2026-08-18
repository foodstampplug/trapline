//! Shared, provider-agnostic breach-record types + helpers used by every
//! credential-breach integration (LeakCheck, Snusbase, DeHashed, LeakRadar).
//! Each provider client parses its own wire format into this normalized shape,
//! so the frontend renders one table for all of them.
//!
//! SECURITY: the plaintext `password` is retained here so the live card can
//! show it at the user's request, but it is NEVER written to disk —
//! `redacted_json` strips it before the watch.db cache, and `build_finding_json`
//! never puts it in a persisted finding.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub fn sha256_hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    h.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LeakSource {
    pub name: String,
    pub date: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LeakRow {
    pub email: String,
    pub username: String,
    /// Plaintext password. Shown in the card (user-authorized); NEVER persisted
    /// — `redacted_json`/`build_finding_json` strip it. Empty when absent.
    pub password: String,
    /// `!password.is_empty()` — severity + card lock marker without re-inspecting.
    pub password_present: bool,
    pub phone: String,
    pub name: String,
    pub hash: String,
    pub ip: String,
    /// Which breach/table/database the row came from.
    pub source: String,
    pub date: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LeakResult {
    pub found: u64,
    pub sources: Vec<LeakSource>,
    pub results: Vec<LeakRow>,
}

impl LeakResult {
    /// Push a source (deduped by name) if it isn't already present.
    pub fn add_source(&mut self, name: &str, date: &str) {
        if !name.is_empty() && !self.sources.iter().any(|s| s.name == name) {
            self.sources.push(LeakSource { name: name.to_string(), date: date.to_string() });
        }
    }
}

/// A copy of `r` with every plaintext password blanked (the `passwordPresent`
/// flag is preserved). Used wherever the result must not carry plaintext at rest.
pub fn strip_passwords(r: &LeakResult) -> LeakResult {
    LeakResult {
        found: r.found,
        sources: r.sources.clone(),
        results: r
            .results
            .iter()
            .map(|row| LeakRow { password: String::new(), ..row.clone() })
            .collect(),
    }
}

/// Serialize a `LeakResult` for AT-REST caching (watch.db enrichment) with
/// plaintext passwords stripped. The live card gets the full result over IPC —
/// passwords included — but nothing plaintext is ever written to disk.
pub fn redacted_json(r: &LeakResult) -> String {
    serde_json::to_string(&strip_passwords(r)).unwrap_or_default()
}

/// Map a normalized breach result to a HunterFinding JSON (camelCase). `None`
/// when `found == 0`. Severity `critical` if any row has a password, else
/// `high`. Evidence is a redacted summary (counts + source names only) — never
/// a plaintext credential. `platform` names the provider (e.g. "Snusbase").
pub fn build_finding_json(platform: &str, target: &str, value: &str, r: &LeakResult) -> Option<String> {
    if r.found == 0 {
        return None;
    }
    let severity = if r.results.iter().any(|row| row.password_present) { "critical" } else { "high" };
    let password_rows = r.results.iter().filter(|row| row.password_present).count();

    let mut evidence = format!("{platform} — credential exposure for {value}\nRecords found: {}\n", r.found);
    if !r.sources.is_empty() {
        let names: Vec<String> = r
            .sources
            .iter()
            .map(|s| if s.date.is_empty() { s.name.clone() } else { format!("{} ({})", s.name, s.date) })
            .collect();
        evidence.push_str(&format!("\nSources ({}):\n{}\n", r.sources.len(), names.join("\n")));
    }
    evidence.push_str(&format!("\n{password_rows} row(s) with password present.\n"));

    let sh = sha256_hex(&format!("{platform}:{value}"));
    let id = format!("breach-{}-{}", Utc::now().format("%Y%m%d-%H%M%S"), &sh[..6]);

    let finding = serde_json::json!({
        "id": id,
        "programName": target,
        "platform": platform,
        "title": format!("Credential exposure: {value} ({} records via {platform})", r.found),
        "severity": severity,
        "status": "draft",
        "endpoint": value,
        "summary": format!("{platform} found {} record(s) for {value}.", r.found),
        "description": "",
        "steps": "",
        "evidence": evidence,
        "impact": "",
        "remediation": "",
        "cvss": "",
        "cvssScore": "",
        "notes": "",
        "cmdline": platform.to_lowercase(),
    });
    Some(serde_json::to_string(&finding).unwrap())
}

/// Persist a breach lookup as an in-process finding, when there's something to
/// report (no-op `Ok(())` when `found == 0`).
pub fn record_findings(platform: &str, target: &str, value: &str, r: &LeakResult) -> Result<(), String> {
    match build_finding_json(platform, target, value, r) {
        Some(json) => crate::findings::save(&json),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_row(pw: &str) -> LeakRow {
        LeakRow {
            email: "a@b.test".into(),
            username: "neo".into(),
            password: pw.into(),
            password_present: !pw.is_empty(),
            source: "BreachX".into(),
            date: "2020-01".into(),
            ..Default::default()
        }
    }

    #[test]
    fn add_source_dedups_by_name() {
        let mut r = LeakResult::default();
        r.add_source("BreachX", "2020-01");
        r.add_source("BreachX", "2020-01");
        r.add_source("BreachY", "");
        assert_eq!(r.sources.len(), 2);
        r.add_source("", "2020"); // empty name ignored
        assert_eq!(r.sources.len(), 2);
    }

    #[test]
    fn redacted_json_strips_password_but_keeps_the_row() {
        let r = LeakResult { found: 1, sources: vec![], results: vec![sample_row("hunter2")] };
        let cache = redacted_json(&r);
        assert!(!cache.contains("hunter2"), "cache must strip the plaintext password");
        assert!(cache.contains("a@b.test") && cache.contains("neo"), "the rest of the row survives");
    }

    #[test]
    fn finding_platform_and_severity_and_no_password() {
        let none = LeakResult { found: 0, ..Default::default() };
        assert!(build_finding_json("Snusbase", "acme", "acme.com", &none).is_none());

        let r = LeakResult { found: 1, sources: vec![], results: vec![sample_row("hunter2")] };
        let j = build_finding_json("Snusbase", "acme", "acme.com", &r).unwrap();
        let v: serde_json::Value = serde_json::from_str(&j).unwrap();
        assert_eq!(v["platform"], "Snusbase");
        assert_eq!(v["severity"], "critical");
        assert!(!j.contains("hunter2"), "finding must never persist the plaintext password");

        let found_only = LeakResult { found: 3, results: vec![], ..Default::default() };
        let v2: serde_json::Value =
            serde_json::from_str(&build_finding_json("DeHashed", "acme", "acme.com", &found_only).unwrap()).unwrap();
        assert_eq!(v2["severity"], "high");
    }
}
