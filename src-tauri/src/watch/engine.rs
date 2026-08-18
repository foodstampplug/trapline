use anyhow::Result;
use chrono::Utc;
use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use url::Url;

use super::config::{Config, Target};
use super::fetch::{Fetched, Fetcher};
use super::parse::{self, Artifact, Kind};
use super::store::Store;
use super::{normalize, score, sourcemap};

fn sha256_hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    h.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

// ── Phase-3 auto-enrich: opt-in harvest helpers ──────────────────────────────
// Pure, read-only extraction over values the cycle already computed (new
// Endpoint/Route artifact values) or already downloaded (unit content). None
// of this changes what gets fetched, diffed, scored, or recorded above.

static EMAIL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").unwrap());

/// Extract the host from a URL-shaped artifact value. Values with no
/// parseable host (relative routes, garbage strings) → `None`.
pub fn host_of(v: &str) -> Option<String> {
    Url::parse(v).ok().and_then(|u| u.host_str().map(|h| h.to_string()))
}

/// Pull every email address out of a chunk of text, deduped (order-preserving,
/// first occurrence wins).
pub fn harvest_emails(content: &str) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for m in EMAIL_RE.find_iter(content) {
        let e = m.as_str().to_string();
        if seen.insert(e.clone()) {
            out.push(e);
        }
    }
    out
}

/// Push `item` onto `v` only if not already present — keeps a `TargetHarvest`
/// deduped across the multiple bundles/units folded into it per cycle.
fn push_dedup(v: &mut Vec<String>, item: String) {
    if !v.contains(&item) {
        v.push(item);
    }
}

/// One target's opt-in harvest for a cycle: hosts pulled from new
/// Endpoint/Route artifacts, and emails pulled from unit content. Only
/// populated when `run_target` is given `Some(&mut TargetHarvest)`.
#[derive(Debug, Default, Clone)]
pub struct TargetHarvest {
    pub target: String,
    pub hosts: Vec<String>,
    pub emails: Vec<String>,
}

/// Safety gate. With an empty scope list we allow everything (and warn at load),
/// otherwise a host must equal or be a sub-domain of a listed suffix.
fn host_in_scope(target_url: &str, scope: &[String]) -> bool {
    if scope.is_empty() {
        return true;
    }
    if let Ok(u) = Url::parse(target_url) {
        if let Some(h) = u.host_str() {
            return scope
                .iter()
                .any(|s| h == s.as_str() || h.ends_with(&format!(".{s}")));
        }
    }
    false
}

/// Fetch a page and pull every <script src> off it, resolved to absolute URLs.
fn discover_js(fetcher: &Fetcher, page_url: &str) -> Result<Vec<String>> {
    let html = match fetcher.get(page_url, None)? {
        Fetched::Body { text, .. } => text,
        Fetched::NotModified => return Ok(vec![]),
    };
    let doc = Html::parse_document(&html);
    let sel = Selector::parse("script[src]").unwrap();
    let base = Url::parse(page_url)?;
    let mut out = Vec::new();
    for el in doc.select(&sel) {
        if let Some(src) = el.value().attr("src") {
            if let Ok(abs) = base.join(src) {
                let s = abs.to_string();
                if s.split(['?', '#']).next().unwrap_or(&s).ends_with(".js") {
                    out.push(s);
                }
            }
        }
    }
    Ok(out)
}

pub fn run_once(cfg: &Config, store: &Store, fetcher: &Fetcher) -> Result<usize> {
    let mut total = 0;
    for target in &cfg.targets {
        match run_target(cfg, store, fetcher, target, None) {
            Ok(n) => total += n,
            Err(e) => eprintln!("[{}] target error: {e}", target.name),
        }
    }
    Ok(total)
}

/// Same cycle as `run_once`, plus an opt-in per-target harvest (hosts +
/// emails) for the scheduler's auto-enrich pass. `out` collects one
/// `TargetHarvest` per target that actually yielded hosts/emails this cycle.
pub fn run_once_harvest(
    cfg: &Config,
    store: &Store,
    fetcher: &Fetcher,
    out: &mut Vec<TargetHarvest>,
) -> Result<usize> {
    let mut total = 0;
    for target in &cfg.targets {
        let mut h = TargetHarvest { target: target.name.clone(), hosts: Vec::new(), emails: Vec::new() };
        match run_target(cfg, store, fetcher, target, Some(&mut h)) {
            Ok(n) => total += n,
            Err(e) => eprintln!("[{}] target error: {e}", target.name),
        }
        if !h.hosts.is_empty() || !h.emails.is_empty() {
            out.push(h);
        }
    }
    Ok(total)
}

fn run_target(
    cfg: &Config,
    store: &Store,
    fetcher: &Fetcher,
    target: &Target,
    mut harvest: Option<&mut TargetHarvest>,
) -> Result<usize> {
    // First run for this target? Then we only record a baseline and never alert,
    // so the initial flood of "every artifact is new" is silenced.
    let baseline = store.asset_count(&target.name)? == 0;

    // Build the JS work list: configured direct URLs + anything discovered on pages.
    let mut js_urls: Vec<String> = target.js.clone();
    for page in &target.pages {
        if !host_in_scope(page, &target.in_scope) {
            eprintln!("[{}] skip out-of-scope page {page}", target.name);
            continue;
        }
        match discover_js(fetcher, page) {
            Ok(found) => js_urls.extend(found),
            Err(e) => eprintln!("[{}] discover failed for {page}: {e}", target.name),
        }
    }
    let mut seen_urls = HashSet::new();
    js_urls.retain(|u| seen_urls.insert(u.clone()));

    let mut new_artifacts: Vec<Artifact> = Vec::new();
    let mut sources: Vec<String> = Vec::new();

    for url in &js_urls {
        if !host_in_scope(url, &target.in_scope) {
            continue;
        }
        let bundle_logical = normalize::logical_url(url);
        let prior_bundle = store.get_asset(&target.name, &bundle_logical)?;
        let etag = prior_bundle.as_ref().map(|(e, _)| e.clone());

        let fetched = match fetcher.get(url, etag.as_deref()) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("[{}] fetch {url}: {e}", target.name);
                continue;
            }
        };
        let (text, new_etag) = match fetched {
            Fetched::NotModified => continue,
            Fetched::Body { text, etag } => (text, etag),
        };

        let bundle_hash = sha256_hex(&text);
        if let Some((_, prev_hash)) = &prior_bundle {
            if *prev_hash == bundle_hash {
                continue; // identical bundle — nothing downstream changed
            }
        }

        // Expand the bundle into analysis UNITS: the original source files when a
        // source map is reachable, otherwise the minified bundle itself. Diffing
        // per source file means a churning bundle hash no longer creates noise —
        // only the source files that actually changed surface artifacts.
        let units = analysis_units(cfg, fetcher, target, url, &text);
        let via_map = units
            .first()
            .map(|(l, _)| l.starts_with("sm:"))
            .unwrap_or(false);

        let mut bundle_had_new = false;
        let mut changed: Vec<String> = Vec::new();
        for (unit_logical, content) in units {
            let prior_unit = store.get_asset(&target.name, &unit_logical)?;
            let uhash = sha256_hex(&content);
            if let Some((_, ph)) = &prior_unit {
                if *ph == uhash {
                    continue; // this source file is unchanged
                }
            }
            // Opt-in harvest: read the content this cycle already downloaded.
            // Does not affect what gets diffed/extracted/scored below.
            if let Some(h) = harvest.as_deref_mut() {
                for e in harvest_emails(&content) {
                    push_dedup(&mut h.emails, e);
                }
            }
            let consider_new = prior_unit.is_some() || !baseline;
            let mut unit_new = false;
            for a in parse::extract(&content) {
                // Only baseline/track artifacts that could actually alert — keeps
                // the "seen" DB free of sub-threshold noise.
                if a.kind != Kind::Secret && score::score(&a) < cfg.alert_threshold {
                    continue;
                }
                let is_new =
                    store.record_artifact(&target.name, a.kind.as_str(), &a.value, &unit_logical)?;
                if is_new && consider_new {
                    new_artifacts.push(a);
                    unit_new = true;
                }
            }
            store.upsert_asset(&target.name, &unit_logical, &uhash, "")?;
            if unit_new {
                bundle_had_new = true;
                changed.push(unit_logical.trim_start_matches("sm:").to_string());
            }
        }
        store.upsert_asset(&target.name, &bundle_logical, &bundle_hash, &new_etag)?;
        if bundle_had_new {
            sources.push(url.clone());
            if via_map && !changed.is_empty() {
                println!(
                    "  \u{21B3} [{}] changed source file(s) via map: {}",
                    target.name,
                    changed.join(", ")
                );
            }
        }
    }

    // Opt-in harvest: read hosts out of the new Endpoint/Route artifacts the
    // cycle already computed above. Runs regardless of whether these
    // artifacts clear the alert threshold below — the harvest is a separate
    // read, not part of the alert path.
    if let Some(h) = harvest.as_deref_mut() {
        for a in &new_artifacts {
            if matches!(a.kind, Kind::Endpoint | Kind::Route) {
                if let Some(host) = host_of(&a.value) {
                    push_dedup(&mut h.hosts, host);
                }
            }
        }
    }

    if new_artifacts.is_empty() {
        return Ok(0);
    }

    // Score, then keep secrets always + endpoints/routes/flags above threshold.
    let mut scored: Vec<(i64, Artifact)> =
        new_artifacts.into_iter().map(|a| (score::score(&a), a)).collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    let kept: Vec<(i64, Artifact)> = scored
        .into_iter()
        .filter(|(s, a)| a.kind == Kind::Secret || *s >= cfg.alert_threshold)
        .collect();
    if kept.is_empty() {
        return Ok(0);
    }

    let top = kept.first().map(|(s, _)| *s).unwrap_or(0);
    let sev = score::severity(top);
    let count = kept.len();
    let title = format!("New attack surface on {} ({count} new artifacts)", target.name);
    let source_url = sources.first().cloned().unwrap_or_else(|| target.name.clone());

    let embed = build_embed(&target.name, &kept, sev, top, &source_url);
    let content = format!("\u{1FAA4} **Trapline Watch** — {title}");
    if let Err(e) = crate::discord::send_watch_alert(&cfg.discord_webhook, &cfg.discord_username, &content, embed) {
        eprintln!("[{}] discord alert failed: {e}", target.name);
    }

    // One id shared across the alert + the in-process finding.
    let sh = sha256_hex(&source_url);
    let id = format!("watch-{}-{}", Utc::now().format("%Y%m%d-%H%M%S"), &sh[..6]);

    // Findings go DIRECTLY into the in-process store (no cross-process sync).
    if let Err(e) = super::sink::record_watch_finding(
        &id, &target.name, &title, &sev.to_lowercase(), top, &kept, &source_url,
    ) {
        eprintln!("[{}] finding write failed: {e}", target.name);
    }
    // SQLite audit row (Watch's own history table) — unchanged.
    store.insert_finding(&target.name, &title, sev, top)?;

    println!("[{}] {count} new artifact(s) — top {top} ({sev})", target.name);
    Ok(count)
}

/// Expand a fetched bundle into the units we actually diff + extract from:
/// original source files (via source map) when available, else the bundle text.
fn analysis_units(
    cfg: &Config,
    fetcher: &Fetcher,
    target: &Target,
    url: &str,
    text: &str,
) -> Vec<(String, String)> {
    if let Some(files) = source_files(cfg, fetcher, target, url, text) {
        if !files.is_empty() {
            return files
                .into_iter()
                .map(|f| (format!("sm:{}", f.path), f.content))
                .collect();
        }
    }
    vec![(normalize::logical_url(url), text.to_string())]
}

fn source_files(
    cfg: &Config,
    fetcher: &Fetcher,
    target: &Target,
    url: &str,
    text: &str,
) -> Option<Vec<sourcemap::SourceFile>> {
    // 1. honor an explicit sourceMappingURL comment (free — we already have the text)
    if let Some(sm_ref) = sourcemap::find_ref(text) {
        let map_text = if sm_ref.starts_with("data:") {
            sourcemap::decode_data_uri(&sm_ref)
        } else if let Some(map_url) = sourcemap::resolve_url(url, &sm_ref) {
            fetch_map(fetcher, target, &map_url)
        } else {
            None
        };
        if let Some(mt) = map_text {
            if let Ok(files) = sourcemap::parse(&mt) {
                if !files.is_empty() {
                    return Some(files);
                }
            }
        }
    }
    // 2. heuristic <bundle>.map probe (one extra request) when enabled
    if cfg.probe_source_maps {
        let guess = sourcemap::heuristic_map_url(url);
        if guess != url {
            if let Some(mt) = fetch_map(fetcher, target, &guess) {
                if let Ok(files) = sourcemap::parse(&mt) {
                    if !files.is_empty() {
                        return Some(files);
                    }
                }
            }
        }
    }
    None
}

fn fetch_map(fetcher: &Fetcher, target: &Target, map_url: &str) -> Option<String> {
    if !host_in_scope(map_url, &target.in_scope) {
        return None;
    }
    match fetcher.get(map_url, None) {
        Ok(Fetched::Body { text, .. }) => Some(text),
        _ => None,
    }
}

fn truncate_list(items: &[String], n: usize) -> String {
    if items.is_empty() {
        return "—".to_string();
    }
    let shown: Vec<String> = items.iter().take(n).cloned().collect();
    let mut s = shown.join("\n");
    if items.len() > n {
        s.push_str(&format!("\n…(+{} more)", items.len() - n));
    }
    if s.len() > 1000 {
        s.truncate(990);
        s.push_str("…");
    }
    s
}

fn build_embed(
    target: &str,
    kept: &[(i64, Artifact)],
    sev: &str,
    top: i64,
    source_url: &str,
) -> serde_json::Value {
    let mut secrets = Vec::new();
    let mut endpoints = Vec::new();
    let mut routes = Vec::new();
    let mut flags = Vec::new();
    for (s, a) in kept {
        let line = format!("`{}` ({s})", a.value);
        match a.kind {
            Kind::Secret => secrets.push(format!("`{}`", a.value)),
            Kind::Endpoint => endpoints.push(line),
            Kind::Route => routes.push(line),
            Kind::Flag => flags.push(line),
        }
    }

    let mut fields = Vec::new();
    if !secrets.is_empty() {
        fields.push(serde_json::json!({ "name": "\u{1F511} Secrets", "value": truncate_list(&secrets, 8), "inline": false }));
    }
    if !endpoints.is_empty() {
        fields.push(serde_json::json!({ "name": "\u{1F310} Endpoints", "value": truncate_list(&endpoints, 10), "inline": false }));
    }
    if !routes.is_empty() {
        fields.push(serde_json::json!({ "name": "\u{1F9ED} Routes", "value": truncate_list(&routes, 8), "inline": false }));
    }
    if !flags.is_empty() {
        fields.push(serde_json::json!({ "name": "\u{1F6A9} Flags", "value": truncate_list(&flags, 8), "inline": false }));
    }

    let mut embed = serde_json::json!({
        "title": format!("{target} — {sev}"),
        "color": score::color(top),
        "description": format!("**{}** new artifacts · top score **{top}**\nSource: {source_url}", kept.len()),
        "fields": fields,
    });
    if source_url.starts_with("http") {
        embed["url"] = serde_json::Value::String(source_url.to_string());
    }
    serde_json::json!([embed])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extract_host_from_url_artifact() {
        assert_eq!(host_of("https://api.acme.com/v2/x"), Some("api.acme.com".to_string()));
        assert_eq!(host_of("/relative/path"), None);      // routes without a host → None
        assert_eq!(host_of("not a url"), None);
    }
    #[test]
    fn harvest_emails_from_content() {
        let c = "contact support@acme.com or admin@acme.io; noise a@b (not an email)";
        let mut got = harvest_emails(c);
        got.sort();
        assert_eq!(got, vec!["admin@acme.io".to_string(), "support@acme.com".to_string()]);
    }
}
