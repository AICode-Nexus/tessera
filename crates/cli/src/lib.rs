use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use tessera_core::{ConversationEngine, ConversationOutcome, ConversationRequest};
use tessera_providers::mock::MockProvider;
use tessera_storage::TraceStore;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DoctorReport {
    pub status: String,
    pub data_dir: String,
    pub trace_writable: bool,
    pub sqlite_index_healthy: bool,
    pub provider_profiles: Vec<String>,
}

pub type Result<T> = anyhow::Result<T>;

pub fn run_doctor(data_dir: impl AsRef<Path>) -> Result<DoctorReport> {
    let data_dir = data_dir.as_ref();
    let store = TraceStore::open(data_dir)?;
    let traces_dir = data_dir.join("traces");
    fs::create_dir_all(&traces_dir)?;
    let probe = traces_dir.join(".write-probe");
    let trace_writable = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&probe)
        .and_then(|_| fs::remove_file(&probe))
        .is_ok();

    Ok(DoctorReport {
        status: if trace_writable && store.is_healthy() {
            "ok".to_string()
        } else {
            "error".to_string()
        },
        data_dir: data_dir.to_string_lossy().to_string(),
        trace_writable,
        sqlite_index_healthy: store.is_healthy(),
        provider_profiles: vec!["mock".to_string()],
    })
}

pub async fn run_chat_mock(
    data_dir: impl AsRef<Path>,
    prompt: impl Into<String>,
) -> Result<ConversationOutcome> {
    let store = TraceStore::open(data_dir)?;
    let engine = ConversationEngine::new(MockProvider::default(), store);
    let outcome = engine.run_chat(ConversationRequest::mock(prompt)).await?;
    Ok(outcome)
}

pub fn resolve_data_dir(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(path);
    }
    tessera_config::default_data_dir().ok_or_else(|| anyhow::anyhow!("cannot resolve data dir"))
}
