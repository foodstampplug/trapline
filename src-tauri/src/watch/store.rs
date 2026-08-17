use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS assets (
  target     TEXT NOT NULL,
  logical    TEXT NOT NULL,
  hash       TEXT NOT NULL,
  etag       TEXT NOT NULL DEFAULT '',
  last_seen  TEXT NOT NULL,
  PRIMARY KEY (target, logical)
);
CREATE TABLE IF NOT EXISTS artifacts (
  target     TEXT NOT NULL,
  kind       TEXT NOT NULL,
  value      TEXT NOT NULL,
  asset      TEXT NOT NULL DEFAULT '',
  first_seen TEXT NOT NULL,
  PRIMARY KEY (target, kind, value)
);
CREATE TABLE IF NOT EXISTS findings (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  target     TEXT NOT NULL,
  title      TEXT NOT NULL,
  severity   TEXT NOT NULL,
  score      INTEGER NOT NULL,
  created_at TEXT NOT NULL
);
"#;

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    /// Returns (etag, hash) of the last fetch for this logical asset, if any.
    pub fn get_asset(&self, target: &str, logical: &str) -> Result<Option<(String, String)>> {
        let row = self
            .conn
            .query_row(
                "SELECT etag, hash FROM assets WHERE target = ?1 AND logical = ?2",
                params![target, logical],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?;
        Ok(row)
    }

    pub fn upsert_asset(&self, target: &str, logical: &str, hash: &str, etag: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO assets (target, logical, hash, etag, last_seen)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(target, logical) DO UPDATE SET
               hash = excluded.hash,
               etag = excluded.etag,
               last_seen = excluded.last_seen",
            params![target, logical, hash, etag, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    /// Records an artifact. Returns true if it was never seen before for this target.
    pub fn record_artifact(&self, target: &str, kind: &str, value: &str, asset: &str) -> Result<bool> {
        let n = self.conn.execute(
            "INSERT OR IGNORE INTO artifacts (target, kind, value, asset, first_seen)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![target, kind, value, asset, Utc::now().to_rfc3339()],
        )?;
        Ok(n == 1)
    }

    pub fn asset_count(&self, target: &str) -> Result<i64> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM assets WHERE target = ?1",
            params![target],
            |r| r.get(0),
        )?;
        Ok(n)
    }

    pub fn artifact_count(&self, target: &str) -> Result<i64> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM artifacts WHERE target = ?1",
            params![target],
            |r| r.get(0),
        )?;
        Ok(n)
    }

    pub fn insert_finding(&self, target: &str, title: &str, severity: &str, score: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO findings (target, title, severity, score, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![target, title, severity, score, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }
}
