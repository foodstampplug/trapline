use crate::AppState;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

use super::engine;
use super::fetch::Fetcher;
use super::store::Store;

/// The scheduler is a single background thread; this flag is its run gate.
static WATCH_RUNNING: AtomicBool = AtomicBool::new(false);

/// Bumped on every `start()`. Lets a superseded thread from a fast stop→start
/// race notice it's stale (its `my_gen` no longer matches) and exit, even if
/// `WATCH_RUNNING` got flipped back to `true` before it observed the `false`.
static WATCH_GEN: AtomicU64 = AtomicU64::new(0);

#[derive(Default, Clone)]
struct Runtime {
    last_run_ms: i64,
    last_assets: i64,
    last_new: i64,
}
static RUNTIME: Lazy<Mutex<Runtime>> = Lazy::new(|| Mutex::new(Runtime::default()));

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WatchStatus {
    pub running: bool,
    pub targets: usize,
    pub interval_secs: u64,
    pub last_run_ms: i64,
    pub last_assets: i64,
    pub last_new: i64,
}

/// `%APPDATA%\Trapline\watch.db` — colocated with config.json/findings.json.
pub fn watch_db_path() -> String {
    let mut p = crate::config::config_path();
    p.pop(); // drop config.json → the Trapline dir
    p.push("watch.db");
    p.to_string_lossy().to_string()
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Snapshot the current app config into an engine config for one cycle.
fn snapshot(app: &AppHandle) -> crate::config::Config {
    app.state::<AppState>().config.lock().unwrap().clone()
}

/// Build the current status from the flag, runtime, and live config.
pub fn status(app: &AppHandle) -> WatchStatus {
    let cfg = snapshot(app);
    let rt = RUNTIME.lock().unwrap().clone();
    WatchStatus {
        running: WATCH_RUNNING.load(Ordering::SeqCst),
        targets: cfg.watch_targets.len(),
        interval_secs: cfg.watch_interval_secs,
        last_run_ms: rt.last_run_ms,
        last_assets: rt.last_assets,
        last_new: rt.last_new,
    }
}

fn emit_status(app: &AppHandle) {
    let _ = app.emit("watch:status", status(app));
}

/// Run exactly one detection cycle across all targets.
fn run_cycle(app: &AppHandle) {
    let app_cfg = snapshot(app);
    let cfg = to_engine_config(&app_cfg, watch_db_path());
    if cfg.targets.is_empty() {
        emit_status(app);
        return;
    }
    let store = match Store::open(&cfg.database) {
        Ok(s) => s,
        Err(e) => { eprintln!("[watch] open db failed: {e}"); return; }
    };
    let fetcher = match Fetcher::new(&cfg.user_agent, cfg.max_requests_per_min) {
        Ok(f) => f,
        Err(e) => { eprintln!("[watch] fetcher init failed: {e}"); return; }
    };
    match engine::run_once(&cfg, &store, &fetcher) {
        Ok(n) => {
            let assets: i64 = cfg
                .targets
                .iter()
                .map(|t| store.asset_count(&t.name).unwrap_or(0))
                .sum();
            {
                let mut rt = RUNTIME.lock().unwrap();
                rt.last_run_ms = now_ms();
                rt.last_assets = assets;
                rt.last_new = n as i64;
            }
            emit_status(app);
            if n > 0 {
                let _ = app.emit("watch:new-finding", serde_json::json!({ "count": n, "ts": now_ms() }));
            }
        }
        Err(e) => eprintln!("[watch] cycle error: {e}"),
    }
}

/// Set watch_enabled in the persisted config (runtime on/off is durable).
fn set_enabled(app: &AppHandle, on: bool) {
    let st = app.state::<AppState>();
    let mut c = st.config.lock().unwrap();
    c.watch_enabled = on;
    crate::config::save(&c);
}

/// Start the scheduler thread if not already running (idempotent).
pub fn start(app: AppHandle) {
    if WATCH_RUNNING.swap(true, Ordering::SeqCst) {
        return; // already running
    }
    // This thread's generation. A subsequent start() (e.g. a fast stop→start
    // race) bumps WATCH_GEN again, so this thread notices it's been
    // superseded even if WATCH_RUNNING got flipped back to true first.
    let my_gen = WATCH_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    set_enabled(&app, true);
    std::thread::spawn(move || {
        while WATCH_RUNNING.load(Ordering::SeqCst) && WATCH_GEN.load(Ordering::SeqCst) == my_gen {
            run_cycle(&app);
            let interval = snapshot(&app).watch_interval_secs.max(1);
            let mut slept = 0u64;
            // Sleep in 1s steps so a stop is responsive.
            while slept < interval
                && WATCH_RUNNING.load(Ordering::SeqCst)
                && WATCH_GEN.load(Ordering::SeqCst) == my_gen
            {
                std::thread::sleep(Duration::from_secs(1));
                slept += 1;
            }
        }
        emit_status(&app);
    });
}

/// Stop the scheduler (idempotent) and persist the off state.
pub fn stop(app: &AppHandle) {
    WATCH_RUNNING.store(false, Ordering::SeqCst);
    set_enabled(app, false);
    emit_status(app);
}

/// Flag-only stop for the app exit handler (no config write / no emit needed).
pub fn stop_flag() {
    WATCH_RUNNING.store(false, Ordering::SeqCst);
}

/// Run a single cycle immediately, off the scheduler cadence (manual "Run once").
pub fn run_once_now(app: AppHandle) {
    std::thread::spawn(move || run_cycle(&app));
}

// ── Tauri commands ───────────────────────────────────────────────────────────
#[tauri::command]
pub fn watch_start(app: AppHandle) {
    start(app);
}

#[tauri::command]
pub fn watch_stop(app: AppHandle) {
    stop(&app);
}

#[tauri::command]
pub fn watch_status(app: AppHandle) -> WatchStatus {
    status(&app)
}

#[tauri::command]
pub fn watch_run_once(app: AppHandle) {
    run_once_now(app);
}

use crate::config::Config as AppConfig;
use super::config::{Config as EngineConfig, Target as EngineTarget, default_user_agent};

/// Map the app-level Config into the engine's in-memory Config for one cycle.
pub fn to_engine_config(app: &AppConfig, database: String) -> EngineConfig {
    EngineConfig {
        interval_secs: app.watch_interval_secs,
        alert_threshold: app.watch_alert_threshold,
        discord_webhook: app.webhook_url.clone(),
        discord_username: app.username.clone(),
        database,
        user_agent: default_user_agent(),
        max_requests_per_min: app.watch_max_rpm as u64,
        probe_source_maps: true,
        targets: app
            .watch_targets
            .iter()
            .map(|t| EngineTarget {
                name: t.name.clone(),
                pages: t.pages.clone(),
                js: t.js.clone(),
                in_scope: t.in_scope.clone(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config as AppConfig, WatchTarget};

    /// Models the fast stop→start race from the review finding using local
    /// atomics (not the module-global WATCH_RUNNING/WATCH_GEN, so this can't
    /// interfere with other tests running in parallel). Before the generation
    /// counter, a superseded thread's loop condition was just `running.load()`,
    /// which a following start() flips back to true — the old thread would
    /// never notice it had been replaced. The generation check closes that:
    /// a thread only keeps looping while ITS captured generation is still the
    /// current one.
    #[test]
    fn stale_generation_no_longer_matches_after_a_fast_stop_then_start() {
        let running = AtomicBool::new(false);
        let gen = AtomicU64::new(0);

        // start() #1 — thread A captures its generation.
        running.store(true, Ordering::SeqCst);
        let a_gen = gen.fetch_add(1, Ordering::SeqCst) + 1;
        assert!(running.load(Ordering::SeqCst) && gen.load(Ordering::SeqCst) == a_gen);

        // stop() — flips the flag off. (Old code: A's loop condition alone
        // would already catch this. Kept here to model the full sequence.)
        running.store(false, Ordering::SeqCst);

        // start() #2, before A polls again (the race window) — flips the flag
        // back on AND bumps the generation.
        running.store(true, Ordering::SeqCst);
        let b_gen = gen.fetch_add(1, Ordering::SeqCst) + 1;

        // A's next poll: the flag alone says "keep going" (this is exactly the
        // bug — flag-only would let A run forever alongside B), but A's
        // captured generation no longer matches, so A's real loop condition
        // (running && gen == my_gen) is false and it exits.
        assert!(running.load(Ordering::SeqCst), "flag flipped back true (the trap)");
        assert_ne!(a_gen, gen.load(Ordering::SeqCst), "A's generation must be stale");
        assert_eq!(b_gen, gen.load(Ordering::SeqCst), "only B's generation is current");
        assert!(!(running.load(Ordering::SeqCst) && gen.load(Ordering::SeqCst) == a_gen));
    }

    #[test]
    fn maps_app_config_to_engine_config() {
        let mut app = AppConfig::default();
        app.webhook_url = "https://discord/wh".into();
        app.username = "Trapline".into();
        app.watch_interval_secs = 900;
        app.watch_alert_threshold = 70;
        app.watch_max_rpm = 20;
        app.watch_targets = vec![WatchTarget {
            name: "acme".into(),
            pages: vec!["https://app.acme.com".into()],
            js: vec![],
            in_scope: vec!["acme.com".into()],
            auto_enrich: false,
        }];
        let ec = to_engine_config(&app, "C:/x/watch.db".into());
        assert_eq!(ec.interval_secs, 900);
        assert_eq!(ec.alert_threshold, 70);
        assert_eq!(ec.max_requests_per_min, 20);
        assert_eq!(ec.discord_webhook, "https://discord/wh");
        assert_eq!(ec.discord_username, "Trapline");
        assert_eq!(ec.database, "C:/x/watch.db");
        assert_eq!(ec.targets.len(), 1);
        assert_eq!(ec.targets[0].name, "acme");
        assert_eq!(ec.targets[0].in_scope, vec!["acme.com".to_string()]);
    }

    #[test]
    fn watch_status_serializes_camelcase() {
        let s = WatchStatus {
            running: true,
            targets: 2,
            interval_secs: 1800,
            last_run_ms: 123,
            last_assets: 18,
            last_new: 3,
        };
        let j = serde_json::to_string(&s).unwrap();
        assert!(j.contains("\"intervalSecs\""));
        assert!(j.contains("\"lastRunMs\""));
        assert!(j.contains("\"lastAssets\""));
        assert!(!j.contains("interval_secs"));
    }
}
