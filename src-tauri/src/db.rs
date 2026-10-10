use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stem {
    pub name: String,
    pub path: String,
    pub bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: i64,
    pub source_path: String,
    pub source_name: String,
    pub model_id: String,
    pub output_dir: String,
    pub duration_secs: f64,
    pub created_at: i64,
    pub stems: Vec<Stem>,
}

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS jobs (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                source_path   TEXT NOT NULL,
                source_name   TEXT NOT NULL,
                model_id      TEXT NOT NULL,
                output_dir    TEXT NOT NULL,
                duration_secs REAL NOT NULL,
                created_at    INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS stems (
                id       INTEGER PRIMARY KEY AUTOINCREMENT,
                job_id   INTEGER NOT NULL,
                name     TEXT NOT NULL,
                path     TEXT NOT NULL,
                bytes    INTEGER NOT NULL,
                FOREIGN KEY(job_id) REFERENCES jobs(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_stems_job ON stems(job_id);
            DELETE FROM jobs WHERE NOT EXISTS (
                SELECT 1 FROM stems WHERE stems.job_id = jobs.id
            );
            "#,
        )?;
        Ok(Self { conn })
    }

    pub fn insert_completed_job(
        &mut self,
        source_path: &str,
        source_name: &str,
        model_id: &str,
        output_dir: &str,
        duration_secs: f64,
        stems: &[(String, String, i64)],
    ) -> Result<i64> {
        let transaction = self.conn.transaction()?;
        transaction.execute(
            "INSERT INTO jobs (source_path, source_name, model_id, output_dir, duration_secs, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                source_path,
                source_name,
                model_id,
                output_dir,
                duration_secs,
                now_millis()
            ],
        )?;
        let job_id = transaction.last_insert_rowid();
        for (name, path, bytes) in stems {
            transaction.execute(
                "INSERT INTO stems (job_id, name, path, bytes) VALUES (?1, ?2, ?3, ?4)",
                params![job_id, name, path, bytes],
            )?;
        }
        transaction.commit()?;
        Ok(job_id)
    }

    pub fn list_jobs(&self, limit: i64) -> Result<Vec<Job>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_path, source_name, model_id, output_dir, duration_secs, created_at
             FROM jobs ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, f64>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })?;

        let mut jobs = Vec::new();
        for row in rows {
            let (id, source_path, source_name, model_id, output_dir, duration_secs, created_at) =
                row?;
            jobs.push(Job {
                id,
                source_path,
                source_name,
                model_id,
                output_dir,
                duration_secs,
                created_at,
                stems: self.get_stems(id)?,
            });
        }
        Ok(jobs)
    }

    pub fn get_stems(&self, job_id: i64) -> Result<Vec<Stem>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, path, bytes FROM stems WHERE job_id = ?1 ORDER BY id")?;
        let rows = stmt.query_map(params![job_id], |row| {
            Ok(Stem {
                name: row.get(0)?,
                path: row.get(1)?,
                bytes: row.get(2)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Error::from)
    }

    pub fn is_known_stem_path(&self, path: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM stems WHERE path = ?1",
            params![path],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn delete_job(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM jobs WHERE id = ?1", params![id])?;
        Ok(())
    }
}

fn now_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
