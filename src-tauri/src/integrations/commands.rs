//! Tauri command glue for the Shodan + LeakCheck integrations: reads the
//! relevant API key out of `AppState.config` under a short lock, then hands
//! off to the pure/async client fns in `integrations::{shodan, leakcheck}`.

use crate::integrations::{breach, dehashed, leakcheck, leakradar, shodan, snusbase};
use crate::AppState;
use tauri::State;

/// Trim-and-validate a key, or an actionable error naming the provider.
/// The one pure, unit-tested piece in this file.
pub fn key_or_err(key: &str, provider: &str) -> Result<String, String> {
    let k = key.trim();
    if k.is_empty() {
        Err(format!("{provider} API key not set — add it in Settings"))
    } else {
        Ok(k.to_string())
    }
}

#[tauri::command]
pub async fn shodan_host(
    ip: String,
    state: State<'_, AppState>,
) -> Result<shodan::ShodanHost, String> {
    // Lock scope ends (guard dropped) before the `.await` below — a
    // std::sync::MutexGuard is not Send and must not cross an await point.
    let key = key_or_err(&state.config.lock().unwrap().shodan_api_key, "Shodan")?;
    shodan::host(&key, &ip).await
}

#[tauri::command]
pub async fn shodan_domain(
    domain: String,
    state: State<'_, AppState>,
) -> Result<shodan::ShodanDomain, String> {
    let key = key_or_err(&state.config.lock().unwrap().shodan_api_key, "Shodan")?;
    shodan::domain(&key, &domain).await
}

#[tauri::command]
pub async fn shodan_search(
    query: String,
    state: State<'_, AppState>,
) -> Result<shodan::ShodanSearch, String> {
    let key = key_or_err(&state.config.lock().unwrap().shodan_api_key, "Shodan")?;
    shodan::search(&key, &query).await
}

#[tauri::command]
pub async fn leakcheck_domain(
    domain: String,
    state: State<'_, AppState>,
) -> Result<breach::LeakResult, String> {
    let key = key_or_err(&state.config.lock().unwrap().leakcheck_api_key, "LeakCheck")?;
    let r = leakcheck::query(&key, &domain, "domain").await?;
    // On-demand lookup has no watch-target context, so `target` is the
    // queried value itself. Best-effort: the card still returns even if the
    // findings write fails.
    let _ = breach::record_findings("LeakCheck", &domain, &domain, &r);
    Ok(r)
}

#[tauri::command]
pub async fn leakcheck_email(
    email: String,
    state: State<'_, AppState>,
) -> Result<breach::LeakResult, String> {
    let key = key_or_err(&state.config.lock().unwrap().leakcheck_api_key, "LeakCheck")?;
    let r = leakcheck::query(&key, &email, "email").await?;
    let _ = breach::record_findings("LeakCheck", &email, &email, &r);
    Ok(r)
}

/// Every LeakCheck v2 query type (docs.leakcheck.io/pro-api/search-types).
/// `phash`/`origin`/`password` are Enterprise-only but valid to send.
pub const LEAKCHECK_KINDS: &[&str] = &[
    "auto", "email", "domain", "username", "phone", "keyword", "hash", "phash", "origin", "password",
];

pub fn leakcheck_valid_kind(kind: &str) -> bool {
    LEAKCHECK_KINDS.contains(&kind)
}

/// Generic LeakCheck lookup for any supported `kind` — backs all the ⌘K
/// LeakCheck commands. `value` is the query, `kind` the search type.
#[tauri::command]
pub async fn leakcheck_query(
    value: String,
    kind: String,
    state: State<'_, AppState>,
) -> Result<breach::LeakResult, String> {
    if !leakcheck_valid_kind(&kind) {
        return Err(format!("Unsupported LeakCheck type '{kind}'"));
    }
    let key = key_or_err(&state.config.lock().unwrap().leakcheck_api_key, "LeakCheck")?;
    let r = leakcheck::query(&key, &value, &kind).await?;
    // On-demand lookup has no watch-target context, so `target` is the queried
    // value itself. Best-effort: the card still returns if the findings write fails.
    let _ = breach::record_findings("LeakCheck", &value, &value, &r);
    Ok(r)
}

/// Resolve a breach provider's key + display label from config.
fn breach_key(cfg: &crate::config::Config, provider: &str) -> Result<(String, &'static str), String> {
    let (raw, label) = match provider {
        "leakcheck" => (&cfg.leakcheck_api_key, "LeakCheck"),
        "snusbase" => (&cfg.snusbase_api_key, "Snusbase"),
        "dehashed" => (&cfg.dehashed_api_key, "DeHashed"),
        "leakradar" => (&cfg.leakradar_api_key, "LeakRadar"),
        other => return Err(format!("Unknown breach provider '{other}'")),
    };
    Ok((key_or_err(raw, label)?, label))
}

/// Generic breach lookup across every provider (LeakCheck / Snusbase / DeHashed
/// / LeakRadar) — backs all the ⌘K breach commands. Reads the provider's key,
/// dispatches to its client, and records a finding (best-effort). Passwords in
/// the returned result are live/in-memory only — the finding + watch.db cache
/// strip them (see breach.rs).
#[tauri::command]
pub async fn breach_query(
    provider: String,
    value: String,
    kind: String,
    state: State<'_, AppState>,
) -> Result<breach::LeakResult, String> {
    // Short lock: resolve key + label, guard dropped before any `.await`.
    let (key, label) = { breach_key(&state.config.lock().unwrap(), &provider)? };
    let r = match provider.as_str() {
        "leakcheck" => leakcheck::query(&key, &value, &kind).await?,
        "snusbase" => snusbase::query(&key, &value, &kind).await?,
        "dehashed" => dehashed::query(&key, &value, &kind).await?,
        "leakradar" => leakradar::query(&key, &value, &kind).await?,
        other => return Err(format!("Unknown breach provider '{other}'")),
    };
    let _ = breach::record_findings(label, &value, &value, &r);
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breach_key_resolves_labels_and_rejects_unknown() {
        let mut cfg = crate::config::Config::default();
        cfg.snusbase_api_key = "SK".into();
        cfg.dehashed_api_key = "DK".into();
        cfg.leakradar_api_key = "LR".into();
        assert_eq!(breach_key(&cfg, "snusbase").unwrap(), ("SK".to_string(), "Snusbase"));
        assert_eq!(breach_key(&cfg, "dehashed").unwrap(), ("DK".to_string(), "DeHashed"));
        assert_eq!(breach_key(&cfg, "leakradar").unwrap(), ("LR".to_string(), "LeakRadar"));
        assert!(breach_key(&cfg, "bogus").is_err());
        // missing key → actionable error
        assert!(breach_key(&crate::config::Config::default(), "snusbase")
            .unwrap_err()
            .contains("Snusbase API key not set"));
    }

    #[test]
    fn leakcheck_kind_allowlist() {
        for k in ["auto", "email", "domain", "username", "phone", "keyword", "hash", "phash", "origin", "password"] {
            assert!(leakcheck_valid_kind(k), "{k} should be valid");
        }
        assert!(!leakcheck_valid_kind("bogus"));
        assert!(!leakcheck_valid_kind(""));
    }

    #[test]
    fn missing_key_message_is_actionable() {
        assert_eq!(
            key_or_err("", "Shodan"),
            Err("Shodan API key not set — add it in Settings".to_string())
        );
        assert_eq!(
            key_or_err("  ", "LeakCheck"),
            Err("LeakCheck API key not set — add it in Settings".to_string())
        );
        assert_eq!(key_or_err("K", "Shodan"), Ok("K".to_string()));
    }
}
