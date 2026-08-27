use anyhow::Context;
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};

// token_hash/created_at mirror the full workers row for whoever queries next; no current
// handler reads them back off a returned Worker (auth only needs id/revoked_at).
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Worker {
    pub id: i64,
    pub discord_user_id: String,
    pub token_hash: String,
    pub created_at: String,
    pub revoked_at: Option<String>,
}

// id/submitted_by/channel_id/created_at mirror the full candidates row; no current handler
// reads them back off a returned Candidate (routes look candidates up by short_name, and
// attribution is written into the Discord caption at submit time, not read back out later).
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Candidate {
    pub id: i64,
    pub short_name: String,
    pub prompt: String,
    pub pipeline: String,
    pub model: String,
    pub variant_of: Option<String>,
    pub submitted_by: i64,
    pub message_id: String,
    pub channel_id: String,
    pub svg_path: String,
    pub png_path: String,
    pub created_at: String,
}

#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS workers (
    id INTEGER PRIMARY KEY,
    discord_user_id TEXT NOT NULL UNIQUE,
    token_hash TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    revoked_at TEXT
);

CREATE TABLE IF NOT EXISTS candidates (
    id INTEGER PRIMARY KEY,
    short_name TEXT NOT NULL UNIQUE,
    prompt TEXT NOT NULL,
    pipeline TEXT NOT NULL,
    model TEXT NOT NULL,
    variant_of TEXT,
    submitted_by INTEGER NOT NULL REFERENCES workers(id),
    message_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    svg_path TEXT NOT NULL,
    png_path TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
";

impl Db {
    pub fn open(path: &str) -> anyhow::Result<Self> {
        let conn = Connection::open(path).with_context(|| format!("failed to open database at {path}"))?;
        conn.execute_batch(SCHEMA)?;
        Ok(Db { conn: Arc::new(Mutex::new(conn)) })
    }

    pub fn insert_worker(&self, discord_user_id: &str, token_hash: &str) -> anyhow::Result<Worker> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO workers (discord_user_id, token_hash) VALUES (?1, ?2)
             ON CONFLICT(discord_user_id) DO UPDATE SET token_hash = excluded.token_hash, revoked_at = NULL",
            params![discord_user_id, token_hash],
        )?;
        Self::read_worker_by(&conn, "discord_user_id", discord_user_id)?.context("failed to read back inserted worker")
    }

    pub fn find_worker_by_token_hash(&self, token_hash: &str) -> anyhow::Result<Option<Worker>> {
        let conn = self.conn.lock().unwrap();
        Self::read_worker_by(&conn, "token_hash", token_hash)
    }

    fn read_worker_by(conn: &Connection, column: &str, value: &str) -> anyhow::Result<Option<Worker>> {
        let sql = format!("SELECT id, discord_user_id, token_hash, created_at, revoked_at FROM workers WHERE {column} = ?1");
        let result = conn.query_row(&sql, params![value], |row| {
            Ok(Worker {
                id: row.get(0)?,
                discord_user_id: row.get(1)?,
                token_hash: row.get(2)?,
                created_at: row.get(3)?,
                revoked_at: row.get(4)?,
            })
        });
        match result {
            Ok(worker) => Ok(Some(worker)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub fn revoke_worker(&self, discord_user_id: &str) -> anyhow::Result<bool> {
        let conn = self.conn.lock().unwrap();
        let updated = conn.execute(
            "UPDATE workers SET revoked_at = datetime('now') WHERE discord_user_id = ?1 AND revoked_at IS NULL",
            params![discord_user_id],
        )?;
        Ok(updated > 0)
    }

    pub fn next_short_name(&self) -> anyhow::Result<String> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM candidates", [], |row| row.get(0))?;
        Ok(format!("logo-{}", count + 1))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_candidate(
        &self,
        short_name: &str,
        prompt: &str,
        pipeline: &str,
        model: &str,
        variant_of: Option<&str>,
        submitted_by: i64,
        message_id: &str,
        channel_id: &str,
        svg_path: &str,
        png_path: &str,
    ) -> anyhow::Result<Candidate> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO candidates
             (short_name, prompt, pipeline, model, variant_of, submitted_by, message_id, channel_id, svg_path, png_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![short_name, prompt, pipeline, model, variant_of, submitted_by, message_id, channel_id, svg_path, png_path],
        )?;
        Self::read_candidate_by_short_name(&conn, short_name)?.context("failed to read back inserted candidate")
    }

    pub fn list_candidates(&self) -> anyhow::Result<Vec<Candidate>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, short_name, prompt, pipeline, model, variant_of, submitted_by, message_id, channel_id, svg_path, png_path, created_at
             FROM candidates ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([], Self::row_to_candidate)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn find_candidate_by_short_name(&self, short_name: &str) -> anyhow::Result<Option<Candidate>> {
        let conn = self.conn.lock().unwrap();
        Self::read_candidate_by_short_name(&conn, short_name)
    }

    fn read_candidate_by_short_name(conn: &Connection, short_name: &str) -> anyhow::Result<Option<Candidate>> {
        let result = conn.query_row(
            "SELECT id, short_name, prompt, pipeline, model, variant_of, submitted_by, message_id, channel_id, svg_path, png_path, created_at
             FROM candidates WHERE short_name = ?1",
            params![short_name],
            Self::row_to_candidate,
        );
        match result {
            Ok(candidate) => Ok(Some(candidate)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    fn row_to_candidate(row: &rusqlite::Row) -> rusqlite::Result<Candidate> {
        Ok(Candidate {
            id: row.get(0)?,
            short_name: row.get(1)?,
            prompt: row.get(2)?,
            pipeline: row.get(3)?,
            model: row.get(4)?,
            variant_of: row.get(5)?,
            submitted_by: row.get(6)?,
            message_id: row.get(7)?,
            channel_id: row.get(8)?,
            svg_path: row.get(9)?,
            png_path: row.get(10)?,
            created_at: row.get(11)?,
        })
    }
}
