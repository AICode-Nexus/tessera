use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tessera_config::{ProviderProfile, TesseraConfig};
use tessera_core::{ConversationEngine, ConversationOutcome, ConversationRequest};
use tessera_protocol::{ModelProfileId, ProviderId};
use tessera_providers::{
    mock::MockProvider, ollama::OllamaProvider, openai_compatible::OpenAiCompatibleProvider,
    ChatProvider,
};
use tessera_storage::TraceStore;
use tessera_tui::ChatViewState;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DoctorReport {
    pub status: String,
    pub data_dir: String,
    pub trace_writable: bool,
    pub sqlite_index_healthy: bool,
    pub provider_profiles: Vec<String>,
}

pub type Result<T> = anyhow::Result<T>;

static TRACE_COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn run_doctor(data_dir: impl AsRef<Path>) -> Result<DoctorReport> {
    run_doctor_with_config(data_dir, &TesseraConfig::default_with_mock())
}

pub fn run_doctor_with_config(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
) -> Result<DoctorReport> {
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
        provider_profiles: config
            .providers
            .iter()
            .map(|profile| profile.id.clone())
            .collect(),
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

pub async fn run_chat_with_config(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
    provider_id: &str,
    prompt: impl Into<String>,
) -> Result<ConversationOutcome> {
    let profile = config
        .providers
        .iter()
        .find(|profile| profile.id == provider_id)
        .ok_or_else(|| anyhow::anyhow!("provider profile not found: {provider_id}"))?;

    match profile.kind.as_str() {
        "mock" => run_chat_for_provider(data_dir, profile, MockProvider::default(), prompt).await,
        "openai-compatible" | "openai_compatible" => {
            let base_url = profile.base_url.as_deref().ok_or_else(|| {
                anyhow::anyhow!("provider profile `{}` requires base_url", profile.id)
            })?;
            let api_key = read_api_key(profile)?;
            let provider = OpenAiCompatibleProvider::new(
                base_url,
                api_key,
                ProviderId::from(profile.id.as_str()),
            );
            run_chat_for_provider(data_dir, profile, provider, prompt).await
        }
        "ollama" => {
            let base_url = profile
                .base_url
                .as_deref()
                .unwrap_or("http://localhost:11434");
            let provider = OllamaProvider::new(base_url, ProviderId::from(profile.id.as_str()));
            run_chat_for_provider(data_dir, profile, provider, prompt).await
        }
        other => Err(anyhow::anyhow!(
            "unsupported provider kind `{other}` for profile `{}`",
            profile.id
        )),
    }
}

pub async fn run_tui_with_config(
    data_dir: PathBuf,
    config: TesseraConfig,
    provider_id: String,
) -> Result<()> {
    let state = ChatViewState::new(provider_id.clone());
    tessera_tui::run_terminal_chat(state, move |prompt| {
        let data_dir = data_dir.clone();
        let config = config.clone();
        let provider_id = provider_id.clone();
        async move {
            let outcome = run_chat_with_config(data_dir, &config, &provider_id, prompt)
                .await
                .map_err(|error| error.to_string())?;
            outcome
                .store
                .read_trace_records(&outcome.trace_id)
                .map_err(|error| error.to_string())
        }
    })
    .await?;
    Ok(())
}

async fn run_chat_for_provider<P>(
    data_dir: impl AsRef<Path>,
    profile: &ProviderProfile,
    provider: P,
    prompt: impl Into<String>,
) -> Result<ConversationOutcome>
where
    P: ChatProvider,
{
    let store = TraceStore::open(data_dir)?;
    let engine = ConversationEngine::new(provider, store);
    let outcome = engine
        .run_chat(ConversationRequest {
            trace_id: next_trace_id(&profile.id),
            provider_id: ProviderId::from(profile.id.as_str()),
            profile_id: ModelProfileId::from(profile.id.as_str()),
            model: profile.default_model.clone(),
            prompt: prompt.into(),
        })
        .await?;
    Ok(outcome)
}

fn read_api_key(profile: &ProviderProfile) -> Result<Option<String>> {
    let Some(env_name) = &profile.api_key_env else {
        return Ok(None);
    };
    let value = std::env::var(env_name)
        .map_err(|_| anyhow::anyhow!("environment variable `{env_name}` is not set"))?;
    Ok(Some(value))
}

pub fn resolve_data_dir(explicit: Option<PathBuf>) -> Result<PathBuf> {
    resolve_data_dir_with_config(explicit, &TesseraConfig::default_with_mock())
}

pub fn resolve_data_dir_with_config(
    explicit: Option<PathBuf>,
    config: &TesseraConfig,
) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(path);
    }
    if let Some(data_dir) = &config.data_dir {
        return Ok(PathBuf::from(data_dir));
    }
    tessera_config::default_data_dir().ok_or_else(|| anyhow::anyhow!("cannot resolve data dir"))
}

pub fn resolve_config(explicit: Option<PathBuf>) -> Result<TesseraConfig> {
    match explicit {
        Some(path) => Ok(TesseraConfig::load_from_path(path)?),
        None => Ok(TesseraConfig::default_with_mock()),
    }
}

fn next_trace_id(provider_id: &str) -> String {
    let provider = provider_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let counter = TRACE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("trace_{provider}_{timestamp}_{counter}")
}
