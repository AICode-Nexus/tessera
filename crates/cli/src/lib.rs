use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tessera_client::ClientSnapshot;
use tessera_config::{ProviderProfile, TesseraConfig};
use tessera_core::{ConversationEngine, ConversationOutcome, ConversationRequest, EventSinkAction};
use tessera_protocol::{EventFrame, ModelProfileId, ProviderId, RunEvent};
use tessera_providers::{
    mock::MockProvider, ollama::OllamaProvider, openai_compatible::OpenAiCompatibleProvider,
    ChatProvider,
};
use tessera_storage::TraceStore;
use tessera_tui::{ChatViewState, LiveClientEvent};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DoctorReport {
    pub status: String,
    pub data_dir: String,
    pub trace_writable: bool,
    pub sqlite_index_healthy: bool,
    pub provider_profiles: Vec<String>,
}

pub type Result<T> = anyhow::Result<T>;

pub const VERSION_TEXT: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (git ",
    env!("TESSERA_GIT_SHA"),
    ")"
);

static TRACE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CliReplCommand {
    Help,
    NewThread,
    Profiles,
    SwitchProfile(String),
    Status,
    Export,
    Quit,
    Unknown(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliReplCommandOutcome {
    pub should_quit: bool,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CliReplSession {
    snapshot: ClientSnapshot,
}

impl CliReplSession {
    pub fn new(config: &TesseraConfig, provider_id: &str) -> Result<Self> {
        ensure_provider_profile(config, provider_id)?;
        let profile_ids = config
            .providers
            .iter()
            .map(|profile| profile.id.clone())
            .collect::<Vec<_>>();
        Ok(Self {
            snapshot: ClientSnapshot::with_profiles(provider_id, profile_ids),
        })
    }

    pub fn snapshot(&self) -> &ClientSnapshot {
        &self.snapshot
    }

    pub fn snapshot_mut(&mut self) -> &mut ClientSnapshot {
        &mut self.snapshot
    }

    pub fn handle_command(
        &mut self,
        config: &TesseraConfig,
        command: CliReplCommand,
    ) -> Result<CliReplCommandOutcome> {
        match command {
            CliReplCommand::Help => Ok(CliReplCommandOutcome::continue_with(help_lines())),
            CliReplCommand::NewThread => {
                self.snapshot.start_new_thread();
                Ok(CliReplCommandOutcome::continue_with(["new thread started"]))
            }
            CliReplCommand::Profiles => {
                let lines: Vec<String> = config
                    .providers
                    .iter()
                    .map(|profile| {
                        let marker = if profile.id == self.snapshot.status.active_profile {
                            "*"
                        } else {
                            " "
                        };
                        format!("{marker} {} ({})", profile.id, profile.kind)
                    })
                    .collect();
                Ok(CliReplCommandOutcome::continue_with(lines))
            }
            CliReplCommand::SwitchProfile(profile_id) => {
                ensure_provider_profile(config, &profile_id)?;
                self.snapshot.status.active_profile = profile_id.clone();
                Ok(CliReplCommandOutcome::continue_with([format!(
                    "profile switched to {profile_id}"
                )]))
            }
            CliReplCommand::Status => {
                Ok(CliReplCommandOutcome::continue_with([self.status_line()]))
            }
            CliReplCommand::Export => Ok(CliReplCommandOutcome::continue_with(
                self.snapshot.export_markdown().lines().map(str::to_string),
            )),
            CliReplCommand::Quit => Ok(CliReplCommandOutcome {
                should_quit: true,
                lines: vec!["bye".to_string()],
            }),
            CliReplCommand::Unknown(command) => {
                Ok(CliReplCommandOutcome::continue_with([format!(
                    "unknown command `{command}`; type /help for commands"
                )]))
            }
        }
    }

    fn status_line(&self) -> String {
        format!(
            "profile {} | {} | {} | {} | {} | {}",
            self.snapshot.status.active_profile,
            self.snapshot.status.task_summary,
            self.snapshot.status.usage_summary,
            self.snapshot.status.cache_summary,
            self.snapshot.status.cost_summary,
            self.snapshot.status.context_summary
        )
    }
}

impl CliReplCommandOutcome {
    fn continue_with<I, S>(lines: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            should_quit: false,
            lines: lines.into_iter().map(Into::into).collect(),
        }
    }
}

pub fn parse_repl_command(input: &str) -> Option<CliReplCommand> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    let command = trimmed.split_whitespace().next().unwrap_or(trimmed);
    let argument = trimmed[command.len()..].trim();
    Some(match command {
        "/help" | "/?" => CliReplCommand::Help,
        "/new" => CliReplCommand::NewThread,
        "/profiles" => CliReplCommand::Profiles,
        "/profile" if !argument.is_empty() => CliReplCommand::SwitchProfile(argument.to_string()),
        "/status" => CliReplCommand::Status,
        "/export" => CliReplCommand::Export,
        "/quit" | "/exit" => CliReplCommand::Quit,
        _ => CliReplCommand::Unknown(trimmed.to_string()),
    })
}

pub async fn run_repl_prompt_with_writer<F>(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
    session: &mut CliReplSession,
    prompt: impl Into<String>,
    mut write_delta: F,
) -> Result<ConversationOutcome>
where
    F: FnMut(&str),
{
    let provider_id = session.snapshot.status.active_profile.clone();
    let snapshot = &mut session.snapshot;
    run_chat_with_config_and_events(data_dir, config, &provider_id, prompt, |frame| {
        snapshot.apply_event(frame);
        if let RunEvent::AssistantDelta { text, .. } = &frame.event {
            write_delta(text);
        }
        EventSinkAction::Continue
    })
    .await
}

pub async fn run_chat_repl_with_config(
    data_dir: PathBuf,
    config: TesseraConfig,
    provider_id: String,
) -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_chat_repl_with_io(data_dir, config, provider_id, stdin.lock(), stdout.lock()).await?;
    Ok(())
}

pub async fn run_chat_repl_with_io<R, W>(
    data_dir: PathBuf,
    config: TesseraConfig,
    provider_id: String,
    mut input: R,
    mut output: W,
) -> Result<ClientSnapshot>
where
    R: BufRead,
    W: Write,
{
    let mut session = CliReplSession::new(&config, &provider_id)?;
    writeln!(output, "Tessera CLI interactive chat")?;
    writeln!(output, "type /help for commands, /quit to exit")?;

    let mut line = String::new();
    loop {
        write!(
            output,
            "\ntessera({})> ",
            session.snapshot.status.active_profile
        )?;
        output.flush()?;

        line.clear();
        if input.read_line(&mut line)? == 0 {
            break;
        }
        let user_input = line.trim();
        if user_input.is_empty() {
            continue;
        }

        if let Some(command) = parse_repl_command(user_input) {
            match session.handle_command(&config, command) {
                Ok(outcome) => {
                    for line in outcome.lines {
                        writeln!(output, "{line}")?;
                    }
                    if outcome.should_quit {
                        break;
                    }
                }
                Err(error) => {
                    writeln!(output, "error: {error}")?;
                }
            }
            continue;
        }

        write!(output, "assistant> ")?;
        output.flush()?;
        let mut write_error = None;
        let result = run_repl_prompt_with_writer(
            &data_dir,
            &config,
            &mut session,
            user_input.to_string(),
            |delta| {
                if write_error.is_some() {
                    return;
                }
                if let Err(error) = write!(output, "{delta}") {
                    write_error = Some(error);
                    return;
                }
                if let Err(error) = output.flush() {
                    write_error = Some(error);
                }
            },
        )
        .await;
        if let Some(error) = write_error {
            return Err(error.into());
        }
        result?;
        writeln!(output)?;
    }

    Ok(session.snapshot)
}

fn help_lines() -> Vec<&'static str> {
    vec![
        "commands:",
        "  /help              show this help",
        "  /new               start a fresh visible thread",
        "  /profiles          list configured provider profiles",
        "  /profile <id>      switch active provider profile",
        "  /status            show compact runtime status",
        "  /export            print markdown transcript",
        "  /quit, /exit       leave the REPL",
    ]
}

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

fn ensure_provider_profile(config: &TesseraConfig, provider_id: &str) -> Result<()> {
    if config
        .providers
        .iter()
        .any(|profile| profile.id == provider_id)
    {
        return Ok(());
    }

    Err(anyhow::anyhow!("provider profile not found: {provider_id}"))
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
    run_chat_with_config_and_events(data_dir, config, provider_id, prompt, |_| {}).await
}

pub async fn run_chat_with_config_and_events<F, R>(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
    provider_id: &str,
    prompt: impl Into<String>,
    mut event_sink: F,
) -> Result<ConversationOutcome>
where
    F: FnMut(&EventFrame) -> R,
    R: Into<EventSinkAction>,
{
    let profile = config
        .providers
        .iter()
        .find(|profile| profile.id == provider_id)
        .ok_or_else(|| anyhow::anyhow!("provider profile not found: {provider_id}"))?;

    match profile.kind.as_str() {
        "mock" => {
            run_chat_for_provider_with_events(
                data_dir,
                profile,
                MockProvider::default(),
                prompt,
                &mut event_sink,
            )
            .await
        }
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
            run_chat_for_provider_with_events(data_dir, profile, provider, prompt, &mut event_sink)
                .await
        }
        "ollama" => {
            let base_url = profile
                .base_url
                .as_deref()
                .unwrap_or("http://localhost:11434");
            let provider = OllamaProvider::new(base_url, ProviderId::from(profile.id.as_str()));
            run_chat_for_provider_with_events(data_dir, profile, provider, prompt, &mut event_sink)
                .await
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
    let state = build_tui_state_with_config(&config, &provider_id)?;
    tessera_tui::run_terminal_chat(state, move |selected_provider_id, prompt, live_events| {
        let data_dir = data_dir.clone();
        let config = config.clone();
        async move {
            run_chat_with_config_and_events(data_dir, &config, &selected_provider_id, prompt, {
                let live_events = live_events.clone();
                move |frame| match live_events
                    .try_send(LiveClientEvent::Frame(Box::new(frame.clone())))
                {
                    Ok(()) => EventSinkAction::Continue,
                    Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                        EventSinkAction::Cancel("live event channel closed".to_string())
                    }
                    Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                        EventSinkAction::Cancel("live event channel full".to_string())
                    }
                }
            })
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
        }
    })
    .await?;
    Ok(())
}

pub fn build_tui_state_with_config(
    config: &TesseraConfig,
    provider_id: &str,
) -> Result<ChatViewState> {
    if !config
        .providers
        .iter()
        .any(|profile| profile.id == provider_id)
    {
        return Err(anyhow::anyhow!("provider profile not found: {provider_id}"));
    }

    let profile_ids = config
        .providers
        .iter()
        .map(|profile| profile.id.clone())
        .collect::<Vec<_>>();
    Ok(ChatViewState::with_profiles(provider_id, profile_ids))
}

async fn run_chat_for_provider_with_events<P, F, R>(
    data_dir: impl AsRef<Path>,
    profile: &ProviderProfile,
    provider: P,
    prompt: impl Into<String>,
    event_sink: &mut F,
) -> Result<ConversationOutcome>
where
    P: ChatProvider,
    F: FnMut(&EventFrame) -> R,
    R: Into<EventSinkAction>,
{
    let store = TraceStore::open(data_dir)?;
    let engine = ConversationEngine::new(provider, store);
    let outcome = engine
        .run_chat_with_event_sink(
            ConversationRequest {
                trace_id: next_trace_id(&profile.id),
                provider_id: ProviderId::from(profile.id.as_str()),
                profile_id: ModelProfileId::from(profile.id.as_str()),
                model: profile.default_model.clone(),
                prompt: prompt.into(),
            },
            event_sink,
        )
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
