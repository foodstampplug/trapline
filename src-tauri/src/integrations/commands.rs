//! Tauri command glue for the Shodan + LeakCheck integrations: reads the
//! relevant API key out of `AppState.config` under a short lock, then hands
//! off to the pure/async client fns in `integrations::{shodan, leakcheck}`.

use crate::integrations::{leakcheck, shodan};
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
) -> Result<leakcheck::LeakResult, String> {
    let key = key_or_err(&state.config.lock().unwrap().leakcheck_api_key, "LeakCheck")?;
    let r = leakcheck::query(&key, &domain, "domain").await?;
    // On-demand lookup has no watch-target context, so `target` is the
    // queried value itself. Best-effort: the card still returns even if the
    // findings write fails.
    let _ = leakcheck::record_findings(&domain, &domain, &r);
    Ok(r)
}

#[tauri::command]
pub async fn leakcheck_email(
    email: String,
    state: State<'_, AppState>,
) -> Result<leakcheck::LeakResult, String> {
    let key = key_or_err(&state.config.lock().unwrap().leakcheck_api_key, "LeakCheck")?;
    let r = leakcheck::query(&key, &email, "email").await?;
    let _ = leakcheck::record_findings(&email, &email, &r);
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

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
