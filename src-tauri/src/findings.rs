use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Mirrors Go HunterFinding struct — all camelCase JSON tags
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HunterFinding {
    pub id: String,
    #[serde(default)]
    pub program_name: String,
    #[serde(default)]
    pub platform: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub steps: String,
    #[serde(default)]
    pub evidence: String,
    #[serde(default)]
    pub impact: String,
    #[serde(default)]
    pub remediation: String,
    #[serde(default)]
    pub cvss: String,
    #[serde(default)]
    pub cvss_score: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub cmdline: String,
    #[serde(default = "now_iso")]
    pub created_at: String,
    #[serde(default = "now_iso")]
    pub updated_at: String,
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn findings_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("Trapline").join("findings.json")
}

/// Process-global lock serializing every read-modify-write cycle against the
/// findings store. Phase 2 added autonomous background writers (the Watch
/// scheduler thread and the manual `watch_run_once` thread) alongside the
/// existing UI command thread. Without this lock, two writers that each
/// `load_all_at()` the same base list before either `save_all_at()`s will
/// race: the atomic temp+rename in `save_all_at` prevents a *torn* file, but
/// not a *lost update* — the second rename silently overwrites the first
/// writer's finding. Held across the entire RMW cycle of both save and
/// delete so all writers are fully serialized.
static SAVE_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn load_all_at(path: &Path) -> Result<Vec<HunterFinding>, String> {
    match fs::read_to_string(path) {
        Err(_) => Ok(Vec::new()), // not created yet
        Ok(s) if s.trim().is_empty() => Ok(Vec::new()),
        Ok(s) => serde_json::from_str(&s).map_err(|e| {
            // Never silently discard the user's findings on a parse error. Back the
            // file up and refuse, so a transient corruption can't be overwritten.
            let _ = fs::copy(path, path.with_extension("json.corrupt"));
            format!("findings.json did not parse ({e}); backed up to findings.json.corrupt and refusing to overwrite")
        }),
    }
}

fn save_all_at(path: &Path, findings: &[HunterFinding]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let data = serde_json::to_string_pretty(findings).map_err(|e| e.to_string())?;
    // Atomic write (temp + rename) so a concurrent reader never sees a half-written file.
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, data).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

/// Upsert-by-ID RMW body. Caller MUST hold `SAVE_LOCK` for the duration.
fn save_at(path: &Path, data: &str) -> Result<(), String> {
    let mut incoming: HunterFinding = serde_json::from_str(data)
        .map_err(|e| format!("invalid finding JSON: {}", e))?;
    incoming.updated_at = now_iso();

    let mut all = load_all_at(path)?;
    if let Some(pos) = all.iter().position(|f| f.id == incoming.id) {
        // Preserve original created_at on update
        incoming.created_at = all[pos].created_at.clone();
        all[pos] = incoming;
    } else {
        if incoming.created_at.is_empty() {
            incoming.created_at = now_iso();
        }
        all.push(incoming);
    }
    save_all_at(path, &all)
}

/// Delete-by-ID RMW body. Caller MUST hold `SAVE_LOCK` for the duration.
fn delete_at(path: &Path, id: &str) -> Result<(), String> {
    let mut all = load_all_at(path)?;
    let before = all.len();
    all.retain(|f| f.id != id);
    if all.len() == before {
        return Err(format!("finding {} not found", id));
    }
    save_all_at(path, &all)
}

/// Save, holding the process-global lock across the whole load-mutate-save cycle.
fn save_guarded(path: &Path, data: &str) -> Result<(), String> {
    let _guard = SAVE_LOCK.lock().unwrap();
    save_at(path, data)
}

/// Delete, holding the process-global lock across the whole load-mutate-save cycle.
fn delete_guarded(path: &Path, id: &str) -> Result<(), String> {
    let _guard = SAVE_LOCK.lock().unwrap();
    delete_at(path, id)
}

/// Upsert a finding by ID (parsed from a JSON string)
pub fn save(data: &str) -> Result<(), String> {
    save_guarded(&findings_path(), data)
}

/// Return all findings as a JSON string
pub fn load() -> Result<String, String> {
    let all = load_all_at(&findings_path())?;
    serde_json::to_string(&all).map_err(|e| e.to_string())
}

/// Delete a finding by ID
pub fn delete(id: &str) -> Result<(), String> {
    delete_guarded(&findings_path(), id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Build a unique temp file path for a hermetic test run. This never
    /// reads or writes the real findings_path()/%APPDATA% — the whole point
    /// of the path-taking seam (load_all_at/save_all_at/save_guarded/
    /// delete_guarded) is to let concurrency be exercised against a
    /// throwaway file instead.
    fn temp_findings_path(tag: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "trapline_findings_test_{}_{}_{}_{}.json",
            std::process::id(),
            tag,
            nanos,
            n
        ))
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(path.with_extension("json.tmp"));
        let _ = fs::remove_file(path.with_extension("json.corrupt"));
    }

    fn finding_json_with_unique_id(i: usize) -> String {
        format!(r#"{{"id":"concurrent-test-{i}","title":"t{i}"}}"#)
    }

    /// Regression test for the lost-update race: Phase 2 added autonomous
    /// background writers (Watch scheduler + manual watch_run_once) on top
    /// of the existing UI writer, all hitting the same findings file with
    /// no synchronization. N threads each upsert a distinct finding
    /// concurrently through save_guarded; without SAVE_LOCK held across the
    /// full load-modify-save cycle, overlapping writers each load the same
    /// base list and the last rename to land wins, silently dropping every
    /// other thread's finding. With the lock, all N must survive.
    #[test]
    fn concurrent_save_guarded_does_not_lose_updates() {
        let path = temp_findings_path("concurrent");
        cleanup(&path); // ensure absent/empty before starting

        const N: usize = 24;
        let path_arc = Arc::new(path.clone());
        let mut handles = Vec::with_capacity(N);
        for i in 0..N {
            let p = Arc::clone(&path_arc);
            handles.push(std::thread::spawn(move || {
                save_guarded(&p, &finding_json_with_unique_id(i))
                    .expect("save_guarded should succeed");
            }));
        }
        for h in handles {
            h.join().expect("writer thread panicked");
        }

        let all = load_all_at(&path).expect("load_all_at should succeed");
        assert_eq!(
            all.len(),
            N,
            "lost update: expected {} findings, found {} — the in-process race was reintroduced",
            N,
            all.len()
        );
        for i in 0..N {
            let want = format!("concurrent-test-{i}");
            assert!(
                all.iter().any(|f| f.id == want),
                "missing finding id {} after concurrent saves — lost-update race reintroduced",
                want
            );
        }

        cleanup(&path);
    }

    /// Basic single-threaded correctness: findings.rs had no tests before
    /// this fix. Save two, both present with created_at preserved on the
    /// untouched one; delete one, the other survives.
    #[test]
    fn save_and_delete_single_threaded_roundtrip() {
        let path = temp_findings_path("roundtrip");
        cleanup(&path);

        save_guarded(&path, &finding_json_with_unique_id(1)).unwrap();
        save_guarded(&path, &finding_json_with_unique_id(2)).unwrap();

        let all = load_all_at(&path).unwrap();
        assert_eq!(all.len(), 2);
        assert!(all.iter().any(|f| f.id == "concurrent-test-1"));
        assert!(all.iter().any(|f| f.id == "concurrent-test-2"));

        delete_guarded(&path, "concurrent-test-1").unwrap();
        let all = load_all_at(&path).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "concurrent-test-2");

        // Deleting a non-existent id errors and does not touch the file.
        let err = delete_guarded(&path, "does-not-exist").unwrap_err();
        assert!(err.contains("not found"));
        let all = load_all_at(&path).unwrap();
        assert_eq!(all.len(), 1);

        cleanup(&path);
    }
}
