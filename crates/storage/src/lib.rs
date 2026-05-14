use rusqlite::{params, Connection};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use tessera_protocol::EventFrame;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, StorageError>;

pub struct TraceStore {
    data_dir: PathBuf,
    conn: Connection,
}

impl TraceStore {
    pub fn open(data_dir: impl AsRef<Path>) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        fs::create_dir_all(data_dir.join("traces"))?;
        fs::create_dir_all(data_dir.join("artifacts"))?;

        let conn = Connection::open(data_dir.join("tessera.sqlite3"))?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS event_index (
                trace_id TEXT NOT NULL,
                seq INTEGER NOT NULL,
                event_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                thread_id TEXT,
                turn_id TEXT,
                item_id TEXT,
                task_id TEXT,
                event_kind TEXT NOT NULL,
                jsonl_offset INTEGER NOT NULL,
                PRIMARY KEY (trace_id, seq)
            );

            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL
            );
            "#,
        )?;

        Ok(Self { data_dir, conn })
    }

    pub fn append(&mut self, frame: &EventFrame) -> Result<()> {
        let trace_path = self.trace_path(&frame.trace_id);
        let offset = trace_path
            .metadata()
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let record = frame.to_trace_record();
        let encoded = serde_json::to_string(&record)?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&trace_path)?;
        writeln!(file, "{encoded}")?;

        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO event_index (
                trace_id, seq, event_id, timestamp, thread_id, turn_id,
                item_id, task_id, event_kind, jsonl_offset
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                record.trace_id,
                record.seq,
                record.event_id.to_string(),
                record.timestamp.as_str(),
                record.thread_id.map(|id| id.to_string()),
                record.turn_id.map(|id| id.to_string()),
                record.item_id.map(|id| id.to_string()),
                record.task_id.map(|id| id.to_string()),
                record.event_kind,
                offset,
            ],
        )?;

        Ok(())
    }

    pub fn list_events(&self, trace_id: &str) -> Result<Vec<String>> {
        let mut statement = self
            .conn
            .prepare("SELECT event_kind FROM event_index WHERE trace_id = ?1 ORDER BY seq ASC")?;
        let events = statement
            .query_map([trace_id], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(events)
    }

    pub fn is_healthy(&self) -> bool {
        self.conn.query_row("SELECT 1", [], |_| Ok(())).is_ok()
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn trace_path(&self, trace_id: &str) -> PathBuf {
        self.data_dir
            .join("traces")
            .join(format!("{trace_id}.jsonl"))
    }
}
