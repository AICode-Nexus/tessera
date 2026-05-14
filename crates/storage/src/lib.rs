use rusqlite::{params, Connection};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use tessera_protocol::{ArtifactId, EventFrame, ItemId, TaskId, ThreadId, TraceRecord, TurnId};

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexedRunObjects {
    pub threads: Vec<ThreadId>,
    pub turns: Vec<TurnId>,
    pub items: Vec<ItemId>,
    pub tasks: Vec<TaskId>,
    pub artifacts: Vec<ArtifactId>,
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

            CREATE TABLE IF NOT EXISTS artifact_index (
                trace_id TEXT NOT NULL,
                artifact_id TEXT NOT NULL,
                seq INTEGER NOT NULL,
                PRIMARY KEY (trace_id, artifact_id)
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

        self.index_record(&record, offset)?;

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

    pub fn read_trace_records(&self, trace_id: &str) -> Result<Vec<TraceRecord>> {
        let file = std::fs::File::open(self.trace_path(trace_id))?;
        let mut records = Vec::new();
        for line in BufReader::new(file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            records.push(serde_json::from_str(&line)?);
        }
        records.sort_by_key(|record: &TraceRecord| record.seq);
        Ok(records)
    }

    pub fn list_indexed_objects(&self, trace_id: &str) -> Result<IndexedRunObjects> {
        Ok(IndexedRunObjects {
            threads: self.query_distinct_ids(
                "SELECT thread_id FROM event_index WHERE trace_id = ?1 AND thread_id IS NOT NULL GROUP BY thread_id ORDER BY MIN(seq)",
                trace_id,
            )?,
            turns: self.query_distinct_ids(
                "SELECT turn_id FROM event_index WHERE trace_id = ?1 AND turn_id IS NOT NULL GROUP BY turn_id ORDER BY MIN(seq)",
                trace_id,
            )?,
            items: self.query_distinct_ids(
                "SELECT item_id FROM event_index WHERE trace_id = ?1 AND item_id IS NOT NULL GROUP BY item_id ORDER BY MIN(seq)",
                trace_id,
            )?,
            tasks: self.query_distinct_ids(
                "SELECT task_id FROM event_index WHERE trace_id = ?1 AND task_id IS NOT NULL GROUP BY task_id ORDER BY MIN(seq)",
                trace_id,
            )?,
            artifacts: self.query_distinct_ids(
                "SELECT artifact_id FROM artifact_index WHERE trace_id = ?1 GROUP BY artifact_id ORDER BY MIN(seq)",
                trace_id,
            )?,
        })
    }

    pub fn rebuild_index(&mut self, trace_id: &str) -> Result<()> {
        let records = read_trace_records_with_offsets(self.trace_path(trace_id))?;

        self.conn
            .execute("DELETE FROM event_index WHERE trace_id = ?1", [trace_id])?;
        self.conn
            .execute("DELETE FROM artifact_index WHERE trace_id = ?1", [trace_id])?;

        for (record, offset) in records {
            self.index_record(&record, offset)?;
        }

        Ok(())
    }

    fn index_record(&self, record: &TraceRecord, offset: u64) -> Result<()> {
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
                record.thread_id.as_ref().map(|id| id.to_string()),
                record.turn_id.as_ref().map(|id| id.to_string()),
                record.item_id.as_ref().map(|id| id.to_string()),
                record.task_id.as_ref().map(|id| id.to_string()),
                record.event_kind,
                offset,
            ],
        )?;

        let artifact_ids = record
            .artifact_refs
            .iter()
            .map(ToString::to_string)
            .chain(artifact_id_from_payload(record));

        for artifact_id in artifact_ids {
            self.conn.execute(
                r#"
                INSERT OR IGNORE INTO artifact_index (trace_id, artifact_id, seq)
                VALUES (?1, ?2, ?3)
                "#,
                params![record.trace_id, artifact_id, record.seq],
            )?;
        }

        Ok(())
    }

    fn query_distinct_ids<T>(&self, sql: &str, trace_id: &str) -> Result<Vec<T>>
    where
        T: From<String>,
    {
        let mut statement = self.conn.prepare(sql)?;
        let values = statement
            .query_map([trace_id], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?
            .into_iter()
            .map(T::from)
            .collect();
        Ok(values)
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

fn artifact_id_from_payload(record: &TraceRecord) -> Option<String> {
    if record.event_kind != "artifact_created" {
        return None;
    }
    record
        .payload
        .get("artifact_id")
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
}

fn read_trace_records_with_offsets(trace_path: PathBuf) -> Result<Vec<(TraceRecord, u64)>> {
    let file = std::fs::File::open(trace_path)?;
    let mut records = Vec::new();
    let mut offset = 0_u64;

    for line in BufReader::new(file).split(b'\n') {
        let line = line?;
        if line.is_empty() {
            offset += 1;
            continue;
        }
        let record = serde_json::from_slice(&line)?;
        records.push((record, offset));
        offset += line.len() as u64 + 1;
    }

    Ok(records)
}
