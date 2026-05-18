use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use tessera_client::{ClientMessage, ClientMessageRole, ClientSnapshot};
use tessera_config::{ProviderProfile, TesseraConfig};
use tessera_core::{
    ConversationEngine, ConversationOutcome, ConversationRequest, EventSinkAction, ReplayRunner,
    ReplaySummary, RunCancellationToken, RunControls, RunPauseToken, RuntimeEventQuery,
    RuntimeReader, RuntimeSessionSummary,
};
use tessera_protocol::{EventFrame, ModelProfileId, ProviderId, RunEvent, TraceRecord};
use tessera_providers::{
    mock::MockProvider, ollama::OllamaProvider, openai_compatible::OpenAiCompatibleProvider,
    ChatProvider, ProviderMessage,
};
use tessera_storage::TraceStore;
use tessera_tui::{ChatViewState, LiveClientEvent};
use tokio::sync::mpsc;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DoctorReport {
    pub status: String,
    pub data_dir: String,
    pub trace_writable: bool,
    pub sqlite_index_healthy: bool,
    pub provider_profiles: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliSessionSummary {
    pub trace_id: String,
    pub event_count: usize,
    pub updated_at: Option<String>,
    pub last_seq: u64,
    pub last_event_kind: Option<String>,
    pub user_preview: String,
    pub assistant_preview: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliTranscript {
    pub trace_id: String,
    pub messages: Vec<ClientMessage>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliChatOutput {
    pub trace_id: String,
    pub assistant_text: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliReplaySummary {
    pub trace_id: String,
    pub event_count: usize,
    pub event_kinds: Vec<String>,
    pub assistant_text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CliEventPage {
    pub trace_id: String,
    pub records: Vec<TraceRecord>,
    pub next_since_seq: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliProviderProfile {
    pub id: String,
    pub kind: String,
    pub default_model: String,
    pub base_url: Option<String>,
    pub api_key_env: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliConfigValidationReport {
    pub status: String,
    pub data_dir: String,
    pub profiles: Vec<CliConfigProfileValidation>,
    pub issues: Vec<CliConfigValidationIssue>,
}

impl CliConfigValidationReport {
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|issue| issue.severity == "error")
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliConfigProfileValidation {
    pub id: String,
    pub kind: String,
    pub default_model: String,
    pub base_url: Option<String>,
    pub api_key_env: Option<String>,
    pub api_key_env_status: Option<String>,
    pub status: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliConfigValidationIssue {
    pub severity: String,
    pub message: String,
    pub profile_id: Option<String>,
}

impl From<ConversationOutcome> for CliChatOutput {
    fn from(outcome: ConversationOutcome) -> Self {
        Self {
            trace_id: outcome.trace_id,
            assistant_text: outcome.assistant_text,
        }
    }
}

impl From<ReplaySummary> for CliReplaySummary {
    fn from(summary: ReplaySummary) -> Self {
        Self {
            event_count: summary.event_kinds.len(),
            trace_id: summary.trace_id,
            assistant_text: summary.assistant_text,
            event_kinds: summary.event_kinds,
        }
    }
}

impl From<tessera_core::RuntimeEventPage> for CliEventPage {
    fn from(page: tessera_core::RuntimeEventPage) -> Self {
        Self {
            trace_id: page.trace_id,
            records: page.records,
            next_since_seq: page.next_since_seq,
        }
    }
}

impl From<&ProviderProfile> for CliProviderProfile {
    fn from(profile: &ProviderProfile) -> Self {
        Self {
            id: profile.id.clone(),
            kind: profile.kind.clone(),
            default_model: profile.default_model.clone(),
            base_url: profile.base_url.clone(),
            api_key_env: profile.api_key_env.clone(),
        }
    }
}

impl From<RuntimeSessionSummary> for CliSessionSummary {
    fn from(session: RuntimeSessionSummary) -> Self {
        Self {
            trace_id: session.trace_id,
            event_count: session.event_count,
            updated_at: session
                .updated_at
                .map(|timestamp| timestamp.as_str().to_string()),
            last_seq: session.last_seq,
            last_event_kind: session.last_event_kind,
            user_preview: session.user_preview,
            assistant_preview: session.assistant_preview,
        }
    }
}

pub fn format_doctor_lines(report: &DoctorReport) -> Vec<String> {
    vec![
        format!("status: {}", report.status),
        format!("data_dir: {}", report.data_dir),
        format!("trace_writable: {}", report.trace_writable),
        format!("sqlite_index_healthy: {}", report.sqlite_index_healthy),
        format!(
            "provider_profiles: {}",
            if report.provider_profiles.is_empty() {
                "none".to_string()
            } else {
                report.provider_profiles.join(", ")
            }
        ),
    ]
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
    Clear,
    Cancel,
    PauseTask(Option<String>),
    ResumeTask(String),
    Paste,
    Profiles,
    SwitchProfile(String),
    Sessions,
    ResumeSession(String),
    Doctor,
    History,
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
            CliReplCommand::Help => Ok(CliReplCommandOutcome::continue_with(chat_command_lines())),
            CliReplCommand::NewThread => {
                self.snapshot.start_new_thread();
                Ok(CliReplCommandOutcome::continue_with(["new thread started"]))
            }
            CliReplCommand::Clear => {
                self.snapshot.start_new_thread();
                Ok(CliReplCommandOutcome::continue_with([
                    "current thread cleared",
                ]))
            }
            CliReplCommand::Cancel => Ok(CliReplCommandOutcome::continue_with([
                "no active run to cancel",
            ])),
            CliReplCommand::PauseTask(task_id) => {
                let task_label = task_id.unwrap_or_else(|| "latest running task".to_string());
                Ok(CliReplCommandOutcome::continue_with([format!(
                    "pause requested for {task_label} as metadata-only CLI intent; no runtime execution was invoked"
                )]))
            }
            CliReplCommand::ResumeTask(task_id) => {
                Ok(CliReplCommandOutcome::continue_with([format!(
                    "resume requested for {task_id} as metadata-only CLI intent; no runtime execution was invoked"
                )]))
            }
            CliReplCommand::Paste => Ok(CliReplCommandOutcome::continue_with([
                "/paste is only available in the interactive REPL".to_string(),
            ])),
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
            CliReplCommand::Sessions => Ok(CliReplCommandOutcome::continue_with([
                "/sessions requires an active data directory".to_string(),
            ])),
            CliReplCommand::ResumeSession(_) => Ok(CliReplCommandOutcome::continue_with([
                "/resume requires an active data directory".to_string(),
            ])),
            CliReplCommand::Doctor => Ok(CliReplCommandOutcome::continue_with([
                "/doctor requires an active data directory".to_string(),
            ])),
            CliReplCommand::Status => {
                Ok(CliReplCommandOutcome::continue_with([self.status_line()]))
            }
            CliReplCommand::History => Ok(CliReplCommandOutcome::continue_with(
                format_history_lines(&self.snapshot.projection.messages),
            )),
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

    pub fn handle_command_with_data_dir(
        &mut self,
        data_dir: impl AsRef<Path>,
        config: &TesseraConfig,
        command: CliReplCommand,
    ) -> Result<CliReplCommandOutcome> {
        match command {
            CliReplCommand::Sessions => self.list_sessions(data_dir),
            CliReplCommand::ResumeSession(trace_id) => self.resume_session(data_dir, &trace_id),
            CliReplCommand::Doctor => self.doctor(data_dir, config),
            other => self.handle_command(config, other),
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

    fn list_sessions(&self, data_dir: impl AsRef<Path>) -> Result<CliReplCommandOutcome> {
        Ok(CliReplCommandOutcome::continue_with(format_session_lines(
            &list_sessions(data_dir)?,
        )))
    }

    fn doctor(
        &self,
        data_dir: impl AsRef<Path>,
        config: &TesseraConfig,
    ) -> Result<CliReplCommandOutcome> {
        Ok(CliReplCommandOutcome::continue_with(format_doctor_lines(
            &run_doctor_with_config(data_dir, config)?,
        )))
    }

    fn resume_session(
        &mut self,
        data_dir: impl AsRef<Path>,
        selector: &str,
    ) -> Result<CliReplCommandOutcome> {
        let data_dir = data_dir.as_ref();
        let trace_id = resolve_session_selector(data_dir, selector)?;
        let reader = RuntimeReader::new(TraceStore::open(data_dir)?);
        let page = reader.list_events(RuntimeEventQuery::new(trace_id.as_str()))?;
        if page.records.is_empty() {
            return Err(anyhow::anyhow!("trace not found or empty: {trace_id}"));
        }

        self.snapshot.start_new_thread();
        for record in &page.records {
            self.snapshot.apply_trace_record(record);
        }
        let message_count = self.snapshot.projection.messages.len();
        Ok(CliReplCommandOutcome::continue_with([format!(
            "resumed trace {trace_id} ({message_count} messages)"
        )]))
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
        "/help" | "/commands" | "/?" => CliReplCommand::Help,
        "/new" => CliReplCommand::NewThread,
        "/clear" => CliReplCommand::Clear,
        "/cancel" => CliReplCommand::Cancel,
        "/pause" => CliReplCommand::PauseTask(if argument.is_empty() {
            None
        } else {
            Some(argument.to_string())
        }),
        "/resume-task" if !argument.is_empty() => CliReplCommand::ResumeTask(argument.to_string()),
        "/paste" => CliReplCommand::Paste,
        "/profiles" => CliReplCommand::Profiles,
        "/profile" if !argument.is_empty() => CliReplCommand::SwitchProfile(argument.to_string()),
        "/sessions" => CliReplCommand::Sessions,
        "/resume" if !argument.is_empty() => CliReplCommand::ResumeSession(argument.to_string()),
        "/doctor" => CliReplCommand::Doctor,
        "/history" => CliReplCommand::History,
        "/status" => CliReplCommand::Status,
        "/export" => CliReplCommand::Export,
        "/quit" | "/exit" => CliReplCommand::Quit,
        _ => CliReplCommand::Unknown(trimmed.to_string()),
    })
}

pub fn list_sessions(data_dir: impl AsRef<Path>) -> Result<Vec<CliSessionSummary>> {
    let reader = RuntimeReader::new(TraceStore::open(data_dir)?);
    Ok(reader
        .list_sessions()?
        .into_iter()
        .map(CliSessionSummary::from)
        .collect())
}

pub fn latest_session_trace_id(data_dir: impl AsRef<Path>) -> Result<String> {
    list_sessions(data_dir)?
        .into_iter()
        .next()
        .map(|session| session.trace_id)
        .ok_or_else(|| anyhow::anyhow!("no sessions found to continue"))
}

pub fn format_session_lines(sessions: &[CliSessionSummary]) -> Vec<String> {
    if sessions.is_empty() {
        return vec!["no sessions found".to_string()];
    }

    sessions
        .iter()
        .enumerate()
        .map(|(index, session)| {
            let updated_at = session.updated_at.as_deref().unwrap_or("unknown");
            let preview = if !session.user_preview.is_empty() {
                session.user_preview.as_str()
            } else {
                session.assistant_preview.as_str()
            };
            format!(
                "{}. {} | {} events | updated {} | {}",
                index + 1,
                session.trace_id,
                session.event_count,
                updated_at,
                preview
            )
        })
        .collect()
}

fn resolve_session_selector(data_dir: &Path, selector: &str) -> Result<String> {
    let Ok(index) = selector.parse::<usize>() else {
        return Ok(selector.to_string());
    };
    if index == 0 {
        return Err(anyhow::anyhow!(
            "session index out of range: 0 (available sessions: {})",
            list_sessions(data_dir)?.len()
        ));
    }

    let sessions = list_sessions(data_dir)?;
    sessions
        .get(index - 1)
        .map(|session| session.trace_id.clone())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "session index out of range: {index} (available sessions: {})",
                sessions.len()
            )
        })
}

pub fn format_history_lines(messages: &[ClientMessage]) -> Vec<String> {
    if messages.is_empty() {
        return vec!["no messages in current thread".to_string()];
    }

    messages
        .iter()
        .enumerate()
        .map(|(index, message)| {
            let role = match message.role {
                ClientMessageRole::System => "system",
                ClientMessageRole::User => "user",
                ClientMessageRole::Assistant => "assistant",
                ClientMessageRole::Reasoning => "reasoning",
            };
            format!(
                "{}. {role}: {}",
                index + 1,
                compact_history_preview(&message.content)
            )
        })
        .collect()
}

fn compact_history_preview(content: &str) -> String {
    let collapsed = content.split_whitespace().collect::<Vec<_>>().join(" ");
    const MAX_CHARS: usize = 120;
    if collapsed.chars().count() <= MAX_CHARS {
        return collapsed;
    }

    collapsed
        .chars()
        .take(MAX_CHARS.saturating_sub(3))
        .chain("...".chars())
        .collect()
}

pub fn load_transcript(data_dir: impl AsRef<Path>, trace_id: &str) -> Result<CliTranscript> {
    let snapshot = load_transcript_snapshot(data_dir, trace_id)?;
    Ok(CliTranscript {
        trace_id: trace_id.to_string(),
        messages: snapshot.projection.messages,
    })
}

pub fn export_transcript_markdown(data_dir: impl AsRef<Path>, trace_id: &str) -> Result<String> {
    Ok(load_transcript_snapshot(data_dir, trace_id)?.export_markdown())
}

pub fn replay_trace(data_dir: impl AsRef<Path>, trace_id: &str) -> Result<CliReplaySummary> {
    let store = TraceStore::open(data_dir)?;
    Ok(CliReplaySummary::from(
        ReplayRunner::new(&store).replay(trace_id)?,
    ))
}

pub fn format_replay_summary(summary: &CliReplaySummary) -> String {
    format!(
        "trace: {}\nevents: {}\nassistant:\n{}\n",
        summary.trace_id, summary.event_count, summary.assistant_text
    )
}

pub fn list_events(
    data_dir: impl AsRef<Path>,
    trace_id: &str,
    since_seq: Option<u64>,
    limit: Option<usize>,
) -> Result<CliEventPage> {
    let reader = RuntimeReader::new(TraceStore::open(data_dir)?);
    let mut query = RuntimeEventQuery::new(trace_id);
    if let Some(since_seq) = since_seq {
        query = query.since_seq(since_seq);
    }
    if let Some(limit) = limit {
        query = query.limit(limit);
    }
    Ok(CliEventPage::from(reader.list_events(query)?))
}

pub fn format_event_lines(page: &CliEventPage) -> Vec<String> {
    let mut lines = vec![format!("trace: {}", page.trace_id)];
    lines.extend(page.records.iter().map(|record| {
        format!(
            "{} | {} | {}",
            record.seq,
            record.timestamp.as_str(),
            record.event_kind
        )
    }));
    lines.push(format!(
        "next_since_seq: {}",
        page.next_since_seq
            .map(|seq| seq.to_string())
            .unwrap_or_else(|| "none".to_string())
    ));
    lines
}

pub fn list_profiles(config: &TesseraConfig) -> Vec<CliProviderProfile> {
    config
        .providers
        .iter()
        .map(CliProviderProfile::from)
        .collect()
}

pub fn format_profile_lines(profiles: &[CliProviderProfile]) -> Vec<String> {
    if profiles.is_empty() {
        return vec!["no provider profiles configured".to_string()];
    }

    profiles
        .iter()
        .map(|profile| {
            let base_url = profile.base_url.as_deref().unwrap_or("none");
            let api_key_env = profile.api_key_env.as_deref().unwrap_or("none");
            format!(
                "{} | {} | model {} | base_url {} | api_key_env {}",
                profile.id, profile.kind, profile.default_model, base_url, api_key_env
            )
        })
        .collect()
}

pub fn validate_config(
    config: &TesseraConfig,
    data_dir: impl AsRef<Path>,
) -> CliConfigValidationReport {
    let mut issues = Vec::new();
    let mut seen_profile_ids = HashSet::new();
    let mut profiles = Vec::new();

    if config.providers.is_empty() {
        issues.push(config_validation_error(
            None,
            "at least one provider profile is required",
        ));
    }

    for profile in &config.providers {
        let mut status = "ok".to_string();
        if !seen_profile_ids.insert(profile.id.clone()) {
            status = "error".to_string();
            issues.push(config_validation_error(
                Some(&profile.id),
                format!("duplicate provider id `{}`", profile.id),
            ));
        }

        match profile.kind.as_str() {
            "mock" | "ollama" => {}
            "openai-compatible" | "openai_compatible" => {
                if profile.base_url.is_none() {
                    status = "error".to_string();
                    issues.push(config_validation_error(
                        Some(&profile.id),
                        format!(
                            "provider `{}` kind openai-compatible requires base_url",
                            profile.id
                        ),
                    ));
                }
            }
            other => {
                status = "error".to_string();
                issues.push(config_validation_error(
                    Some(&profile.id),
                    format!(
                        "unsupported provider kind `{other}` for profile `{}`",
                        profile.id
                    ),
                ));
            }
        }

        let api_key_env_status = profile.api_key_env.as_ref().map(|env_name| {
            if std::env::var_os(env_name).is_some() {
                "set".to_string()
            } else {
                status = "error".to_string();
                issues.push(config_validation_error(
                    Some(&profile.id),
                    format!(
                        "provider `{}` api_key_env `{env_name}` is not set",
                        profile.id
                    ),
                ));
                "missing".to_string()
            }
        });

        profiles.push(CliConfigProfileValidation {
            id: profile.id.clone(),
            kind: profile.kind.clone(),
            default_model: profile.default_model.clone(),
            base_url: profile.base_url.clone(),
            api_key_env: profile.api_key_env.clone(),
            api_key_env_status,
            status,
        });
    }

    let status = if issues.iter().any(|issue| issue.severity == "error") {
        "error"
    } else {
        "ok"
    };

    CliConfigValidationReport {
        status: status.to_string(),
        data_dir: data_dir.as_ref().to_string_lossy().to_string(),
        profiles,
        issues,
    }
}

pub fn format_config_validation_lines(report: &CliConfigValidationReport) -> Vec<String> {
    let mut lines = vec![
        format!("status: {}", report.status),
        format!("data_dir: {}", report.data_dir),
    ];

    for profile in &report.profiles {
        let mut details = vec![
            profile.kind.clone(),
            format!("model {}", profile.default_model),
        ];
        if let Some(api_key_env) = &profile.api_key_env {
            let api_key_env_status = profile.api_key_env_status.as_deref().unwrap_or("unknown");
            details.push(format!("api_key_env {api_key_env} {api_key_env_status}"));
        }
        lines.push(format!(
            "profile {}: {} ({})",
            profile.id,
            profile.status,
            details.join(", ")
        ));
    }

    for issue in &report.issues {
        lines.push(format!("{}: {}", issue.severity, issue.message));
    }

    lines
}

fn config_validation_error(
    profile_id: Option<&str>,
    message: impl Into<String>,
) -> CliConfigValidationIssue {
    CliConfigValidationIssue {
        severity: "error".to_string(),
        message: message.into(),
        profile_id: profile_id.map(str::to_string),
    }
}

fn load_transcript_snapshot(data_dir: impl AsRef<Path>, trace_id: &str) -> Result<ClientSnapshot> {
    let reader = RuntimeReader::new(TraceStore::open(data_dir)?);
    let page = reader.list_events(RuntimeEventQuery::new(trace_id))?;
    if page.records.is_empty() {
        return Err(anyhow::anyhow!("trace not found or empty: {trace_id}"));
    }

    let mut snapshot = ClientSnapshot::new("transcript");
    for record in &page.records {
        snapshot.apply_trace_record(record);
    }
    Ok(snapshot)
}

pub fn provider_history_from_snapshot(snapshot: &ClientSnapshot) -> Vec<ProviderMessage> {
    snapshot
        .projection
        .messages
        .iter()
        .filter(|message| !message.content.trim().is_empty())
        .filter_map(|message| match message.role {
            ClientMessageRole::User => Some(ProviderMessage::user(message.content.clone())),
            ClientMessageRole::Assistant => {
                Some(ProviderMessage::assistant(message.content.clone()))
            }
            ClientMessageRole::System | ClientMessageRole::Reasoning => None,
        })
        .collect()
}

pub async fn run_repl_prompt_with_writer<F>(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
    session: &mut CliReplSession,
    prompt: impl Into<String>,
    write_delta: F,
) -> Result<ConversationOutcome>
where
    F: FnMut(&str),
{
    run_repl_prompt_with_writer_and_controls(
        data_dir,
        config,
        session,
        prompt,
        RunControls::default(),
        write_delta,
    )
    .await
}

pub async fn run_repl_prompt_with_writer_and_controls<F>(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
    session: &mut CliReplSession,
    prompt: impl Into<String>,
    controls: RunControls,
    mut write_delta: F,
) -> Result<ConversationOutcome>
where
    F: FnMut(&str),
{
    let provider_id = session.snapshot.status.active_profile.clone();
    let history = provider_history_from_snapshot(&session.snapshot);
    let snapshot = &mut session.snapshot;
    run_chat_with_config_history_controls_and_events(
        data_dir,
        config,
        &provider_id,
        prompt,
        history,
        controls,
        |frame| {
            snapshot.apply_event(frame);
            if let RunEvent::AssistantDelta { text, .. } = &frame.event {
                write_delta(text);
            }
            EventSinkAction::Continue
        },
    )
    .await
}

pub async fn run_chat_repl_with_config(
    data_dir: PathBuf,
    config: TesseraConfig,
    provider_id: String,
) -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_chat_repl_with_io_and_resume(
        data_dir,
        config,
        provider_id,
        None,
        BufReader::new(stdin),
        stdout.lock(),
    )
    .await?;
    Ok(())
}

pub async fn run_chat_repl_with_config_and_resume(
    data_dir: PathBuf,
    config: TesseraConfig,
    provider_id: String,
    resume_trace_id: Option<String>,
) -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_chat_repl_with_io_and_resume(
        data_dir,
        config,
        provider_id,
        resume_trace_id,
        BufReader::new(stdin),
        stdout.lock(),
    )
    .await?;
    Ok(())
}

pub async fn run_chat_repl_with_io<R, W>(
    data_dir: PathBuf,
    config: TesseraConfig,
    provider_id: String,
    input: R,
    output: W,
) -> Result<ClientSnapshot>
where
    R: BufRead + Send + 'static,
    W: Write,
{
    run_chat_repl_with_io_and_resume(data_dir, config, provider_id, None, input, output).await
}

const REPL_LINE_BUFFER_CAPACITY: usize = 128;

type ReplLineReceiver = mpsc::Receiver<io::Result<String>>;

fn spawn_repl_line_reader<R>(mut input: R) -> ReplLineReceiver
where
    R: BufRead + Send + 'static,
{
    let (sender, receiver) = mpsc::channel(REPL_LINE_BUFFER_CAPACITY);
    thread::spawn(move || loop {
        let mut line = String::new();
        match input.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                if sender.blocking_send(Ok(line)).is_err() {
                    break;
                }
            }
            Err(error) => {
                let _ = sender.blocking_send(Err(error));
                break;
            }
        }
    });
    receiver
}

async fn next_repl_line(
    input_lines: &mut ReplLineReceiver,
    pending_lines: &mut VecDeque<String>,
) -> Result<Option<String>> {
    if let Some(line) = pending_lines.pop_front() {
        return Ok(Some(line));
    }
    match input_lines.recv().await {
        Some(Ok(line)) => Ok(Some(line)),
        Some(Err(error)) => Err(error.into()),
        None => Ok(None),
    }
}

pub async fn run_chat_repl_with_io_and_resume<R, W>(
    data_dir: PathBuf,
    config: TesseraConfig,
    provider_id: String,
    resume_trace_id: Option<String>,
    input: R,
    mut output: W,
) -> Result<ClientSnapshot>
where
    R: BufRead + Send + 'static,
    W: Write,
{
    let mut input_lines = spawn_repl_line_reader(input);
    let mut pending_lines = VecDeque::new();
    let mut session = CliReplSession::new(&config, &provider_id)?;
    for line in repl_startup_lines(&data_dir, &config, &provider_id) {
        writeln!(output, "{line}")?;
    }
    if let Some(trace_id) = resume_trace_id {
        let outcome = session.handle_command_with_data_dir(
            &data_dir,
            &config,
            CliReplCommand::ResumeSession(trace_id),
        )?;
        for line in outcome.lines {
            writeln!(output, "{line}")?;
        }
    }

    loop {
        write!(
            output,
            "\ntessera({})> ",
            session.snapshot.status.active_profile
        )?;
        output.flush()?;

        let Some(line) = next_repl_line(&mut input_lines, &mut pending_lines).await? else {
            break;
        };
        let user_input = line.trim();
        if user_input.is_empty() {
            continue;
        }

        if let Some(command) = parse_repl_command(user_input) {
            if matches!(command, CliReplCommand::Paste) {
                writeln!(output, "paste mode; end with /send or /cancel")?;
                let mut pasted = String::new();
                let mut should_quit = false;
                loop {
                    write!(output, "paste> ")?;
                    output.flush()?;

                    let Some(line) = next_repl_line(&mut input_lines, &mut pending_lines).await?
                    else {
                        should_quit = true;
                        break;
                    };
                    let pasted_line = line.trim_end_matches(['\r', '\n']);
                    match pasted_line {
                        "/send" => {
                            if pasted.trim().is_empty() {
                                writeln!(output, "paste is empty; nothing sent")?;
                            } else {
                                run_repl_prompt_and_write(
                                    &data_dir,
                                    &config,
                                    &mut session,
                                    pasted,
                                    &mut output,
                                    &mut input_lines,
                                    &mut pending_lines,
                                )
                                .await?;
                            }
                            break;
                        }
                        "/cancel" => {
                            writeln!(output, "paste cancelled")?;
                            break;
                        }
                        _ => {
                            if !pasted.is_empty() {
                                pasted.push('\n');
                            }
                            pasted.push_str(pasted_line);
                        }
                    }
                }
                if should_quit {
                    break;
                }
                continue;
            }

            match session.handle_command_with_data_dir(&data_dir, &config, command) {
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

        run_repl_prompt_and_write(
            &data_dir,
            &config,
            &mut session,
            user_input.to_string(),
            &mut output,
            &mut input_lines,
            &mut pending_lines,
        )
        .await?;
    }

    Ok(session.snapshot)
}

async fn run_repl_prompt_and_write<W>(
    data_dir: &Path,
    config: &TesseraConfig,
    session: &mut CliReplSession,
    prompt: String,
    output: &mut W,
    input_lines: &mut ReplLineReceiver,
    pending_lines: &mut VecDeque<String>,
) -> Result<()>
where
    W: Write,
{
    write!(output, "assistant> ")?;
    output.flush()?;

    let cancellation_token = RunCancellationToken::new();
    let pause_token = RunPauseToken::new();
    let controls = RunControls {
        event_timeout: None,
        cancellation_token: Some(cancellation_token.clone()),
        pause_token: Some(pause_token.clone()),
    };
    let (delta_tx, mut delta_rx) = mpsc::unbounded_channel::<String>();
    let run = run_repl_prompt_with_writer_and_controls(
        data_dir,
        config,
        session,
        prompt,
        controls,
        move |delta| {
            let _ = delta_tx.send(delta.to_string());
        },
    );
    tokio::pin!(run);
    let mut input_closed = false;
    let mut cancel_announced = false;
    let mut pause_announced = false;

    loop {
        tokio::select! {
            result = &mut run => {
                while let Ok(delta) = delta_rx.try_recv() {
                    write!(output, "{delta}")?;
                    output.flush()?;
                }
                result?;
                writeln!(output)?;
                return Ok(());
            }
            maybe_delta = delta_rx.recv() => {
                if let Some(delta) = maybe_delta {
                    write!(output, "{delta}")?;
                    output.flush()?;
                }
            }
            maybe_line = input_lines.recv(), if !input_closed => {
                match maybe_line {
                    Some(Ok(line)) => {
                        if pending_lines.is_empty() {
                            match parse_repl_command(line.trim()) {
                                Some(CliReplCommand::Cancel) => {
                                    cancellation_token.cancel("cli repl cancel requested");
                                    if !cancel_announced {
                                        writeln!(output, "\ncancel requested")?;
                                        output.flush()?;
                                        cancel_announced = true;
                                    }
                                }
                                Some(CliReplCommand::PauseTask(_)) => {
                                    pause_token.pause("cli repl pause requested");
                                    if !pause_announced {
                                        writeln!(output, "\npause requested")?;
                                        output.flush()?;
                                        pause_announced = true;
                                    }
                                }
                                _ => pending_lines.push_back(line),
                            }
                        } else {
                            pending_lines.push_back(line);
                        }
                    }
                    Some(Err(error)) => return Err(error.into()),
                    None => {
                        input_closed = true;
                    }
                }
            }
        }
    }
}

pub fn repl_startup_lines(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
    provider_id: &str,
) -> Vec<String> {
    let available_profiles = config
        .providers
        .iter()
        .map(|profile| profile.id.as_str())
        .collect::<Vec<_>>();

    vec![
        "Tessera CLI interactive chat".to_string(),
        format!("active_profile: {provider_id}"),
        format!("data_dir: {}", data_dir.as_ref().display()),
        format!(
            "available_profiles: {}",
            if available_profiles.is_empty() {
                "none".to_string()
            } else {
                available_profiles.join(", ")
            }
        ),
        "type /help or run `tessera chat --list-commands` for commands; use /doctor for runtime health, /quit to exit".to_string(),
    ]
}

pub fn chat_command_lines() -> Vec<&'static str> {
    vec![
        "commands:",
        "  /help, /commands   show this help",
        "  /new               start a fresh visible thread",
        "  /clear             clear the current visible thread",
        "  /cancel            cancel active paste/run when available",
        "  /pause [task_id]   pause active run when available; otherwise record metadata-only intent",
        "  /resume-task <task_id> record a metadata-only resume intent",
        "  /paste             enter multiline prompt mode",
        "  /profiles          list configured provider profiles",
        "  /profile <id>      switch active provider profile",
        "  /sessions          list trace-backed sessions",
        "  /resume <trace_id|#> project a trace into this session",
        "  /doctor            show runtime health for this session",
        "  /history           list current visible messages",
        "  /status            show compact runtime status",
        "  /export            print markdown transcript",
        "  /quit, /exit       leave the REPL",
    ]
}

pub fn default_config_template() -> &'static str {
    r#"# Tessera local configuration
# This template stores provider secret *environment variable names* only.
# Do not paste API keys or bearer tokens into this file.

data_dir = "./.tessera"

[[providers]]
id = "mock"
kind = "mock"
default_model = "mock-chat"

[[providers]]
id = "ollama"
kind = "ollama"
default_model = "llama3"
base_url = "http://localhost:11434"

[[providers]]
id = "openai-compatible"
kind = "openai-compatible"
default_model = "deepseek-chat"
base_url = "https://api.example.com/v1"
api_key_env = "TESSERA_OPENAI_COMPATIBLE_API_KEY"
"#
}

pub fn write_config_template(path: impl AsRef<Path>, force: bool) -> Result<PathBuf> {
    let path = path.as_ref();
    if path.exists() && !force {
        return Err(anyhow::anyhow!(
            "config file already exists: {} (pass --force to overwrite)",
            path.display()
        ));
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(path, default_config_template())?;
    Ok(path.to_path_buf())
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
    event_sink: F,
) -> Result<ConversationOutcome>
where
    F: FnMut(&EventFrame) -> R,
    R: Into<EventSinkAction>,
{
    run_chat_with_config_and_controls_and_events(
        data_dir,
        config,
        provider_id,
        prompt,
        RunControls::default(),
        event_sink,
    )
    .await
}

pub async fn run_chat_with_config_and_controls_and_events<F, R>(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
    provider_id: &str,
    prompt: impl Into<String>,
    controls: RunControls,
    event_sink: F,
) -> Result<ConversationOutcome>
where
    F: FnMut(&EventFrame) -> R,
    R: Into<EventSinkAction>,
{
    run_chat_with_config_history_controls_and_events(
        data_dir,
        config,
        provider_id,
        prompt,
        Vec::new(),
        controls,
        event_sink,
    )
    .await
}

pub async fn run_chat_with_config_history_and_events<F, R>(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
    provider_id: &str,
    prompt: impl Into<String>,
    history: Vec<ProviderMessage>,
    event_sink: F,
) -> Result<ConversationOutcome>
where
    F: FnMut(&EventFrame) -> R,
    R: Into<EventSinkAction>,
{
    run_chat_with_config_history_controls_and_events(
        data_dir,
        config,
        provider_id,
        prompt,
        history,
        RunControls::default(),
        event_sink,
    )
    .await
}

pub async fn run_chat_with_config_history_controls_and_events<F, R>(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
    provider_id: &str,
    prompt: impl Into<String>,
    history: Vec<ProviderMessage>,
    controls: RunControls,
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
                history,
                controls,
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
            run_chat_for_provider_with_events(
                data_dir,
                profile,
                provider,
                prompt,
                history,
                controls,
                &mut event_sink,
            )
            .await
        }
        "ollama" => {
            let base_url = profile
                .base_url
                .as_deref()
                .unwrap_or("http://localhost:11434");
            let provider = OllamaProvider::new(base_url, ProviderId::from(profile.id.as_str()));
            run_chat_for_provider_with_events(
                data_dir,
                profile,
                provider,
                prompt,
                history,
                controls,
                &mut event_sink,
            )
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
    let active_cancellation_token: Arc<Mutex<Option<RunCancellationToken>>> =
        Arc::new(Mutex::new(None));
    let active_pause_token: Arc<Mutex<Option<RunPauseToken>>> = Arc::new(Mutex::new(None));
    let submit_cancellation_token = Arc::clone(&active_cancellation_token);
    let submit_pause_token = Arc::clone(&active_pause_token);
    let cancel_cancellation_token = Arc::clone(&active_cancellation_token);
    let pause_pause_token = Arc::clone(&active_pause_token);

    tessera_tui::run_terminal_chat_with_runtime_handlers(
        state,
        move |selected_provider_id, prompt, live_events| {
            let data_dir = data_dir.clone();
            let config = config.clone();
            let active_cancellation_token = Arc::clone(&submit_cancellation_token);
            let active_pause_token = Arc::clone(&submit_pause_token);
            async move {
                let cancellation_token = RunCancellationToken::new();
                let pause_token = RunPauseToken::new();
                {
                    let mut active = active_cancellation_token
                        .lock()
                        .map_err(|_| "active cancellation token lock poisoned".to_string())?;
                    *active = Some(cancellation_token.clone());
                }
                {
                    let mut active = active_pause_token
                        .lock()
                        .map_err(|_| "active pause token lock poisoned".to_string())?;
                    *active = Some(pause_token.clone());
                }

                let controls = RunControls {
                    event_timeout: None,
                    cancellation_token: Some(cancellation_token.clone()),
                    pause_token: Some(pause_token.clone()),
                };
                let result = run_chat_with_config_and_controls_and_events(
                    data_dir,
                    &config,
                    &selected_provider_id,
                    prompt,
                    controls,
                    {
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
                    },
                )
                .await
                .map(|_| ())
                .map_err(|error| error.to_string());

                let mut active = active_cancellation_token
                    .lock()
                    .map_err(|_| "active cancellation token lock poisoned".to_string())?;
                if active
                    .as_ref()
                    .is_some_and(|current| current.is_same_handle(&cancellation_token))
                {
                    *active = None;
                }
                let mut active = active_pause_token
                    .lock()
                    .map_err(|_| "active pause token lock poisoned".to_string())?;
                if active
                    .as_ref()
                    .is_some_and(|current| current.is_same_handle(&pause_token))
                {
                    *active = None;
                }

                result
            }
        },
        move |_task_id| {
            let token = cancel_cancellation_token
                .lock()
                .map_err(|_| "active cancellation token lock poisoned".to_string())?
                .clone();
            match token {
                Some(token) => {
                    token.cancel("tui cancel requested");
                    Ok("cancel requested".to_string())
                }
                None => Err("no active run to cancel".to_string()),
            }
        },
        move |_task_id| {
            let token = pause_pause_token
                .lock()
                .map_err(|_| "active pause token lock poisoned".to_string())?
                .clone();
            match token {
                Some(token) => {
                    token.pause("tui pause requested");
                    Ok("pause requested".to_string())
                }
                None => Err("no active run to pause".to_string()),
            }
        },
    )
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
    history: Vec<ProviderMessage>,
    controls: RunControls,
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
        .run_chat_with_controls_and_event_sink(
            ConversationRequest {
                trace_id: next_trace_id(&profile.id),
                provider_id: ProviderId::from(profile.id.as_str()),
                profile_id: ModelProfileId::from(profile.id.as_str()),
                model: profile.default_model.clone(),
                prompt: prompt.into(),
                history,
            },
            controls,
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
