use futures::TryStreamExt;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tessera_protocol::{
    AgentProfile, AgentProfileId, AgentRunSummary, AgentStepStatus, AgentStepSummary, ArtifactId,
    ArtifactKind, ContextBudget, ContextId, ContextPlacement, ContextReference, ContextSource,
    ContextSourceKind, Diagnostic, DiagnosticReport, DiagnosticReportId, EventFrame, EventRange,
    ExtensionMap, InstructionLoadStatus, InstructionRedactionStatus, InstructionSource,
    InstructionSourceKind, ItemId, ModelProfileId, NoProgressAction, NoProgressLoop,
    NoProgressSignalKind, OsSandboxFilesystem, OsSandboxMode, OsSandboxNetwork, OsSandboxProfile,
    OsSandboxProfileId, OsSandboxShell, PolicyDecisionId, PolicyOutcome, ProviderCapability,
    ProviderId, ResumeMode, RouteDecision, RouteDecisionId, RouteStrategy, RunEvent,
    SandboxDecision, SandboxDecisionId, SandboxDecisionKind, SkillActivation,
    SkillActivationStatus, SkillActivationStep, SkillEntrypoint, SkillEntrypointFormat, SkillId,
    SkillLoadStatus, SkillManifest, SkillPolicy, SkillRedactionStatus, SkillReferenceSource,
    SkillRequirements, SkillSource, SkillSourceKind, SkillStepKind, SkillStepStatus, SnapshotId,
    SnapshotKind, TaskId, TaskKind, TaskPauseCheckpoint, TaskPauseCheckpointId, TaskStatus,
    ThreadId, Timestamp, ToolCallRequest, ToolDescriptor, ToolDispatch, ToolId, ToolPermission,
    ToolPolicyDecision, ToolRepairId, ToolRepairKind, ToolRepairReport, ToolResult, ToolSideEffect,
    TraceRecord, TurnId, WorkspaceAccess, WorkspaceCheckpoint, WorkspaceGuardrail, WorkspaceScope,
};
use tessera_providers::{ChatProvider, ProviderError, ProviderMessage, ProviderRequest};
use tessera_storage::TraceStore;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("provider failed: {0}")]
    Provider(#[from] tessera_providers::ProviderError),
    #[error("storage failed: {0}")]
    Storage(#[from] tessera_storage::StorageError),
    #[error("json failed: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventSinkAction {
    Continue,
    Cancel(String),
}

impl EventSinkAction {
    fn cancel_reason(self) -> Option<String> {
        match self {
            Self::Continue => None,
            Self::Cancel(reason) => Some(reason),
        }
    }
}

impl From<()> for EventSinkAction {
    fn from(_: ()) -> Self {
        Self::Continue
    }
}

#[derive(Clone)]
pub struct RunCancellationToken {
    inner: Arc<RunCancellationState>,
}

struct RunCancellationState {
    reason: Mutex<Option<String>>,
    notify: tokio::sync::Notify,
}

impl RunCancellationToken {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RunCancellationState {
                reason: Mutex::new(None),
                notify: tokio::sync::Notify::new(),
            }),
        }
    }

    pub fn cancel(&self, reason: impl Into<String>) {
        let mut guard = self
            .inner
            .reason
            .lock()
            .expect("cancellation mutex poisoned");
        if guard.is_none() {
            *guard = Some(reason.into());
            self.inner.notify.notify_waiters();
        }
    }

    pub fn cancellation_reason(&self) -> Option<String> {
        self.inner
            .reason
            .lock()
            .expect("cancellation mutex poisoned")
            .clone()
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancellation_reason().is_some()
    }

    pub fn is_same_handle(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    async fn cancelled(&self) -> String {
        loop {
            if let Some(reason) = self.cancellation_reason() {
                return reason;
            }
            self.inner.notify.notified().await;
        }
    }
}

impl Default for RunCancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for RunCancellationToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RunCancellationToken")
            .field("is_cancelled", &self.is_cancelled())
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub struct RunPauseToken {
    inner: Arc<RunPauseState>,
}

struct RunPauseState {
    reason: Mutex<Option<String>>,
    notify: tokio::sync::Notify,
}

impl RunPauseToken {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RunPauseState {
                reason: Mutex::new(None),
                notify: tokio::sync::Notify::new(),
            }),
        }
    }

    pub fn pause(&self, reason: impl Into<String>) {
        let mut guard = self.inner.reason.lock().expect("pause mutex poisoned");
        if guard.is_none() {
            *guard = Some(reason.into());
            self.inner.notify.notify_waiters();
        }
    }

    pub fn pause_reason(&self) -> Option<String> {
        self.inner
            .reason
            .lock()
            .expect("pause mutex poisoned")
            .clone()
    }

    pub fn is_paused(&self) -> bool {
        self.pause_reason().is_some()
    }

    pub fn is_same_handle(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    async fn paused(&self) -> String {
        loop {
            if let Some(reason) = self.pause_reason() {
                return reason;
            }
            self.inner.notify.notified().await;
        }
    }
}

impl Default for RunPauseToken {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for RunPauseToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RunPauseToken")
            .field("is_paused", &self.is_paused())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, Default)]
pub struct RunControls {
    pub event_timeout: Option<Duration>,
    pub cancellation_token: Option<RunCancellationToken>,
    pub pause_token: Option<RunPauseToken>,
}

enum RunControlSignal {
    Cancelled(String),
    Paused(String),
}

async fn next_run_control_signal(
    cancellation_token: Option<&RunCancellationToken>,
    pause_token: Option<&RunPauseToken>,
) -> RunControlSignal {
    match (cancellation_token, pause_token) {
        (Some(cancellation_token), Some(pause_token)) => {
            tokio::select! {
                reason = cancellation_token.cancelled() => RunControlSignal::Cancelled(reason),
                reason = pause_token.paused() => RunControlSignal::Paused(reason),
            }
        }
        (Some(cancellation_token), None) => {
            RunControlSignal::Cancelled(cancellation_token.cancelled().await)
        }
        (None, Some(pause_token)) => RunControlSignal::Paused(pause_token.paused().await),
        (None, None) => std::future::pending().await,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversationRequest {
    pub trace_id: String,
    pub provider_id: ProviderId,
    pub profile_id: ModelProfileId,
    pub model: String,
    pub prompt: String,
    pub history: Vec<ProviderMessage>,
}

impl ConversationRequest {
    pub fn mock(prompt: impl Into<String>) -> Self {
        Self {
            trace_id: "trace_mock".to_string(),
            provider_id: ProviderId::from_static("mock"),
            profile_id: ModelProfileId::from_static("mock-default"),
            model: "mock-chat".to_string(),
            prompt: prompt.into(),
            history: Vec::new(),
        }
    }

    pub fn provider_messages(&self) -> Vec<ProviderMessage> {
        let mut messages = self.history.clone();
        messages.push(ProviderMessage::user(self.prompt.clone()));
        messages
    }
}

pub struct ConversationOutcome {
    pub trace_id: String,
    pub assistant_text: String,
    pub store: TraceStore,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentRunRequest {
    pub trace_id: String,
    pub provider_id: ProviderId,
    pub profile_id: ModelProfileId,
    pub agent_profile: AgentProfile,
    pub model: String,
    pub objective: String,
    pub context_references: Vec<ContextReference>,
    pub instruction_context: Option<LoadedInstructionSet>,
    pub skill_context: Option<LoadedSkillSet>,
    pub history: Vec<ProviderMessage>,
    pub max_steps: u32,
}

impl AgentRunRequest {
    pub fn provider_messages(&self) -> Vec<ProviderMessage> {
        let mut messages = Vec::new();
        if let Some(instruction_context) = &self.instruction_context {
            if !instruction_context.loaded.is_empty() {
                messages.push(ProviderMessage::system(render_instruction_system_message(
                    instruction_context,
                )));
            }
        }
        if let Some(skill_context) = &self.skill_context {
            if !skill_context.skills.is_empty() {
                messages.push(ProviderMessage::system(
                    skill_context.render_system_message(),
                ));
            }
        }
        messages.extend(self.history.clone());
        messages.push(ProviderMessage::user(self.objective.clone()));
        messages
    }
}

pub struct AgentRunOutcome {
    pub trace_id: String,
    pub task_id: TaskId,
    pub status: TaskStatus,
    pub summary: AgentRunSummary,
    pub assistant_text: String,
    pub store: TraceStore,
}

pub struct AgentLoop<P> {
    provider: P,
    store: TraceStore,
}

pub struct ConversationEngine<P> {
    provider: P,
    store: TraceStore,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRouteRequest {
    pub requested_profile: Option<ModelProfileId>,
    pub default_profile: ModelProfileId,
    pub requested_model: String,
    pub reasoning_level: Option<String>,
    pub provider_capability: Option<ProviderCapability>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModelRouter;

impl ModelRouter {
    pub fn draft() -> Self {
        Self
    }

    pub fn route(&self, request: ModelRouteRequest) -> RouteDecision {
        let requested_profile = request.requested_profile.clone();
        let selected_profile = requested_profile
            .clone()
            .unwrap_or_else(|| request.default_profile.clone());
        let strategy = if requested_profile.is_some() {
            RouteStrategy::Manual
        } else {
            RouteStrategy::DefaultProfile
        };
        let reason = if requested_profile.is_some() {
            "manual_profile_selected_auto_routing_disabled"
        } else {
            "default_profile_selected_auto_routing_disabled"
        };

        RouteDecision {
            requested_profile,
            selected_profile,
            selected_model: request.requested_model,
            reasoning_level: request.reasoning_level,
            strategy,
            decision_reason: Some(reason.to_string()),
            fallback_reason: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NoProgressPolicy {
    pub no_output_threshold: u32,
    pub repeated_read_only_threshold: u32,
    pub repeated_repair_threshold: u32,
}

impl Default for NoProgressPolicy {
    fn default() -> Self {
        Self {
            no_output_threshold: 1,
            repeated_read_only_threshold: 3,
            repeated_repair_threshold: 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NoProgressObservation {
    AssistantOutput,
    NoOutput,
    ReadOnlyStep,
    RepairStep,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NoProgressDetector {
    policy: NoProgressPolicy,
    no_output_count: u32,
    read_only_count: u32,
    repair_count: u32,
    current_assistant_message_has_output: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SkillRegistry {
    manifests: Vec<SkillManifest>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillDiscoveryOptions {
    pub workspace_root: PathBuf,
    pub target_dir: PathBuf,
    pub skill_roots: Vec<PathBuf>,
    pub entrypoint_name: String,
    pub entrypoint_byte_limit: usize,
    pub reference_byte_limit: usize,
    pub combined_byte_limit: usize,
    pub placement: ContextPlacement,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SkillDiscoveryReport {
    pub manifests: Vec<SkillManifest>,
    pub sources: Vec<SkillReferenceSource>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillActivationRequest {
    pub skill: String,
    pub references: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillRuntimeOptions {
    pub discovery: SkillDiscoveryOptions,
    pub requests: Vec<SkillActivationRequest>,
    pub best_effort_references: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedSkillReference {
    pub source: SkillReferenceSource,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedSkill {
    pub manifest: SkillManifest,
    pub entrypoint: SkillReferenceSource,
    pub text: String,
    pub references: Vec<LoadedSkillReference>,
    pub activation: SkillActivation,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LoadedSkillSet {
    pub activations: Vec<SkillActivation>,
    pub skills: Vec<LoadedSkill>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SkillRuntimePlanner;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AgentRegistry {
    profiles: Vec<AgentProfile>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ToolRegistry {
    descriptors: Vec<ToolDescriptor>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct McpToolAnnotations {
    pub title: Option<String>,
    pub read_only_hint: Option<bool>,
    pub destructive_hint: Option<bool>,
    pub idempotent_hint: Option<bool>,
    pub open_world_hint: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct McpToolSpec {
    pub server_id: String,
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
    pub output_schema: Option<serde_json::Value>,
    pub annotations: McpToolAnnotations,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct McpToolAdapter;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiagnosticsReporter;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OrderedToolResultBuffer {
    dispatches: Vec<ToolDispatch>,
    order: Vec<u32>,
    next_cursor: usize,
    pending_results: BTreeMap<u32, ToolResult>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ToolRepairTelemetry;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PolicyGate;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceGuardrailChecker {
    scope: WorkspaceScope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OsSandboxPlanner {
    workspace_root: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceCheckpointPlanner {
    kind: SnapshotKind,
    storage_prefix: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextBudgetSummary {
    pub max_tokens: u64,
    pub reserved_output_tokens: u64,
    pub available_tokens: u64,
    pub used_tokens: u64,
    pub remaining_tokens: u64,
    pub stable_prefix_tokens: u64,
    pub append_only_transcript_tokens: u64,
    pub volatile_scratch_tokens: u64,
    pub over_budget: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextProjection {
    pub references: Vec<ContextReference>,
    pub summary: ContextBudgetSummary,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextWorkbench {
    budget: ContextBudget,
    references: Vec<ContextReference>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstructionDiscoveryOptions {
    pub workspace_root: PathBuf,
    pub target_dir: PathBuf,
    pub per_source_byte_limit: usize,
    pub combined_byte_limit: usize,
    pub placement: ContextPlacement,
}

impl InstructionDiscoveryOptions {
    pub fn new(workspace_root: impl Into<PathBuf>, target_dir: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            target_dir: target_dir.into(),
            per_source_byte_limit: 64 * 1024,
            combined_byte_limit: 192 * 1024,
            placement: ContextPlacement::StablePrefix,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedInstruction {
    pub source: InstructionSource,
    pub text: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LoadedInstructionSet {
    pub sources: Vec<InstructionSource>,
    pub loaded: Vec<LoadedInstruction>,
    pub context_references: Vec<ContextReference>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InstructionDiscoveryPlanner;

impl ContextWorkbench {
    pub fn new(budget: ContextBudget) -> Self {
        Self {
            budget,
            references: Vec::new(),
        }
    }

    pub fn from_references<I>(budget: ContextBudget, references: I) -> Self
    where
        I: IntoIterator<Item = ContextReference>,
    {
        Self {
            budget,
            references: references.into_iter().collect(),
        }
    }

    pub fn add_reference(&mut self, reference: ContextReference) {
        if let Some(index) = self
            .references
            .iter()
            .position(|existing| existing.id == reference.id)
        {
            self.references[index] = reference;
            return;
        }

        self.references.push(reference);
    }

    pub fn remove_reference(&mut self, context_id: &ContextId) -> Option<ContextReference> {
        let index = self
            .references
            .iter()
            .position(|reference| &reference.id == context_id)?;
        Some(self.references.remove(index))
    }

    pub fn list_references(&self) -> &[ContextReference] {
        &self.references
    }

    pub fn projection(&self) -> ContextProjection {
        ContextProjection {
            references: self.references.clone(),
            summary: self.summary(),
        }
    }

    pub fn summary(&self) -> ContextBudgetSummary {
        let mut stable_prefix_tokens = 0_u64;
        let mut append_only_transcript_tokens = 0_u64;
        let mut volatile_scratch_tokens = 0_u64;

        for reference in &self.references {
            match reference.placement {
                ContextPlacement::StablePrefix => {
                    stable_prefix_tokens =
                        stable_prefix_tokens.saturating_add(reference.estimated_tokens);
                }
                ContextPlacement::AppendOnlyTranscript => {
                    append_only_transcript_tokens =
                        append_only_transcript_tokens.saturating_add(reference.estimated_tokens);
                }
                ContextPlacement::VolatileScratch => {
                    volatile_scratch_tokens =
                        volatile_scratch_tokens.saturating_add(reference.estimated_tokens);
                }
            }
        }

        let used_tokens = stable_prefix_tokens
            .saturating_add(append_only_transcript_tokens)
            .saturating_add(volatile_scratch_tokens);
        let available_tokens = self
            .budget
            .max_tokens
            .saturating_sub(self.budget.reserved_output_tokens);
        let remaining_tokens = available_tokens.saturating_sub(used_tokens);

        ContextBudgetSummary {
            max_tokens: self.budget.max_tokens,
            reserved_output_tokens: self.budget.reserved_output_tokens,
            available_tokens,
            used_tokens,
            remaining_tokens,
            stable_prefix_tokens,
            append_only_transcript_tokens,
            volatile_scratch_tokens,
            over_budget: used_tokens > available_tokens,
        }
    }
}

impl SkillDiscoveryOptions {
    pub fn new(workspace_root: impl Into<PathBuf>, target_dir: impl Into<PathBuf>) -> Self {
        let workspace_root = workspace_root.into();
        Self {
            skill_roots: vec![workspace_root.join(".tessera").join("skills")],
            workspace_root,
            target_dir: target_dir.into(),
            entrypoint_name: "SKILL.md".to_string(),
            entrypoint_byte_limit: 64 * 1024,
            reference_byte_limit: 32 * 1024,
            combined_byte_limit: 128 * 1024,
            placement: ContextPlacement::StablePrefix,
        }
    }
}

impl SkillRuntimeOptions {
    pub fn new(
        workspace_root: impl Into<PathBuf>,
        target_dir: impl Into<PathBuf>,
        requests: Vec<SkillActivationRequest>,
    ) -> Self {
        Self {
            discovery: SkillDiscoveryOptions::new(workspace_root, target_dir),
            requests,
            best_effort_references: false,
        }
    }
}

impl LoadedSkillSet {
    pub fn render_system_message(&self) -> String {
        let mut message = "Skills selected explicitly for this Tessera run. Treat them as stable behavior guidance. Do not execute scripts or tools from skill directories.".to_string();

        for loaded in &self.skills {
            message.push_str("\n\n--- Skill: ");
            message.push_str(&loaded.manifest.name);
            message.push_str(" (");
            message.push_str(&loaded.entrypoint.relative_path);
            message.push_str(") ---\n");
            message.push_str(&loaded.text);
            if !loaded.text.ends_with('\n') {
                message.push('\n');
            }

            for reference in &loaded.references {
                message.push_str("\n--- Skill reference: ");
                message.push_str(&loaded.manifest.name);
                message.push(' ');
                message.push_str(&reference.source.relative_path);
                message.push_str(" ---\n");
                message.push_str(&reference.text);
                if !reference.text.ends_with('\n') {
                    message.push('\n');
                }
            }
        }

        message
    }

    fn activations_for_task(&self, task_id: &TaskId) -> Vec<SkillActivation> {
        self.activations
            .iter()
            .cloned()
            .map(|mut activation| {
                activation.task_id = task_id.clone();
                activation
            })
            .collect()
    }
}

impl SkillRuntimePlanner {
    pub fn discover(&self, options: SkillDiscoveryOptions) -> Result<SkillDiscoveryReport> {
        if options.entrypoint_byte_limit == 0 {
            return Err(CoreError::InvalidRequest(
                "entrypoint_byte_limit must be at least 1".to_string(),
            ));
        }
        if options.reference_byte_limit == 0 {
            return Err(CoreError::InvalidRequest(
                "reference_byte_limit must be at least 1".to_string(),
            ));
        }
        if options.combined_byte_limit == 0 {
            return Err(CoreError::InvalidRequest(
                "combined_byte_limit must be at least 1".to_string(),
            ));
        }

        let workspace_root = canonicalize_existing_dir("workspace_root", &options.workspace_root)?;
        let target_dir = canonicalize_existing_dir("target_dir", &options.target_dir)?;
        if !target_dir.starts_with(&workspace_root) {
            return Err(CoreError::InvalidRequest(
                "target_dir must be within workspace_root".to_string(),
            ));
        }

        let mut report = SkillDiscoveryReport::default();
        let mut seen_skill_ids = BTreeMap::<SkillId, String>::new();

        for skill_root in &options.skill_roots {
            let root_metadata = match std::fs::symlink_metadata(skill_root) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    report.warnings.push(format!(
                        "skill_root_missing: {}",
                        path_to_string(skill_root)
                    ));
                    continue;
                }
                Err(error) => {
                    report.warnings.push(format!(
                        "skill_root_read_failed: {}: {}",
                        path_to_string(skill_root),
                        error.kind()
                    ));
                    continue;
                }
            };
            if root_metadata.file_type().is_symlink() {
                report.warnings.push(format!(
                    "skill_root_symlink_skipped: {}",
                    path_to_string(skill_root)
                ));
                continue;
            }
            if !root_metadata.is_dir() {
                report.warnings.push(format!(
                    "skill_root_not_directory: {}",
                    path_to_string(skill_root)
                ));
                continue;
            }

            let skill_root = std::fs::canonicalize(skill_root).map_err(|error| {
                CoreError::InvalidRequest(format!("failed to resolve skill_root: {error}"))
            })?;
            if !skill_root.starts_with(&workspace_root) {
                return Err(CoreError::InvalidRequest(
                    "skill_root must be within workspace_root".to_string(),
                ));
            }

            let mut skill_dirs = std::fs::read_dir(&skill_root)
                .map_err(|error| {
                    CoreError::InvalidRequest(format!("failed to read skill_root: {error}"))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| {
                    CoreError::InvalidRequest(format!("failed to read skill_root entry: {error}"))
                })?;
            skill_dirs.sort_by_key(|entry| path_to_string(&entry.path()));

            for entry in skill_dirs {
                let skill_dir = entry.path();
                let metadata = match std::fs::symlink_metadata(&skill_dir) {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        report.warnings.push(format!(
                            "skill_dir_read_failed: {}: {}",
                            path_to_string(&skill_dir),
                            error.kind()
                        ));
                        continue;
                    }
                };
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    continue;
                }

                let entrypoint = skill_dir.join(&options.entrypoint_name);
                if !skill_candidate_exists(&entrypoint) {
                    continue;
                }
                let mut evaluation = inspect_skill_entrypoint(
                    &workspace_root,
                    &entrypoint,
                    &options.entrypoint_name,
                    options.entrypoint_byte_limit,
                );

                if let Some(manifest) = &evaluation.manifest {
                    if let Some(first_source) = seen_skill_ids.get(&manifest.id) {
                        evaluation.source.status = SkillLoadStatus::SkippedDuplicate;
                        evaluation.source.loaded_bytes = 0;
                        evaluation.source.warnings.push(format!(
                            "skipped_duplicate_skill_id: first_source={first_source}"
                        ));
                        evaluation.manifest = None;
                    } else {
                        seen_skill_ids
                            .insert(manifest.id.clone(), evaluation.source.relative_path.clone());
                    }
                }

                report.warnings.extend(evaluation.source.warnings.clone());
                if let Some(manifest) = evaluation.manifest {
                    report.manifests.push(manifest);
                }
                report.sources.push(evaluation.source);
            }
        }

        Ok(report)
    }

    pub fn activate(&self, options: SkillRuntimeOptions) -> Result<LoadedSkillSet> {
        if options.requests.is_empty() {
            return Ok(LoadedSkillSet::default());
        }

        let workspace_root =
            canonicalize_existing_dir("workspace_root", &options.discovery.workspace_root)?;
        let target_dir = canonicalize_existing_dir("target_dir", &options.discovery.target_dir)?;
        if !target_dir.starts_with(&workspace_root) {
            return Err(CoreError::InvalidRequest(
                "target_dir must be within workspace_root".to_string(),
            ));
        }

        let discovery = self.discover(options.discovery.clone())?;
        let mut loaded_set = LoadedSkillSet {
            warnings: discovery.warnings.clone(),
            ..LoadedSkillSet::default()
        };
        let mut remaining_budget = options.discovery.combined_byte_limit;

        for request in options.requests {
            let (manifest, discovered_source) =
                resolve_skill_request(&discovery, &request.skill).ok_or_else(|| {
                    CoreError::InvalidRequest(format!("requested skill not found: {}", request.skill))
                })?;
            let entrypoint_path = workspace_root.join(&discovered_source.relative_path);
            let skill_dir = entrypoint_path.parent().ok_or_else(|| {
                CoreError::InvalidRequest("skill entrypoint must have a parent directory".to_string())
            })?;

            let mut steps = Vec::new();
            steps.push(SkillActivationStep {
                step_index: steps.len() as u32,
                kind: SkillStepKind::DiscoverEntrypoint,
                status: SkillStepStatus::Completed,
                source_id: Some(discovered_source.source_id.clone()),
                warnings: Vec::new(),
            });

            let loaded_entrypoint = load_skill_text_source(
                &workspace_root,
                &entrypoint_path,
                options.discovery.entrypoint_byte_limit,
                &mut remaining_budget,
                true,
            )?;
            steps.push(SkillActivationStep {
                step_index: steps.len() as u32,
                kind: SkillStepKind::LoadEntrypoint,
                status: SkillStepStatus::Completed,
                source_id: Some(loaded_entrypoint.source.source_id.clone()),
                warnings: loaded_entrypoint.source.warnings.clone(),
            });

            let mut references = Vec::new();
            for reference in request.references {
                match load_skill_reference(
                    &workspace_root,
                    skill_dir,
                    &reference,
                    options.discovery.reference_byte_limit,
                    &mut remaining_budget,
                ) {
                    Ok(loaded_reference) => {
                        steps.push(SkillActivationStep {
                            step_index: steps.len() as u32,
                            kind: SkillStepKind::LoadReference,
                            status: SkillStepStatus::Completed,
                            source_id: Some(loaded_reference.source.source_id.clone()),
                            warnings: loaded_reference.source.warnings.clone(),
                        });
                        references.push(loaded_reference);
                    }
                    Err(error) if options.best_effort_references => {
                        let warning = format!("reference_load_failed: {error}");
                        loaded_set.warnings.push(warning.clone());
                        steps.push(SkillActivationStep {
                            step_index: steps.len() as u32,
                            kind: SkillStepKind::LoadReference,
                            status: SkillStepStatus::Failed,
                            source_id: None,
                            warnings: vec![warning],
                        });
                    }
                    Err(error) => return Err(error),
                }
            }

            steps.push(SkillActivationStep {
                step_index: steps.len() as u32,
                kind: SkillStepKind::RenderContext,
                status: SkillStepStatus::Completed,
                source_id: None,
                warnings: Vec::new(),
            });

            let activation = SkillActivation {
                task_id: TaskId::from_static("task_skill_activation_pending"),
                skill_id: manifest.id.clone(),
                manifest: manifest.clone(),
                status: SkillActivationStatus::Activated,
                entrypoint: loaded_entrypoint.source.clone(),
                references: references
                    .iter()
                    .map(|reference| reference.source.clone())
                    .collect(),
                steps,
                warnings: Vec::new(),
            };
            let loaded_skill = LoadedSkill {
                manifest,
                entrypoint: loaded_entrypoint.source.clone(),
                text: loaded_entrypoint.text,
                references,
                activation: activation.clone(),
            };
            loaded_set.warnings.extend(activation.entrypoint.warnings.clone());
            for reference in &loaded_skill.references {
                loaded_set.warnings.extend(reference.source.warnings.clone());
            }
            loaded_set.activations.push(activation);
            loaded_set.skills.push(loaded_skill);
        }

        Ok(loaded_set)
    }
}

struct SkillEntrypointEvaluation {
    source: SkillReferenceSource,
    manifest: Option<SkillManifest>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ParsedSkillMd {
    name: String,
    description: String,
    version: Option<String>,
    body: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LoadedSkillTextSource {
    source: SkillReferenceSource,
    text: String,
}

fn resolve_skill_request(
    discovery: &SkillDiscoveryReport,
    requested: &str,
) -> Option<(SkillManifest, SkillReferenceSource)> {
    let by_id = discovery
        .manifests
        .iter()
        .find(|manifest| manifest.id.as_str() == requested);
    let manifest = by_id.or_else(|| {
        discovery
            .manifests
            .iter()
            .find(|manifest| manifest.name == requested)
    })?;
    let uri = manifest.source.uri.as_deref()?;
    let source = discovery
        .sources
        .iter()
        .find(|source| source.relative_path == uri && source.status == SkillLoadStatus::Loaded)?;
    Some((manifest.clone(), source.clone()))
}

fn load_skill_text_source(
    workspace_root: &Path,
    path: &Path,
    byte_limit: usize,
    remaining_budget: &mut usize,
    use_skill_body: bool,
) -> Result<LoadedSkillTextSource> {
    if *remaining_budget == 0 {
        return Err(CoreError::InvalidRequest(
            "skill context byte budget exhausted".to_string(),
        ));
    }

    let mut source = base_skill_source(workspace_root, path);
    if !path.starts_with(workspace_root) {
        source.status = SkillLoadStatus::SkippedOutsideWorkspace;
        return Err(CoreError::InvalidRequest(
            "skill source must be within workspace_root".to_string(),
        ));
    }

    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        CoreError::InvalidRequest(format!("failed to read skill source metadata: {error}"))
    })?;
    source.original_bytes = metadata.len();
    if metadata.file_type().is_symlink() {
        source.status = SkillLoadStatus::SkippedSymlink;
        return Err(CoreError::InvalidRequest(
            "skill source symlinks are not allowed".to_string(),
        ));
    }
    if !metadata.is_file() {
        source.status = SkillLoadStatus::ReadFailed;
        return Err(CoreError::InvalidRequest(
            "skill source must be a regular file".to_string(),
        ));
    }
    if metadata_len_exceeds(metadata.len(), byte_limit) {
        source.status = SkillLoadStatus::SkippedTooLarge;
        return Err(CoreError::InvalidRequest(
            "skill source exceeds byte limit".to_string(),
        ));
    }

    let bytes = std::fs::read(path)
        .map_err(|error| CoreError::InvalidRequest(format!("failed to read skill source: {error}")))?;
    source.original_bytes = bytes.len() as u64;
    source.sha256 = Some(sha256_hex(&bytes));

    let text = String::from_utf8(bytes).map_err(|_| {
        source.status = SkillLoadStatus::SkippedNonUtf8;
        CoreError::InvalidRequest("skill source must be UTF-8".to_string())
    })?;
    let text = if use_skill_body {
        parse_skill_md(&text)
            .map_err(|reason| CoreError::InvalidRequest(format!("invalid skill manifest: {reason}")))?
            .body
    } else {
        text
    };

    let (redacted, redaction_status, mut warnings) = redact_skill_text(&text);
    let loaded_text = truncate_to_utf8_boundary(&redacted, *remaining_budget);
    if loaded_text.len() < redacted.len() {
        warnings.push("truncated_to_combined_byte_limit".to_string());
    }

    *remaining_budget = remaining_budget.saturating_sub(loaded_text.len());
    source.status = SkillLoadStatus::Loaded;
    source.loaded_bytes = loaded_text.len() as u64;
    source.redaction_status = redaction_status;
    source.warnings.append(&mut warnings);

    Ok(LoadedSkillTextSource {
        source,
        text: loaded_text,
    })
}

fn load_skill_reference(
    workspace_root: &Path,
    skill_dir: &Path,
    reference: &str,
    byte_limit: usize,
    remaining_budget: &mut usize,
) -> Result<LoadedSkillReference> {
    let reference_path = Path::new(reference);
    if reference_path.as_os_str().is_empty() || reference_path.is_absolute() {
        return Err(CoreError::InvalidRequest(
            "skill reference must be a relative path".to_string(),
        ));
    }
    if reference_path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(CoreError::InvalidRequest(
            "skill reference must stay inside the skill directory".to_string(),
        ));
    }
    if reference_path
        .components()
        .next()
        .is_some_and(|component| component.as_os_str() == "scripts")
    {
        return Err(CoreError::InvalidRequest(
            "skill references under scripts/ are not loadable in v0.5".to_string(),
        ));
    }

    let canonical_skill_dir = std::fs::canonicalize(skill_dir)
        .map_err(|error| CoreError::InvalidRequest(format!("failed to resolve skill dir: {error}")))?;
    let path = canonical_skill_dir.join(reference_path);
    let metadata = std::fs::symlink_metadata(&path).map_err(|error| {
        CoreError::InvalidRequest(format!("failed to read skill reference metadata: {error}"))
    })?;
    if metadata.file_type().is_symlink() {
        return Err(CoreError::InvalidRequest(
            "skill reference symlinks are not allowed".to_string(),
        ));
    }
    let canonical_path = std::fs::canonicalize(&path).map_err(|error| {
        CoreError::InvalidRequest(format!("failed to resolve skill reference: {error}"))
    })?;
    if !canonical_path.starts_with(&canonical_skill_dir) {
        return Err(CoreError::InvalidRequest(
            "skill reference must stay inside the skill directory".to_string(),
        ));
    }

    let loaded = load_skill_text_source(
        workspace_root,
        &canonical_path,
        byte_limit,
        remaining_budget,
        false,
    )?;
    Ok(LoadedSkillReference {
        source: loaded.source,
        text: loaded.text,
    })
}

fn redact_skill_text(text: &str) -> (String, SkillRedactionStatus, Vec<String>) {
    let (redacted, status, warnings) = redact_instruction_text(text);
    let status = match status {
        InstructionRedactionStatus::Clean => SkillRedactionStatus::Clean,
        InstructionRedactionStatus::Redacted => SkillRedactionStatus::Redacted,
    };
    (redacted, status, warnings)
}

fn skill_candidate_exists(path: &Path) -> bool {
    match std::fs::symlink_metadata(path) {
        Ok(_) => true,
        Err(error) => error.kind() != std::io::ErrorKind::NotFound,
    }
}

fn inspect_skill_entrypoint(
    workspace_root: &Path,
    path: &Path,
    entrypoint_name: &str,
    entrypoint_byte_limit: usize,
) -> SkillEntrypointEvaluation {
    let mut source = base_skill_source(workspace_root, path);

    if !path.starts_with(workspace_root) {
        source.status = SkillLoadStatus::SkippedOutsideWorkspace;
        source
            .warnings
            .push("skipped_outside_workspace".to_string());
        return SkillEntrypointEvaluation {
            source,
            manifest: None,
        };
    }

    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            source.status = SkillLoadStatus::ReadFailed;
            source
                .warnings
                .push(format!("read_failed: {}", error.kind()));
            return SkillEntrypointEvaluation {
                source,
                manifest: None,
            };
        }
    };
    source.original_bytes = metadata.len();

    if metadata.file_type().is_symlink() {
        source.status = SkillLoadStatus::SkippedSymlink;
        source.warnings.push("skipped_symlink_source".to_string());
        return SkillEntrypointEvaluation {
            source,
            manifest: None,
        };
    }
    if !metadata.is_file() {
        source.status = SkillLoadStatus::ReadFailed;
        source.warnings.push("not_a_regular_file".to_string());
        return SkillEntrypointEvaluation {
            source,
            manifest: None,
        };
    }
    if metadata_len_exceeds(metadata.len(), entrypoint_byte_limit) {
        source.status = SkillLoadStatus::SkippedTooLarge;
        source.warnings.push("skipped_too_large_source".to_string());
        return SkillEntrypointEvaluation {
            source,
            manifest: None,
        };
    }

    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            source.status = SkillLoadStatus::ReadFailed;
            source
                .warnings
                .push(format!("read_failed: {}", error.kind()));
            return SkillEntrypointEvaluation {
                source,
                manifest: None,
            };
        }
    };
    source.original_bytes = bytes.len() as u64;
    source.sha256 = Some(sha256_hex(&bytes));

    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => {
            source.status = SkillLoadStatus::SkippedNonUtf8;
            source.warnings.push("skipped_non_utf8_source".to_string());
            return SkillEntrypointEvaluation {
                source,
                manifest: None,
            };
        }
    };

    let parsed = match parse_skill_md(&text) {
        Ok(parsed) => parsed,
        Err(reason) => {
            source.status = SkillLoadStatus::InvalidManifest;
            source.warnings.push(format!("invalid_manifest: {reason}"));
            return SkillEntrypointEvaluation {
                source,
                manifest: None,
            };
        }
    };

    let skill_id = skill_id_from_name(&parsed.name);
    let manifest = SkillManifest {
        id: skill_id,
        name: parsed.name,
        version: parsed.version,
        description: parsed.description,
        source: SkillSource {
            kind: SkillSourceKind::Workspace,
            uri: Some(source.relative_path.clone()),
        },
        entrypoint: SkillEntrypoint {
            format: SkillEntrypointFormat::SkillMd,
            path: entrypoint_name.to_string(),
        },
        requirements: SkillRequirements::default(),
        policy: SkillPolicy {
            default_permission: "ask".to_string(),
            network: "deny".to_string(),
            write_files: "deny".to_string(),
        },
        metadata: None,
    };

    source.status = SkillLoadStatus::Loaded;
    source.loaded_bytes = 0;
    source.redaction_status = SkillRedactionStatus::Clean;

    SkillEntrypointEvaluation {
        source,
        manifest: Some(manifest),
    }
}

fn base_skill_source(workspace_root: &Path, path: &Path) -> SkillReferenceSource {
    let relative_path = relative_instruction_path(workspace_root, path);
    SkillReferenceSource {
        source_id: ContextId::from(format!(
            "context_skill_{}",
            sanitize_mcp_id_fragment(&relative_path)
        )),
        path: path_to_string(path),
        relative_path,
        status: SkillLoadStatus::ReadFailed,
        original_bytes: 0,
        loaded_bytes: 0,
        sha256: None,
        redaction_status: SkillRedactionStatus::Clean,
        warnings: Vec::new(),
    }
}

fn parse_skill_md(text: &str) -> std::result::Result<ParsedSkillMd, String> {
    let mut lines = text.split_inclusive('\n');
    let first = lines
        .next()
        .ok_or_else(|| "missing_frontmatter".to_string())?;
    if trim_line_ending(first).trim() != "---" {
        return Err("missing_frontmatter_start".to_string());
    }

    let mut frontmatter = Vec::new();
    let mut body = String::new();
    let mut found_end = false;
    for line in lines {
        if !found_end && trim_line_ending(line).trim() == "---" {
            found_end = true;
            continue;
        }

        if found_end {
            body.push_str(line);
        } else {
            frontmatter.push(line.to_string());
        }
    }
    if !found_end {
        return Err("missing_frontmatter_end".to_string());
    }

    let mut name = None;
    let mut description = None;
    let mut version = None;

    for line in frontmatter {
        let line = trim_line_ending(&line).trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = trim_frontmatter_value(value);
        match key {
            "name" => name = Some(value),
            "description" => description = Some(value),
            "version" => version = Some(value),
            _ => {}
        }
    }

    let name = name
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "missing_name".to_string())?;
    let description = description
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "missing_description".to_string())?;

    Ok(ParsedSkillMd {
        name,
        description,
        version: version.filter(|value| !value.is_empty()),
        body,
    })
}

fn trim_line_ending(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

fn trim_frontmatter_value(value: &str) -> String {
    let value = value.trim();
    value
        .strip_prefix('"')
        .and_then(|stripped| stripped.strip_suffix('"'))
        .unwrap_or(value)
        .trim()
        .to_string()
}

fn skill_id_from_name(name: &str) -> SkillId {
    let fragment = sanitize_mcp_id_fragment(name);
    if fragment.starts_with("skill_") {
        SkillId::from(fragment)
    } else {
        SkillId::from(format!("skill_{fragment}"))
    }
}

impl InstructionDiscoveryPlanner {
    pub fn discover(&self, options: InstructionDiscoveryOptions) -> Result<LoadedInstructionSet> {
        if options.per_source_byte_limit == 0 {
            return Err(CoreError::InvalidRequest(
                "per_source_byte_limit must be at least 1".to_string(),
            ));
        }
        if options.combined_byte_limit == 0 {
            return Err(CoreError::InvalidRequest(
                "combined_byte_limit must be at least 1".to_string(),
            ));
        }

        let workspace_root = canonicalize_existing_dir("workspace_root", &options.workspace_root)?;
        let target_dir = canonicalize_existing_dir("target_dir", &options.target_dir)?;
        if !target_dir.starts_with(&workspace_root) {
            return Err(CoreError::InvalidRequest(
                "target_dir must be within workspace_root".to_string(),
            ));
        }

        let mut remaining_budget = options.combined_byte_limit;
        let mut set = LoadedInstructionSet::default();
        let mut precedence = 0_u32;

        for directory in instruction_search_directories(&workspace_root, &target_dir)? {
            let agents_path = directory.join("AGENTS.md");
            let claude_path = directory.join("CLAUDE.md");

            if instruction_candidate_exists(&agents_path) {
                let evaluated = inspect_instruction_candidate(
                    &workspace_root,
                    &agents_path,
                    InstructionSourceKind::AgentsMd,
                    precedence,
                    options.placement,
                    remaining_budget,
                    options.per_source_byte_limit,
                );
                push_instruction_evaluation(&mut set, evaluated, &mut remaining_budget);

                if instruction_candidate_exists(&claude_path) {
                    let source = skipped_lower_precedence_instruction_source(
                        &workspace_root,
                        &claude_path,
                        InstructionSourceKind::ClaudeMd,
                        precedence,
                        options.placement,
                    );
                    set.warnings.extend(source.warnings.clone());
                    set.sources.push(source);
                }
                precedence = precedence.saturating_add(1);
            } else if instruction_candidate_exists(&claude_path) {
                let evaluated = inspect_instruction_candidate(
                    &workspace_root,
                    &claude_path,
                    InstructionSourceKind::ClaudeMd,
                    precedence,
                    options.placement,
                    remaining_budget,
                    options.per_source_byte_limit,
                );
                push_instruction_evaluation(&mut set, evaluated, &mut remaining_budget);
                precedence = precedence.saturating_add(1);
            }
        }

        Ok(set)
    }
}

struct InstructionCandidateEvaluation {
    source: InstructionSource,
    loaded_text: Option<String>,
}

fn push_instruction_evaluation(
    set: &mut LoadedInstructionSet,
    evaluation: InstructionCandidateEvaluation,
    remaining_budget: &mut usize,
) {
    let source = evaluation.source;
    set.warnings.extend(source.warnings.clone());

    if let Some(text) = evaluation.loaded_text {
        *remaining_budget = remaining_budget.saturating_sub(source.loaded_bytes as usize);
        set.context_references
            .push(context_reference_for_instruction(&source, &text));
        set.loaded.push(LoadedInstruction {
            source: source.clone(),
            text,
        });
    }

    set.sources.push(source);
}

fn canonicalize_existing_dir(label: &str, path: &Path) -> Result<PathBuf> {
    let canonical = std::fs::canonicalize(path).map_err(|error| {
        CoreError::InvalidRequest(format!("failed to resolve {label}: {error}"))
    })?;
    if !canonical.is_dir() {
        return Err(CoreError::InvalidRequest(format!(
            "{label} must be an existing directory"
        )));
    }
    Ok(canonical)
}

fn instruction_search_directories(
    workspace_root: &Path,
    target_dir: &Path,
) -> Result<Vec<PathBuf>> {
    let mut directories = Vec::new();
    let mut current = target_dir.to_path_buf();

    loop {
        directories.push(current.clone());
        if current == workspace_root {
            break;
        }
        current = current.parent().map(Path::to_path_buf).ok_or_else(|| {
            CoreError::InvalidRequest("target_dir must be within workspace_root".to_string())
        })?;
    }

    directories.reverse();
    Ok(directories)
}

fn instruction_candidate_exists(path: &Path) -> bool {
    match std::fs::symlink_metadata(path) {
        Ok(_) => true,
        Err(error) => error.kind() != std::io::ErrorKind::NotFound,
    }
}

fn inspect_instruction_candidate(
    workspace_root: &Path,
    path: &Path,
    kind: InstructionSourceKind,
    precedence: u32,
    placement: ContextPlacement,
    remaining_budget: usize,
    per_source_byte_limit: usize,
) -> InstructionCandidateEvaluation {
    let mut source = base_instruction_source(workspace_root, path, kind, precedence, placement);

    if !path.starts_with(workspace_root) {
        source.status = InstructionLoadStatus::SkippedOutsideWorkspace;
        source
            .warnings
            .push("skipped_outside_workspace".to_string());
        return InstructionCandidateEvaluation {
            source,
            loaded_text: None,
        };
    }

    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            source.status = InstructionLoadStatus::ReadFailed;
            source
                .warnings
                .push(format!("read_failed: {}", error.kind()));
            return InstructionCandidateEvaluation {
                source,
                loaded_text: None,
            };
        }
    };
    source.original_bytes = metadata.len();

    if metadata.file_type().is_symlink() {
        source.status = InstructionLoadStatus::SkippedSymlink;
        source.warnings.push("skipped_symlink_source".to_string());
        return InstructionCandidateEvaluation {
            source,
            loaded_text: None,
        };
    }
    if !metadata.is_file() {
        source.status = InstructionLoadStatus::ReadFailed;
        source.warnings.push("not_a_regular_file".to_string());
        return InstructionCandidateEvaluation {
            source,
            loaded_text: None,
        };
    }
    if metadata_len_exceeds(metadata.len(), per_source_byte_limit) {
        source.status = InstructionLoadStatus::SkippedTooLarge;
        source.warnings.push("skipped_too_large_source".to_string());
        return InstructionCandidateEvaluation {
            source,
            loaded_text: None,
        };
    }

    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            source.status = InstructionLoadStatus::ReadFailed;
            source
                .warnings
                .push(format!("read_failed: {}", error.kind()));
            return InstructionCandidateEvaluation {
                source,
                loaded_text: None,
            };
        }
    };
    source.original_bytes = bytes.len() as u64;
    source.sha256 = Some(sha256_hex(&bytes));

    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => {
            source.status = InstructionLoadStatus::SkippedNonUtf8;
            source.warnings.push("skipped_non_utf8_source".to_string());
            return InstructionCandidateEvaluation {
                source,
                loaded_text: None,
            };
        }
    };

    if remaining_budget == 0 {
        source.status = InstructionLoadStatus::SkippedTooLarge;
        source
            .warnings
            .push("skipped_combined_byte_limit_exhausted".to_string());
        return InstructionCandidateEvaluation {
            source,
            loaded_text: None,
        };
    }

    let (redacted, redaction_status, mut redaction_warnings) = redact_instruction_text(&text);
    let loaded_text = truncate_to_utf8_boundary(&redacted, remaining_budget);
    if loaded_text.len() < redacted.len() {
        source
            .warnings
            .push("truncated_to_combined_byte_limit".to_string());
    }
    source.warnings.append(&mut redaction_warnings);
    source.status = InstructionLoadStatus::Loaded;
    source.redaction_status = redaction_status;
    source.loaded_bytes = loaded_text.len() as u64;

    InstructionCandidateEvaluation {
        source,
        loaded_text: Some(loaded_text),
    }
}

fn base_instruction_source(
    workspace_root: &Path,
    path: &Path,
    kind: InstructionSourceKind,
    precedence: u32,
    placement: ContextPlacement,
) -> InstructionSource {
    let relative_path = relative_instruction_path(workspace_root, path);
    InstructionSource {
        source_id: ContextId::from(format!(
            "context_instruction_{}",
            sanitize_mcp_id_fragment(&relative_path)
        )),
        kind,
        path: path_to_string(path),
        relative_path,
        precedence,
        placement,
        status: InstructionLoadStatus::ReadFailed,
        original_bytes: 0,
        loaded_bytes: 0,
        sha256: None,
        redaction_status: InstructionRedactionStatus::Clean,
        warnings: Vec::new(),
    }
}

fn skipped_lower_precedence_instruction_source(
    workspace_root: &Path,
    path: &Path,
    kind: InstructionSourceKind,
    precedence: u32,
    placement: ContextPlacement,
) -> InstructionSource {
    let mut source = base_instruction_source(workspace_root, path, kind, precedence, placement);
    source.status = InstructionLoadStatus::SkippedLowerPrecedence;
    source.original_bytes = std::fs::symlink_metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    source
        .warnings
        .push("skipped_lower_precedence_source".to_string());
    source
}

fn relative_instruction_path(workspace_root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(workspace_root).unwrap_or(path);
    path_to_string(relative).replace('\\', "/")
}

fn metadata_len_exceeds(metadata_len: u64, byte_limit: usize) -> bool {
    usize::try_from(metadata_len).map_or(true, |len| len > byte_limit)
}

fn redact_instruction_text(text: &str) -> (String, InstructionRedactionStatus, Vec<String>) {
    let mut redacted = String::new();
    let mut changed = false;

    for segment in text.split_inclusive('\n') {
        let line_without_newline = segment.strip_suffix('\n').unwrap_or(segment);
        if instruction_line_may_contain_secret(line_without_newline) {
            redacted.push_str("[REDACTED: possible secret]");
            if segment.ends_with('\n') {
                redacted.push('\n');
            }
            changed = true;
        } else {
            redacted.push_str(segment);
        }
    }

    if !text.ends_with('\n') && !text.is_empty() && text.rsplit('\n').next().is_some() {
        let last_line = text.rsplit('\n').next().unwrap();
        if !text.contains('\n') && instruction_line_may_contain_secret(last_line) {
            redacted = "[REDACTED: possible secret]".to_string();
            changed = true;
        }
    }

    if changed {
        (
            redacted,
            InstructionRedactionStatus::Redacted,
            vec!["redacted_possible_secret_line".to_string()],
        )
    } else {
        (redacted, InstructionRedactionStatus::Clean, Vec::new())
    }
}

fn instruction_line_may_contain_secret(line: &str) -> bool {
    let normalized = line.to_ascii_lowercase();
    [
        "api_key",
        "apikey",
        "authorization",
        "bearer",
        "cookie",
        ".env",
        "password",
        "secret",
        "token",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn truncate_to_utf8_boundary(text: &str, byte_limit: usize) -> String {
    if text.len() <= byte_limit {
        return text.to_string();
    }

    let mut end = byte_limit;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn context_reference_for_instruction(source: &InstructionSource, text: &str) -> ContextReference {
    ContextReference {
        id: source.source_id.clone(),
        source: ContextSource {
            kind: ContextSourceKind::File,
            uri: Some(source.relative_path.clone()),
            label: Some(format!("project instructions: {}", source.relative_path)),
        },
        placement: source.placement,
        estimated_tokens: estimate_instruction_tokens(text),
        pinned: true,
        summary: Some(format!(
            "Project instructions loaded from {}",
            source.relative_path
        )),
        metadata: None,
    }
}

fn estimate_instruction_tokens(text: &str) -> u64 {
    if text.is_empty() {
        0
    } else {
        ((text.len() as u64).saturating_add(3) / 4).max(1)
    }
}

fn render_instruction_system_message(instruction_context: &LoadedInstructionSet) -> String {
    let mut message =
        "Project instructions loaded by Tessera for this run. Follow them as stable workspace context."
            .to_string();

    for loaded in &instruction_context.loaded {
        message.push_str("\n\n### ");
        message.push_str(&loaded.source.relative_path);
        message.push('\n');
        message.push_str(&loaded.text);
        if !loaded.text.ends_with('\n') {
            message.push('\n');
        }
    }

    message
}

impl SkillRegistry {
    pub fn from_manifests<I>(manifests: I) -> Self
    where
        I: IntoIterator<Item = SkillManifest>,
    {
        Self {
            manifests: manifests.into_iter().collect(),
        }
    }

    pub fn list_skills(&self) -> Vec<SkillManifest> {
        self.manifests.clone()
    }

    pub fn find_skill(&self, skill_id: &SkillId) -> Option<&SkillManifest> {
        self.manifests
            .iter()
            .find(|manifest| &manifest.id == skill_id)
    }
}

impl AgentRegistry {
    pub fn from_profiles<I>(profiles: I) -> Self
    where
        I: IntoIterator<Item = AgentProfile>,
    {
        Self {
            profiles: profiles.into_iter().collect(),
        }
    }

    pub fn list_agents(&self) -> Vec<AgentProfile> {
        self.profiles.clone()
    }

    pub fn find_agent(&self, profile_id: &AgentProfileId) -> Option<&AgentProfile> {
        self.profiles
            .iter()
            .find(|profile| &profile.id == profile_id)
    }
}

impl ToolRegistry {
    pub fn from_descriptors<I>(descriptors: I) -> Self
    where
        I: IntoIterator<Item = ToolDescriptor>,
    {
        Self {
            descriptors: descriptors.into_iter().collect(),
        }
    }

    pub fn list_tools(&self) -> Vec<ToolDescriptor> {
        self.descriptors.clone()
    }

    pub fn find_tool(&self, tool_id: &ToolId) -> Option<&ToolDescriptor> {
        self.descriptors
            .iter()
            .find(|descriptor| &descriptor.id == tool_id)
    }
}

impl McpToolAdapter {
    pub fn descriptor_from_spec(&self, spec: &McpToolSpec) -> ToolDescriptor {
        ToolDescriptor {
            id: ToolId::from(format!(
                "tool_mcp_{}_{}",
                sanitize_mcp_id_fragment(&spec.server_id),
                sanitize_mcp_id_fragment(&spec.name)
            )),
            display_name: spec
                .annotations
                .title
                .clone()
                .unwrap_or_else(|| spec.name.clone()),
            description: spec
                .description
                .clone()
                .unwrap_or_else(|| format!("MCP tool {}", spec.name)),
            input_schema: spec.input_schema.clone(),
            output_schema: spec
                .output_schema
                .clone()
                .unwrap_or_else(|| serde_json::json!({ "type": "object" })),
            required_permissions: mcp_required_permissions(&spec.annotations),
            side_effects: mcp_side_effects(&spec.annotations),
            parallel_safe: false,
            metadata: Some(mcp_tool_metadata(spec)),
        }
    }

    pub fn request_from_arguments(
        &self,
        descriptor: &ToolDescriptor,
        arguments: serde_json::Value,
    ) -> ToolCallRequest {
        let mut metadata = ExtensionMap::new();
        if let Some(descriptor_metadata) = &descriptor.metadata {
            for key in [
                "mcp_server_id",
                "mcp_tool_name",
                "mcp_read_only_hint",
                "mcp_destructive_hint",
                "mcp_idempotent_hint",
                "mcp_open_world_hint",
            ] {
                if let Some(value) = descriptor_metadata.get(key) {
                    metadata.insert(key.to_string(), value.clone());
                }
            }
        }
        metadata.insert(
            "mcp_adapter".to_string(),
            serde_json::Value::String("metadata_only".to_string()),
        );

        ToolCallRequest {
            call_id: tessera_protocol::ToolCallId::new(),
            tool_id: descriptor.id.clone(),
            input: arguments,
            metadata: Some(metadata),
        }
    }
}

impl DiagnosticsReporter {
    pub fn report<I>(&self, source: impl Into<String>, diagnostics: I) -> DiagnosticReport
    where
        I: IntoIterator<Item = Diagnostic>,
    {
        DiagnosticReport {
            report_id: DiagnosticReportId::new(),
            source: source.into(),
            diagnostics: diagnostics.into_iter().collect(),
            metadata: None,
        }
    }

    pub fn report_event(&self, report: DiagnosticReport) -> RunEvent {
        RunEvent::DiagnosticsReported { report }
    }
}

fn mcp_tool_metadata(spec: &McpToolSpec) -> ExtensionMap {
    let mut metadata = ExtensionMap::new();
    metadata.insert(
        "mcp_server_id".to_string(),
        serde_json::Value::String(spec.server_id.clone()),
    );
    metadata.insert(
        "mcp_tool_name".to_string(),
        serde_json::Value::String(spec.name.clone()),
    );
    insert_optional_bool(
        &mut metadata,
        "mcp_read_only_hint",
        spec.annotations.read_only_hint,
    );
    insert_optional_bool(
        &mut metadata,
        "mcp_destructive_hint",
        spec.annotations.destructive_hint,
    );
    insert_optional_bool(
        &mut metadata,
        "mcp_idempotent_hint",
        spec.annotations.idempotent_hint,
    );
    insert_optional_bool(
        &mut metadata,
        "mcp_open_world_hint",
        spec.annotations.open_world_hint,
    );
    metadata
}

fn insert_optional_bool(metadata: &mut ExtensionMap, key: &str, value: Option<bool>) {
    if let Some(value) = value {
        metadata.insert(key.to_string(), serde_json::Value::Bool(value));
    }
}

fn mcp_required_permissions(annotations: &McpToolAnnotations) -> Vec<ToolPermission> {
    if annotations.open_world_hint.unwrap_or(true) {
        vec![ToolPermission::Network]
    } else {
        Vec::new()
    }
}

fn mcp_side_effects(annotations: &McpToolAnnotations) -> Vec<ToolSideEffect> {
    if annotations.open_world_hint.unwrap_or(true) {
        return vec![ToolSideEffect::Network];
    }

    if annotations.read_only_hint == Some(true) && annotations.destructive_hint != Some(true) {
        vec![ToolSideEffect::ReadOnly]
    } else {
        vec![ToolSideEffect::PersistentState]
    }
}

fn sanitize_mcp_id_fragment(value: &str) -> String {
    let mut sanitized = String::new();
    let mut previous_was_separator = false;

    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            sanitized.push(character.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !sanitized.is_empty() && !previous_was_separator {
            sanitized.push('_');
            previous_was_separator = true;
        }
    }

    while sanitized.ends_with('_') {
        sanitized.pop();
    }

    if sanitized.is_empty() {
        "unknown".to_string()
    } else {
        sanitized
    }
}

impl OrderedToolResultBuffer {
    pub fn from_dispatches<I>(dispatches: I) -> Self
    where
        I: IntoIterator<Item = ToolDispatch>,
    {
        let mut dispatches: Vec<ToolDispatch> = dispatches.into_iter().collect();
        dispatches.sort_by_key(|dispatch| dispatch.declared_index);
        let order = dispatches
            .iter()
            .map(|dispatch| dispatch.declared_index)
            .collect();

        Self {
            dispatches,
            order,
            next_cursor: 0,
            pending_results: BTreeMap::new(),
        }
    }

    pub fn start_events(&self) -> Vec<RunEvent> {
        self.dispatches
            .iter()
            .cloned()
            .map(|dispatch| RunEvent::ToolDispatchStarted { dispatch })
            .collect()
    }

    pub fn record_completion(&mut self, result: ToolResult) -> Vec<RunEvent> {
        self.pending_results.insert(result.declared_index, result);
        let mut released = Vec::new();

        while let Some(next_index) = self.order.get(self.next_cursor).copied() {
            let Some(result) = self.pending_results.remove(&next_index) else {
                break;
            };

            released.push(RunEvent::ToolDispatchCompleted {
                result: result.clone(),
            });
            released.push(RunEvent::ToolResultRecorded { result });
            self.next_cursor += 1;
        }

        released
    }
}

impl ToolRepairTelemetry {
    pub fn flattened_nested_calls(
        &self,
        call_id: Option<tessera_protocol::ToolCallId>,
        tool_id: Option<ToolId>,
        original_call_count: u32,
        repaired_call_count: u32,
        reason: impl Into<String>,
    ) -> ToolRepairReport {
        tool_repair_report(
            ToolRepairKind::FlattenedNestedCalls,
            call_id,
            tool_id,
            Some(original_call_count),
            Some(repaired_call_count),
            None,
            reason,
        )
    }

    pub fn scavenged_json(
        &self,
        call_id: Option<tessera_protocol::ToolCallId>,
        tool_id: Option<ToolId>,
        original_call_count: u32,
        repaired_call_count: u32,
        reason: impl Into<String>,
    ) -> ToolRepairReport {
        tool_repair_report(
            ToolRepairKind::ScavengedJson,
            call_id,
            tool_id,
            Some(original_call_count),
            Some(repaired_call_count),
            None,
            reason,
        )
    }

    pub fn truncated_arguments(
        &self,
        call_id: Option<tessera_protocol::ToolCallId>,
        tool_id: Option<ToolId>,
        truncated_bytes: u64,
        reason: impl Into<String>,
    ) -> ToolRepairReport {
        tool_repair_report(
            ToolRepairKind::TruncatedArguments,
            call_id,
            tool_id,
            None,
            None,
            Some(truncated_bytes),
            reason,
        )
    }

    pub fn call_storm_detected(
        &self,
        original_call_count: u32,
        repaired_call_count: u32,
        reason: impl Into<String>,
    ) -> ToolRepairReport {
        tool_repair_report(
            ToolRepairKind::CallStormDetected,
            None,
            None,
            Some(original_call_count),
            Some(repaired_call_count),
            None,
            reason,
        )
    }
}

fn tool_repair_report(
    kind: ToolRepairKind,
    call_id: Option<tessera_protocol::ToolCallId>,
    tool_id: Option<ToolId>,
    original_call_count: Option<u32>,
    repaired_call_count: Option<u32>,
    truncated_bytes: Option<u64>,
    reason: impl Into<String>,
) -> ToolRepairReport {
    ToolRepairReport {
        repair_id: ToolRepairId::new(),
        call_id,
        tool_id,
        kind,
        reason: reason.into(),
        original_call_count,
        repaired_call_count,
        truncated_bytes,
        metadata: None,
    }
}

impl PolicyGate {
    pub fn evaluate(
        &self,
        descriptor: &ToolDescriptor,
        request: &ToolCallRequest,
    ) -> ToolPolicyDecision {
        let outcome = if is_denied_until_sandbox(descriptor) {
            PolicyOutcome::Deny
        } else if is_read_only(descriptor) {
            PolicyOutcome::Allow
        } else {
            PolicyOutcome::AskUser
        };
        let approval_id = match outcome {
            PolicyOutcome::AskUser => Some(tessera_protocol::ApprovalId::new()),
            PolicyOutcome::Allow | PolicyOutcome::Deny => None,
        };
        let reason = match outcome {
            PolicyOutcome::Allow => "read_only_tool_allowed",
            PolicyOutcome::AskUser => "side_effect_requires_user_approval",
            PolicyOutcome::Deny => "dangerous_tool_denied_until_sandbox_exists",
        };

        ToolPolicyDecision {
            decision_id: PolicyDecisionId::new(),
            call_id: request.call_id.clone(),
            tool_id: request.tool_id.clone(),
            outcome,
            reason: reason.to_string(),
            required_permissions: descriptor.required_permissions.clone(),
            side_effects: descriptor.side_effects.clone(),
            approval_id,
        }
    }
}

impl WorkspaceGuardrailChecker {
    pub fn new(scope: WorkspaceScope) -> Self {
        Self { scope }
    }

    pub fn scope(&self) -> &WorkspaceScope {
        &self.scope
    }

    pub fn evaluate_tool_path(
        &self,
        descriptor: &ToolDescriptor,
        request: &ToolCallRequest,
        requested_path: impl AsRef<str>,
    ) -> SandboxDecision {
        let requested_path = requested_path.as_ref();
        let access = workspace_access(descriptor);
        let workspace_root = normalize_lexical(Path::new(&self.scope.workspace_root));
        let resolved_path = resolve_workspace_path(&workspace_root, requested_path);
        let within_workspace = resolved_path.starts_with(&workspace_root);
        let (kind, reason) = if is_denied_until_sandbox(descriptor) {
            (
                SandboxDecisionKind::Deny,
                "dangerous_tool_denied_until_sandbox_exists",
            )
        } else if !within_workspace {
            (SandboxDecisionKind::Deny, "path_outside_workspace")
        } else if access == WorkspaceAccess::Write {
            (
                SandboxDecisionKind::AskUser,
                "workspace_write_requires_approval",
            )
        } else {
            (SandboxDecisionKind::Allow, "workspace_read_allowed")
        };

        SandboxDecision {
            decision_id: SandboxDecisionId::new(),
            call_id: Some(request.call_id.clone()),
            tool_id: Some(request.tool_id.clone()),
            kind,
            reason: reason.to_string(),
            guardrail: WorkspaceGuardrail {
                scope: self.scope.clone(),
                requested_path: Some(requested_path.to_string()),
                resolved_path: Some(path_to_string(&resolved_path)),
                access,
                within_workspace,
                required_permissions: descriptor.required_permissions.clone(),
                side_effects: descriptor.side_effects.clone(),
            },
            metadata: None,
        }
    }
}

impl OsSandboxPlanner {
    pub fn new(workspace_root: impl Into<String>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
        }
    }

    pub fn plan_tool(&self, descriptor: &ToolDescriptor) -> OsSandboxProfile {
        let (mode, filesystem, network, requires_checkpoint, reason) =
            if is_denied_until_sandbox(descriptor) {
                (
                    OsSandboxMode::Denied,
                    OsSandboxFilesystem::Denied,
                    OsSandboxNetwork::Disabled,
                    false,
                    "dangerous_tool_requires_real_os_sandbox",
                )
            } else if requires_network(descriptor) {
                (
                    OsSandboxMode::NetworkRequired,
                    OsSandboxFilesystem::ReadOnly,
                    OsSandboxNetwork::Requested,
                    false,
                    "network_tool_requires_sandbox_policy",
                )
            } else if requires_workspace_write(descriptor) {
                (
                    OsSandboxMode::WorkspaceWrite,
                    OsSandboxFilesystem::WorkspaceWrite,
                    OsSandboxNetwork::Disabled,
                    true,
                    "workspace_write_requires_checkpointed_sandbox",
                )
            } else {
                (
                    OsSandboxMode::ReadOnly,
                    OsSandboxFilesystem::ReadOnly,
                    OsSandboxNetwork::Disabled,
                    false,
                    "read_only_tool_uses_read_only_sandbox_profile",
                )
            };

        OsSandboxProfile {
            profile_id: OsSandboxProfileId::new(),
            mode,
            workspace_root: Some(self.workspace_root.clone()),
            filesystem,
            network,
            shell: OsSandboxShell::Denied,
            requires_checkpoint,
            reason: reason.to_string(),
            metadata: None,
        }
    }
}

impl WorkspaceCheckpointPlanner {
    pub fn new(kind: SnapshotKind, storage_prefix: impl Into<String>) -> Self {
        let storage_prefix = storage_prefix.into();
        Self {
            kind,
            storage_prefix: storage_prefix.trim_end_matches('/').to_string(),
        }
    }

    pub fn plan_required_checkpoint(
        &self,
        sandbox_profile: &OsSandboxProfile,
        parent_snapshot_id: Option<SnapshotId>,
        summary: impl Into<String>,
    ) -> Option<WorkspaceCheckpoint> {
        if !sandbox_profile.requires_checkpoint {
            return None;
        }

        Some(self.plan_checkpoint(
            sandbox_profile.workspace_root.clone(),
            parent_snapshot_id,
            summary,
        ))
    }

    pub fn plan_checkpoint(
        &self,
        workspace_root: Option<String>,
        parent_snapshot_id: Option<SnapshotId>,
        summary: impl Into<String>,
    ) -> WorkspaceCheckpoint {
        let id = SnapshotId::new();
        WorkspaceCheckpoint {
            storage_uri: format!("{}/{}", self.storage_prefix, id.as_str()),
            id,
            kind: self.kind,
            workspace_root,
            parent_snapshot_id,
            summary: Some(summary.into()),
            metadata: None,
        }
    }
}

fn is_read_only(descriptor: &ToolDescriptor) -> bool {
    let permissions_are_read_only = descriptor
        .required_permissions
        .iter()
        .all(|permission| matches!(permission, ToolPermission::FilesystemRead));
    let side_effects_are_read_only = descriptor
        .side_effects
        .iter()
        .all(|side_effect| matches!(side_effect, ToolSideEffect::ReadOnly));

    permissions_are_read_only && side_effects_are_read_only
}

fn requires_network(descriptor: &ToolDescriptor) -> bool {
    descriptor
        .required_permissions
        .iter()
        .any(|permission| matches!(permission, ToolPermission::Network))
        || descriptor
            .side_effects
            .iter()
            .any(|side_effect| matches!(side_effect, ToolSideEffect::Network))
}

fn requires_workspace_write(descriptor: &ToolDescriptor) -> bool {
    descriptor.required_permissions.iter().any(|permission| {
        matches!(
            permission,
            ToolPermission::FilesystemWrite | ToolPermission::Git
        )
    }) || descriptor.side_effects.iter().any(|side_effect| {
        matches!(
            side_effect,
            ToolSideEffect::WritesWorkspace | ToolSideEffect::PersistentState
        )
    })
}

fn workspace_access(descriptor: &ToolDescriptor) -> WorkspaceAccess {
    if descriptor
        .required_permissions
        .iter()
        .any(|permission| matches!(permission, ToolPermission::Shell))
        || descriptor
            .side_effects
            .iter()
            .any(|side_effect| matches!(side_effect, ToolSideEffect::Shell))
    {
        WorkspaceAccess::Execute
    } else if requires_workspace_write(descriptor)
        || descriptor
            .side_effects
            .iter()
            .any(|side_effect| matches!(side_effect, ToolSideEffect::WritesOutsideWorkspace))
    {
        WorkspaceAccess::Write
    } else {
        WorkspaceAccess::Read
    }
}

fn is_denied_until_sandbox(descriptor: &ToolDescriptor) -> bool {
    descriptor
        .required_permissions
        .iter()
        .any(|permission| matches!(permission, ToolPermission::Shell | ToolPermission::EnvRead))
        || descriptor.side_effects.iter().any(|side_effect| {
            matches!(
                side_effect,
                ToolSideEffect::Shell | ToolSideEffect::WritesOutsideWorkspace
            )
        })
}

fn resolve_workspace_path(workspace_root: &Path, requested_path: &str) -> PathBuf {
    let requested_path = Path::new(requested_path);
    let joined = if requested_path.is_absolute() {
        PathBuf::from(requested_path)
    } else {
        workspace_root.join(requested_path)
    };

    normalize_lexical(&joined)
}

fn normalize_lexical(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new("/")),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push("..");
                }
            }
            Component::Normal(segment) => normalized.push(segment),
        }
    }

    if normalized.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        normalized
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

impl Default for NoProgressDetector {
    fn default() -> Self {
        Self::new(NoProgressPolicy::default())
    }
}

impl NoProgressDetector {
    pub fn new(policy: NoProgressPolicy) -> Self {
        Self {
            policy,
            no_output_count: 0,
            read_only_count: 0,
            repair_count: 0,
            current_assistant_message_has_output: false,
        }
    }

    pub fn observe_event(&mut self, event: &RunEvent) -> Option<NoProgressLoop> {
        match event {
            RunEvent::AssistantMessageStarted { .. } => {
                self.current_assistant_message_has_output = false;
                None
            }
            RunEvent::AssistantDelta { text, .. } => {
                if text.trim().is_empty() {
                    None
                } else {
                    self.current_assistant_message_has_output = true;
                    self.record_observation(NoProgressObservation::AssistantOutput)
                }
            }
            RunEvent::AssistantMessageCompleted { .. } => {
                if self.current_assistant_message_has_output {
                    self.current_assistant_message_has_output = false;
                    None
                } else {
                    self.record_observation(NoProgressObservation::NoOutput)
                }
            }
            _ => None,
        }
    }

    pub fn record_observation(
        &mut self,
        observation: NoProgressObservation,
    ) -> Option<NoProgressLoop> {
        match observation {
            NoProgressObservation::AssistantOutput => {
                self.no_output_count = 0;
                self.read_only_count = 0;
                self.repair_count = 0;
                None
            }
            NoProgressObservation::NoOutput => {
                self.no_output_count = self.no_output_count.saturating_add(1);
                self.read_only_count = 0;
                self.repair_count = 0;
                no_progress_loop(
                    NoProgressSignalKind::NoOutput,
                    self.no_output_count,
                    self.policy.no_output_threshold,
                )
            }
            NoProgressObservation::ReadOnlyStep => {
                self.read_only_count = self.read_only_count.saturating_add(1);
                self.no_output_count = 0;
                self.repair_count = 0;
                no_progress_loop(
                    NoProgressSignalKind::RepeatedReadOnly,
                    self.read_only_count,
                    self.policy.repeated_read_only_threshold,
                )
            }
            NoProgressObservation::RepairStep => {
                self.repair_count = self.repair_count.saturating_add(1);
                self.no_output_count = 0;
                self.read_only_count = 0;
                no_progress_loop(
                    NoProgressSignalKind::RepeatedRepair,
                    self.repair_count,
                    self.policy.repeated_repair_threshold,
                )
            }
        }
    }
}

fn no_progress_loop(
    kind: NoProgressSignalKind,
    consecutive_count: u32,
    threshold: u32,
) -> Option<NoProgressLoop> {
    let threshold = threshold.max(1);
    if consecutive_count < threshold {
        return None;
    }

    let (action, reason) = match kind {
        NoProgressSignalKind::NoOutput => {
            (NoProgressAction::Stop, "assistant_completed_without_output")
        }
        NoProgressSignalKind::RepeatedReadOnly => (
            NoProgressAction::AskUser,
            "repeated_read_only_steps_without_new_output",
        ),
        NoProgressSignalKind::RepeatedRepair => (
            NoProgressAction::Summarize,
            "repeated_repair_steps_without_new_output",
        ),
    };

    Some(NoProgressLoop {
        kind,
        consecutive_count,
        threshold,
        action,
        reason: reason.to_string(),
        route_escalation_allowed: false,
    })
}

struct RunContext {
    trace_id: String,
    thread_id: ThreadId,
    turn_id: TurnId,
    task_id: TaskId,
    provider_id: ProviderId,
    profile_id: ModelProfileId,
    model: String,
    seq: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplaySummary {
    pub trace_id: String,
    pub assistant_text: String,
    pub event_kinds: Vec<String>,
}

pub struct ReplayRunner<'a> {
    store: &'a TraceStore,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeEventQuery {
    pub trace_id: String,
    pub since_seq: Option<u64>,
    pub limit: Option<usize>,
}

impl RuntimeEventQuery {
    pub fn new(trace_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
            since_seq: None,
            limit: None,
        }
    }

    pub fn since_seq(mut self, seq: u64) -> Self {
        self.since_seq = Some(seq);
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeEventPage {
    pub trace_id: String,
    pub records: Vec<TraceRecord>,
    pub next_since_seq: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeHttpEventRequest {
    pub trace_id: String,
    pub since_seq: Option<u64>,
    pub limit: Option<usize>,
}

impl RuntimeHttpEventRequest {
    pub fn new(trace_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
            since_seq: None,
            limit: None,
        }
    }

    pub fn since_seq(mut self, seq: u64) -> Self {
        self.since_seq = Some(seq);
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    fn into_query(self) -> RuntimeEventQuery {
        let mut query = RuntimeEventQuery::new(self.trace_id);
        if let Some(since_seq) = self.since_seq {
            query = query.since_seq(since_seq);
        }
        if let Some(limit) = self.limit {
            query = query.limit(limit);
        }
        query
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeSseFrame {
    pub id: String,
    pub event: String,
    pub data: String,
}

impl RuntimeSseFrame {
    pub fn encode(&self) -> String {
        let mut encoded = String::new();
        encoded.push_str("id: ");
        encoded.push_str(&self.id);
        encoded.push('\n');
        encoded.push_str("event: ");
        encoded.push_str(&self.event);
        encoded.push('\n');

        for line in self.data.lines() {
            encoded.push_str("data: ");
            encoded.push_str(line);
            encoded.push('\n');
        }

        encoded.push('\n');
        encoded
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeObjectIndex {
    pub threads: Vec<ThreadId>,
    pub turns: Vec<TurnId>,
    pub items: Vec<ItemId>,
    pub tasks: Vec<TaskId>,
    pub artifacts: Vec<ArtifactId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeTaskSummary {
    pub task_id: TaskId,
    pub kind: Option<TaskKind>,
    pub status: TaskStatus,
    pub thread_id: Option<ThreadId>,
    pub turn_id: Option<TurnId>,
    pub created_at: Option<Timestamp>,
    pub started_at: Option<Timestamp>,
    pub finished_at: Option<Timestamp>,
    pub cancel_reason: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

impl RuntimeTaskSummary {
    fn new(task_id: TaskId) -> Self {
        Self {
            task_id,
            kind: None,
            status: TaskStatus::Pending,
            thread_id: None,
            turn_id: None,
            created_at: None,
            started_at: None,
            finished_at: None,
            cancel_reason: None,
            error_code: None,
            error_message: None,
        }
    }

    fn update_scope(&mut self, thread_id: Option<ThreadId>, turn_id: Option<TurnId>) {
        if thread_id.is_some() {
            self.thread_id = thread_id;
        }
        if turn_id.is_some() {
            self.turn_id = turn_id;
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeArtifactSummary {
    pub artifact_id: ArtifactId,
    pub kind: Option<ArtifactKind>,
    pub thread_id: Option<ThreadId>,
    pub turn_id: Option<TurnId>,
    pub task_id: Option<TaskId>,
    pub item_id: Option<ItemId>,
    pub created_at: Option<Timestamp>,
    pub referenced_by_event_kinds: Vec<String>,
}

impl RuntimeArtifactSummary {
    fn new(artifact_id: ArtifactId) -> Self {
        Self {
            artifact_id,
            kind: None,
            thread_id: None,
            turn_id: None,
            task_id: None,
            item_id: None,
            created_at: None,
            referenced_by_event_kinds: Vec::new(),
        }
    }

    fn update_scope(
        &mut self,
        thread_id: Option<ThreadId>,
        turn_id: Option<TurnId>,
        task_id: Option<TaskId>,
        item_id: Option<ItemId>,
    ) {
        if thread_id.is_some() {
            self.thread_id = thread_id;
        }
        if turn_id.is_some() {
            self.turn_id = turn_id;
        }
        if task_id.is_some() {
            self.task_id = task_id;
        }
        if item_id.is_some() {
            self.item_id = item_id;
        }
    }

    fn record_reference(&mut self, event_kind: &str) {
        if !self
            .referenced_by_event_kinds
            .iter()
            .any(|existing| existing == event_kind)
        {
            self.referenced_by_event_kinds.push(event_kind.to_string());
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeSnapshotSummary {
    pub snapshot_id: SnapshotId,
    pub kind: Option<SnapshotKind>,
    pub task_id: Option<TaskId>,
    pub turn_id: Option<TurnId>,
    pub created_at: Option<Timestamp>,
    pub storage_uri: Option<String>,
    pub workspace_root: Option<String>,
    pub parent_snapshot_id: Option<SnapshotId>,
    pub summary: Option<String>,
}

impl RuntimeSnapshotSummary {
    fn new(snapshot_id: SnapshotId) -> Self {
        Self {
            snapshot_id,
            kind: None,
            task_id: None,
            turn_id: None,
            created_at: None,
            storage_uri: None,
            workspace_root: None,
            parent_snapshot_id: None,
            summary: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimePauseCheckpointSummary {
    pub checkpoint_id: TaskPauseCheckpointId,
    pub task_id: TaskId,
    pub trace_id: String,
    pub event_seq: u64,
    pub last_seq: u64,
    pub thread_id: Option<ThreadId>,
    pub turn_id: Option<TurnId>,
    pub provider_id: ProviderId,
    pub profile_id: ModelProfileId,
    pub model: String,
    pub resume_mode: ResumeMode,
    pub workspace_snapshot_id: Option<SnapshotId>,
    pub transcript_event_range: Option<EventRange>,
    pub context_handle_ids: Vec<ContextId>,
    pub reason: Option<String>,
    pub created_at: Option<Timestamp>,
}

impl RuntimePauseCheckpointSummary {
    fn from_checkpoint(record: &TraceRecord, checkpoint: TaskPauseCheckpoint) -> Self {
        Self {
            checkpoint_id: checkpoint.checkpoint_id,
            task_id: checkpoint.task_id,
            trace_id: checkpoint.trace_id,
            event_seq: record.seq,
            last_seq: checkpoint.last_seq,
            thread_id: checkpoint.thread_id,
            turn_id: checkpoint.turn_id,
            provider_id: checkpoint.provider_id,
            profile_id: checkpoint.profile_id,
            model: checkpoint.model,
            resume_mode: checkpoint.resume_mode,
            workspace_snapshot_id: checkpoint.workspace_snapshot_id,
            transcript_event_range: checkpoint.transcript_event_range,
            context_handle_ids: checkpoint.context_handle_ids,
            reason: checkpoint.reason,
            created_at: Some(record.timestamp.clone()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeSessionSummary {
    pub trace_id: String,
    pub event_count: usize,
    pub first_timestamp: Option<Timestamp>,
    pub updated_at: Option<Timestamp>,
    pub last_seq: u64,
    pub last_event_kind: Option<String>,
    pub user_preview: String,
    pub assistant_preview: String,
}

pub struct RuntimeReader {
    store: TraceStore,
}

pub struct RuntimeTaskResumer {
    store: TraceStore,
}

pub struct RuntimeHttpApi {
    reader: RuntimeReader,
}

impl RuntimeReader {
    pub fn new(store: TraceStore) -> Self {
        Self { store }
    }

    pub fn list_events(&self, query: RuntimeEventQuery) -> Result<RuntimeEventPage> {
        let since_seq = query.since_seq.unwrap_or(0);
        let mut records = self
            .store
            .read_trace_records(&query.trace_id)?
            .into_iter()
            .filter(|record| record.seq > since_seq)
            .collect::<Vec<_>>();

        if let Some(limit) = query.limit {
            records.truncate(limit);
        }

        let next_since_seq = records.last().map(|record| record.seq);
        Ok(RuntimeEventPage {
            trace_id: query.trace_id,
            records,
            next_since_seq,
        })
    }

    pub fn list_sessions(&self) -> Result<Vec<RuntimeSessionSummary>> {
        let mut sessions = Vec::new();
        for trace_id in self.store.list_trace_ids()? {
            let records = self.store.read_trace_records(&trace_id)?;
            if records.is_empty() {
                continue;
            }
            sessions.push(summarize_session(trace_id, &records));
        }

        sessions.sort_by(|left, right| {
            let left_updated = left
                .updated_at
                .as_ref()
                .map(|timestamp| timestamp.as_str())
                .unwrap_or("");
            let right_updated = right
                .updated_at
                .as_ref()
                .map(|timestamp| timestamp.as_str())
                .unwrap_or("");
            right_updated
                .cmp(left_updated)
                .then_with(|| left.trace_id.cmp(&right.trace_id))
        });
        Ok(sessions)
    }

    pub fn list_objects(&self, trace_id: &str) -> Result<RuntimeObjectIndex> {
        let objects = self.store.list_indexed_objects(trace_id)?;
        Ok(RuntimeObjectIndex {
            threads: objects.threads,
            turns: objects.turns,
            items: objects.items,
            tasks: objects.tasks,
            artifacts: objects.artifacts,
        })
    }

    pub fn list_tasks(&self, trace_id: &str) -> Result<Vec<RuntimeTaskSummary>> {
        let records = self.store.read_trace_records(trace_id)?;
        let mut tasks = Vec::new();
        for record in records {
            apply_task_record(&mut tasks, &record);
        }
        Ok(tasks)
    }

    pub fn list_artifacts(&self, trace_id: &str) -> Result<Vec<RuntimeArtifactSummary>> {
        let records = self.store.read_trace_records(trace_id)?;
        let mut artifacts = Vec::new();
        for record in records {
            apply_artifact_record(&mut artifacts, &record);
        }
        Ok(artifacts)
    }

    pub fn list_snapshots(&self, trace_id: &str) -> Result<Vec<RuntimeSnapshotSummary>> {
        let records = self.store.read_trace_records(trace_id)?;
        let mut snapshots = Vec::new();
        for record in records {
            apply_snapshot_record(&mut snapshots, &record);
        }
        Ok(snapshots)
    }

    pub fn list_pause_checkpoints(
        &self,
        trace_id: &str,
    ) -> Result<Vec<RuntimePauseCheckpointSummary>> {
        let records = self.store.read_trace_records(trace_id)?;
        let mut checkpoints = Vec::new();
        for record in records {
            apply_pause_checkpoint_record(&mut checkpoints, &record)?;
        }
        checkpoints.sort_by_key(|checkpoint| checkpoint.event_seq);
        Ok(checkpoints)
    }

    pub fn find_pause_checkpoint(
        &self,
        task_id: &TaskId,
    ) -> Result<Option<RuntimePauseCheckpointSummary>> {
        let mut latest = None;
        for trace_id in self.store.list_trace_ids()? {
            for checkpoint in self.list_pause_checkpoints(&trace_id)? {
                if checkpoint.task_id != *task_id {
                    continue;
                }

                let should_replace =
                    latest
                        .as_ref()
                        .is_none_or(|existing: &RuntimePauseCheckpointSummary| {
                            pause_checkpoint_sort_key(&checkpoint)
                                > pause_checkpoint_sort_key(existing)
                        });
                if should_replace {
                    latest = Some(checkpoint);
                }
            }
        }
        Ok(latest)
    }
}

impl RuntimeTaskResumer {
    pub fn new(store: TraceStore) -> Self {
        Self { store }
    }

    pub fn mark_task_resumed(
        &mut self,
        checkpoint: &RuntimePauseCheckpointSummary,
        reason: Option<String>,
    ) -> Result<()> {
        let records = self.store.read_trace_records(&checkpoint.trace_id)?;
        let next_seq = records
            .iter()
            .map(|record| record.seq)
            .max()
            .unwrap_or_default()
            + 1;
        let mut frame = EventFrame::new(
            checkpoint.trace_id.clone(),
            next_seq,
            RunEvent::TaskResumed {
                task_id: checkpoint.task_id.clone(),
                reason,
            },
        )
        .with_task_id(checkpoint.task_id.clone());
        if let Some(thread_id) = checkpoint.thread_id.clone() {
            frame = frame.with_thread_id(thread_id);
        }
        if let Some(turn_id) = checkpoint.turn_id.clone() {
            frame = frame.with_turn_id(turn_id);
        }

        self.store.append(&frame)?;
        Ok(())
    }
}

fn summarize_session(trace_id: String, records: &[TraceRecord]) -> RuntimeSessionSummary {
    let mut user_preview = String::new();
    let mut assistant_preview = String::new();

    for record in records {
        match record.event_kind.as_str() {
            "user_message_recorded" => {
                append_preview_text(&mut user_preview, &record.payload);
            }
            "assistant_delta" => {
                append_preview_text(&mut assistant_preview, &record.payload);
            }
            _ => {}
        }
    }

    RuntimeSessionSummary {
        trace_id,
        event_count: records.len(),
        first_timestamp: records.first().map(|record| record.timestamp.clone()),
        updated_at: records.last().map(|record| record.timestamp.clone()),
        last_seq: records.last().map(|record| record.seq).unwrap_or_default(),
        last_event_kind: records.last().map(|record| record.event_kind.clone()),
        user_preview: truncate_preview(&user_preview, 120),
        assistant_preview: truncate_preview(&assistant_preview, 120),
    }
}

fn append_preview_text(preview: &mut String, payload: &serde_json::Value) {
    let Some(text) = payload.get("text").and_then(|value| value.as_str()) else {
        return;
    };
    if !preview.is_empty() {
        preview.push(' ');
    }
    preview.push_str(text.trim());
}

fn truncate_preview(input: &str, max_chars: usize) -> String {
    let mut output = input.chars().take(max_chars).collect::<String>();
    if input.chars().count() > max_chars {
        output.push_str("...");
    }
    output
}

impl RuntimeHttpApi {
    pub fn new(reader: RuntimeReader) -> Self {
        Self { reader }
    }

    pub fn list_events(&self, request: RuntimeHttpEventRequest) -> Result<RuntimeEventPage> {
        self.reader.list_events(request.into_query())
    }

    pub fn list_events_json(&self, request: RuntimeHttpEventRequest) -> Result<serde_json::Value> {
        let page = self.list_events(request)?;
        Ok(serde_json::json!({
            "trace_id": page.trace_id,
            "records": page.records,
            "next_since_seq": page.next_since_seq,
        }))
    }

    pub fn sse_event_frames(
        &self,
        request: RuntimeHttpEventRequest,
    ) -> Result<Vec<RuntimeSseFrame>> {
        let page = self.list_events(request)?;
        page.records
            .into_iter()
            .map(|record| {
                Ok(RuntimeSseFrame {
                    id: record.seq.to_string(),
                    event: record.event_kind.clone(),
                    data: serde_json::to_string(&record)?,
                })
            })
            .collect()
    }
}

fn apply_task_record(tasks: &mut Vec<RuntimeTaskSummary>, record: &TraceRecord) {
    match record.event_kind.as_str() {
        "task_created" => {
            let Some(task_id) = trace_record_task_id(record) else {
                return;
            };
            let kind = record
                .payload
                .get("kind")
                .and_then(|value| value.as_str())
                .and_then(TaskKind::from_snake_case);
            let task = task_mut_or_insert(tasks, &task_id);
            task.kind = kind;
            task.status = TaskStatus::Pending;
            task.created_at = Some(record.timestamp.clone());
            task.finished_at = None;
            task.cancel_reason = None;
            task.error_code = None;
            task.error_message = None;
            task.update_scope(record.thread_id.clone(), record.turn_id.clone());
        }
        "task_started" => {
            let Some(task_id) = trace_record_task_id(record) else {
                return;
            };
            let task = task_mut_or_insert(tasks, &task_id);
            task.status = TaskStatus::Running;
            task.started_at = Some(record.timestamp.clone());
            task.update_scope(record.thread_id.clone(), record.turn_id.clone());
        }
        "task_completed" => {
            let Some(task_id) = trace_record_task_id(record) else {
                return;
            };
            let task = task_mut_or_insert(tasks, &task_id);
            task.status = TaskStatus::Completed;
            task.finished_at = Some(record.timestamp.clone());
        }
        "task_failed" => {
            let Some(task_id) = trace_record_task_id(record) else {
                return;
            };
            let task = task_mut_or_insert(tasks, &task_id);
            task.status = TaskStatus::Failed;
            task.finished_at = Some(record.timestamp.clone());
            task.error_code = record
                .payload
                .get("error")
                .and_then(|error| error.get("code"))
                .and_then(|value| value.as_str())
                .map(str::to_string);
            task.error_message = record
                .payload
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(|value| value.as_str())
                .map(str::to_string);
        }
        "task_cancelled" => {
            let Some(task_id) = trace_record_task_id(record) else {
                return;
            };
            let task = task_mut_or_insert(tasks, &task_id);
            task.status = TaskStatus::Cancelled;
            task.finished_at = Some(record.timestamp.clone());
            task.cancel_reason = record
                .payload
                .get("reason")
                .and_then(|value| value.as_str())
                .map(str::to_string);
        }
        "task_paused" => {
            let Some(task_id) = trace_record_task_id(record) else {
                return;
            };
            let task = task_mut_or_insert(tasks, &task_id);
            task.status = TaskStatus::Paused;
            task.update_scope(record.thread_id.clone(), record.turn_id.clone());
        }
        "task_resumed" => {
            let Some(task_id) = trace_record_task_id(record) else {
                return;
            };
            let task = task_mut_or_insert(tasks, &task_id);
            task.status = TaskStatus::Running;
            if task.started_at.is_none() {
                task.started_at = Some(record.timestamp.clone());
            }
            task.finished_at = None;
            task.update_scope(record.thread_id.clone(), record.turn_id.clone());
        }
        _ => {}
    }
}

fn task_mut_or_insert<'a>(
    tasks: &'a mut Vec<RuntimeTaskSummary>,
    task_id: &TaskId,
) -> &'a mut RuntimeTaskSummary {
    if let Some(index) = tasks.iter().position(|task| &task.task_id == task_id) {
        return &mut tasks[index];
    }

    tasks.push(RuntimeTaskSummary::new(task_id.clone()));
    tasks
        .last_mut()
        .expect("task was just inserted into non-empty registry")
}

fn trace_record_task_id(record: &TraceRecord) -> Option<TaskId> {
    record.task_id.clone().or_else(|| {
        record
            .payload
            .get("task_id")
            .and_then(|value| value.as_str())
            .map(TaskId::from)
    })
}

fn apply_artifact_record(artifacts: &mut Vec<RuntimeArtifactSummary>, record: &TraceRecord) {
    if record.event_kind == "artifact_created" {
        if let Some(artifact_id) = trace_record_artifact_id(record) {
            let kind = record
                .payload
                .get("kind")
                .and_then(|value| value.as_str())
                .and_then(ArtifactKind::from_snake_case);
            let artifact = artifact_mut_or_insert(artifacts, &artifact_id);
            artifact.kind = kind;
            artifact.created_at = Some(record.timestamp.clone());
            artifact.update_scope(
                record.thread_id.clone(),
                record.turn_id.clone(),
                record.task_id.clone(),
                record.item_id.clone(),
            );
        }
    }

    if record.artifact_refs.is_empty() {
        return;
    }

    for artifact_id in &record.artifact_refs {
        let artifact = artifact_mut_or_insert(artifacts, artifact_id);
        artifact.update_scope(
            record.thread_id.clone(),
            record.turn_id.clone(),
            record.task_id.clone(),
            record.item_id.clone(),
        );
        artifact.record_reference(&record.event_kind);
    }
}

fn artifact_mut_or_insert<'a>(
    artifacts: &'a mut Vec<RuntimeArtifactSummary>,
    artifact_id: &ArtifactId,
) -> &'a mut RuntimeArtifactSummary {
    if let Some(index) = artifacts
        .iter()
        .position(|artifact| &artifact.artifact_id == artifact_id)
    {
        return &mut artifacts[index];
    }

    artifacts.push(RuntimeArtifactSummary::new(artifact_id.clone()));
    artifacts
        .last_mut()
        .expect("artifact was just inserted into non-empty registry")
}

fn trace_record_artifact_id(record: &TraceRecord) -> Option<ArtifactId> {
    record
        .payload
        .get("artifact_id")
        .and_then(|value| value.as_str())
        .map(ArtifactId::from)
}

fn apply_snapshot_record(snapshots: &mut Vec<RuntimeSnapshotSummary>, record: &TraceRecord) {
    if record.event_kind != "snapshot_created" {
        return;
    }

    let Some(snapshot_id) = trace_record_snapshot_id(record) else {
        return;
    };
    let checkpoint = &record.payload["checkpoint"];
    let snapshot = snapshot_mut_or_insert(snapshots, &snapshot_id);
    snapshot.kind = checkpoint
        .get("kind")
        .and_then(|value| value.as_str())
        .and_then(SnapshotKind::from_snake_case);
    snapshot.task_id = record.task_id.clone();
    snapshot.turn_id = record.turn_id.clone();
    snapshot.created_at = Some(record.timestamp.clone());
    snapshot.storage_uri = checkpoint
        .get("storage_uri")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    snapshot.workspace_root = checkpoint
        .get("workspace_root")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    snapshot.parent_snapshot_id = checkpoint
        .get("parent_snapshot_id")
        .and_then(|value| value.as_str())
        .map(SnapshotId::from);
    snapshot.summary = checkpoint
        .get("summary")
        .and_then(|value| value.as_str())
        .map(str::to_string);
}

fn snapshot_mut_or_insert<'a>(
    snapshots: &'a mut Vec<RuntimeSnapshotSummary>,
    snapshot_id: &SnapshotId,
) -> &'a mut RuntimeSnapshotSummary {
    if let Some(index) = snapshots
        .iter()
        .position(|snapshot| &snapshot.snapshot_id == snapshot_id)
    {
        return &mut snapshots[index];
    }

    snapshots.push(RuntimeSnapshotSummary::new(snapshot_id.clone()));
    snapshots
        .last_mut()
        .expect("snapshot was just inserted into non-empty registry")
}

fn trace_record_snapshot_id(record: &TraceRecord) -> Option<SnapshotId> {
    record
        .payload
        .get("checkpoint")
        .and_then(|checkpoint| checkpoint.get("id"))
        .and_then(|value| value.as_str())
        .map(SnapshotId::from)
}

fn apply_pause_checkpoint_record(
    checkpoints: &mut Vec<RuntimePauseCheckpointSummary>,
    record: &TraceRecord,
) -> Result<()> {
    if record.event_kind != "task_pause_checkpoint_created" {
        return Ok(());
    }

    let Some(checkpoint_payload) = record.payload.get("checkpoint") else {
        return Ok(());
    };
    let checkpoint: TaskPauseCheckpoint = serde_json::from_value(checkpoint_payload.clone())?;
    let summary = RuntimePauseCheckpointSummary::from_checkpoint(record, checkpoint);

    if let Some(index) = checkpoints
        .iter()
        .position(|checkpoint| checkpoint.task_id == summary.task_id)
    {
        if checkpoints[index].event_seq <= summary.event_seq {
            checkpoints[index] = summary;
        }
        return Ok(());
    }

    checkpoints.push(summary);
    Ok(())
}

fn pause_checkpoint_sort_key(checkpoint: &RuntimePauseCheckpointSummary) -> (&str, &str, u64) {
    (
        checkpoint
            .created_at
            .as_ref()
            .map(Timestamp::as_str)
            .unwrap_or(""),
        checkpoint.trace_id.as_str(),
        checkpoint.event_seq,
    )
}

impl<'a> ReplayRunner<'a> {
    pub fn new(store: &'a TraceStore) -> Self {
        Self { store }
    }

    pub fn replay(&self, trace_id: &str) -> Result<ReplaySummary> {
        let records = self.store.read_trace_records(trace_id)?;
        let mut assistant_text = String::new();
        let mut event_kinds = Vec::new();

        for record in records {
            if record.event_kind == "assistant_delta" {
                if let Some(text) = record.payload.get("text").and_then(|value| value.as_str()) {
                    assistant_text.push_str(text);
                }
            }
            event_kinds.push(record.event_kind);
        }

        Ok(ReplaySummary {
            trace_id: trace_id.to_string(),
            assistant_text,
            event_kinds,
        })
    }
}

impl<P> AgentLoop<P>
where
    P: ChatProvider,
{
    pub fn new(provider: P, store: TraceStore) -> Self {
        Self { provider, store }
    }

    pub async fn run_agent(self, request: AgentRunRequest) -> Result<AgentRunOutcome> {
        self.run_agent_with_controls_and_event_sink(request, RunControls::default(), |_| {})
            .await
    }

    pub async fn run_agent_with_controls_and_event_sink<F, R>(
        mut self,
        request: AgentRunRequest,
        controls: RunControls,
        mut event_sink: F,
    ) -> Result<AgentRunOutcome>
    where
        F: FnMut(&EventFrame) -> R,
        R: Into<EventSinkAction>,
    {
        if request.max_steps == 0 || request.agent_profile.max_steps == 0 {
            return Err(CoreError::InvalidRequest(
                "agent max_steps must be at least 1".to_string(),
            ));
        }

        let trace_id = request.trace_id.clone();
        let mut context = RunContext {
            trace_id: trace_id.clone(),
            thread_id: ThreadId::new(),
            turn_id: TurnId::new(),
            task_id: TaskId::new(),
            provider_id: request.provider_id.clone(),
            profile_id: request.profile_id.clone(),
            model: request.model.clone(),
            seq: 1,
        };
        let task_id = context.task_id.clone();
        let user_item_id = ItemId::new();
        let assistant_item_id = ItemId::new();
        let mut assistant_text = String::new();
        let mut no_progress_detector = NoProgressDetector::default();
        let provider_messages = request.provider_messages();
        let instruction_sources = request
            .instruction_context
            .as_ref()
            .map(|context| context.sources.clone())
            .unwrap_or_default();
        let skill_activations = request
            .skill_context
            .as_ref()
            .map(|context| context.activations_for_task(&task_id))
            .unwrap_or_default();
        let objective = request.objective.clone();
        let agent_profile_id = request.agent_profile.id.clone();
        let cancellation_token = controls.cancellation_token.clone();
        let pause_token = controls.pause_token.clone();

        macro_rules! append_event {
            ($event:expr) => {{
                let action = self.append_contextual(&mut context, $event, &mut event_sink)?;
                if let Some(reason) = action.cancel_reason() {
                    let summary = agent_step_summary(
                        &task_id,
                        1,
                        AgentStepStatus::Cancelled,
                        &assistant_text,
                        Some(reason.clone()),
                    );
                    let _ = self.append_contextual(
                        &mut context,
                        RunEvent::AgentStepCompleted { summary },
                        &mut event_sink,
                    )?;
                    return self.finish_cancelled(
                        task_id,
                        agent_profile_id,
                        assistant_text,
                        &mut context,
                        reason,
                        &mut event_sink,
                    );
                }
            }};
        }

        append_event!(RunEvent::TaskCreated {
            task_id: task_id.clone(),
            kind: TaskKind::AgentRun,
        });
        append_event!(RunEvent::TaskStarted {
            task_id: task_id.clone(),
        });
        let thread_id = context.thread_id.clone();
        append_event!(RunEvent::ThreadCreated { thread_id });
        let turn_id = context.turn_id.clone();
        append_event!(RunEvent::TurnStarted { turn_id });
        append_event!(RunEvent::UserMessageRecorded {
            item_id: user_item_id,
            text: objective.clone(),
        });
        if !instruction_sources.is_empty() {
            append_event!(RunEvent::InstructionsDiscovered {
                task_id: task_id.clone(),
                sources: instruction_sources,
            });
        }
        for activation in skill_activations {
            append_event!(RunEvent::SkillActivated {
                task_id: task_id.clone(),
                activation,
            });
        }
        append_event!(RunEvent::AgentRunStarted {
            task_id: task_id.clone(),
            profile_id: request.agent_profile.id.clone(),
            objective: objective.clone(),
        });
        append_event!(RunEvent::AgentStepStarted {
            task_id: task_id.clone(),
            step_index: 1,
        });

        if let Some(reason) = cancellation_token
            .as_ref()
            .and_then(RunCancellationToken::cancellation_reason)
        {
            let summary = agent_step_summary(
                &task_id,
                1,
                AgentStepStatus::Cancelled,
                &assistant_text,
                Some(reason.clone()),
            );
            append_event!(RunEvent::AgentStepCompleted { summary });
            return self.finish_cancelled(
                task_id,
                agent_profile_id,
                assistant_text,
                &mut context,
                reason,
                &mut event_sink,
            );
        }
        if let Some(reason) = pause_token.as_ref().and_then(RunPauseToken::pause_reason) {
            let summary = agent_step_summary(
                &task_id,
                1,
                AgentStepStatus::Paused,
                &assistant_text,
                Some(reason.clone()),
            );
            append_event!(RunEvent::AgentStepCompleted { summary });
            return self.finish_paused(
                task_id,
                agent_profile_id,
                assistant_text,
                &mut context,
                reason,
                &mut event_sink,
            );
        }

        let capability = match self.provider.capability().await {
            Ok(capability) => capability,
            Err(error) => {
                return self.finish_failed(
                    task_id,
                    agent_profile_id,
                    assistant_text,
                    &mut context,
                    error,
                    &mut event_sink,
                );
            }
        };

        let route_decision = ModelRouter::draft().route(ModelRouteRequest {
            requested_profile: Some(request.profile_id.clone()),
            default_profile: request.profile_id.clone(),
            requested_model: request.model.clone(),
            reasoning_level: None,
            provider_capability: Some(capability.clone()),
        });
        let selected_profile = route_decision.selected_profile.clone();
        let selected_model = route_decision.selected_model.clone();
        context.profile_id = selected_profile.clone();
        context.model = selected_model.clone();

        append_event!(RunEvent::ProviderCapabilityReported {
            provider_id: request.provider_id.clone(),
            capability,
        });
        append_event!(RunEvent::RouteDecisionRecorded {
            decision_id: RouteDecisionId::new(),
            decision: route_decision,
        });
        append_event!(RunEvent::ProviderRequestStarted {
            provider_id: request.provider_id.clone(),
            profile_id: selected_profile.clone(),
            model: selected_model.clone(),
        });

        let mut stream = match self
            .provider
            .stream_chat(ProviderRequest {
                provider_id: request.provider_id.clone(),
                profile_id: selected_profile,
                model: selected_model,
                prompt: objective,
                messages: provider_messages,
                assistant_item_id,
            })
            .await
        {
            Ok(stream) => stream,
            Err(error) => {
                return self.finish_failed(
                    task_id,
                    agent_profile_id,
                    assistant_text,
                    &mut context,
                    error,
                    &mut event_sink,
                );
            }
        };

        loop {
            if let Some(reason) = cancellation_token
                .as_ref()
                .and_then(RunCancellationToken::cancellation_reason)
            {
                let summary = agent_step_summary(
                    &task_id,
                    1,
                    AgentStepStatus::Cancelled,
                    &assistant_text,
                    Some(reason.clone()),
                );
                append_event!(RunEvent::AgentStepCompleted { summary });
                return self.finish_cancelled(
                    task_id,
                    agent_profile_id,
                    assistant_text,
                    &mut context,
                    reason,
                    &mut event_sink,
                );
            }
            if let Some(reason) = pause_token.as_ref().and_then(RunPauseToken::pause_reason) {
                let summary = agent_step_summary(
                    &task_id,
                    1,
                    AgentStepStatus::Paused,
                    &assistant_text,
                    Some(reason.clone()),
                );
                append_event!(RunEvent::AgentStepCompleted { summary });
                return self.finish_paused(
                    task_id,
                    agent_profile_id,
                    assistant_text,
                    &mut context,
                    reason,
                    &mut event_sink,
                );
            }

            let next_event = match controls.event_timeout {
                Some(timeout) => {
                    tokio::select! {
                        signal = next_run_control_signal(cancellation_token.as_ref(), pause_token.as_ref()) => {
                            match signal {
                                RunControlSignal::Cancelled(reason) => {
                                    let summary = agent_step_summary(
                                        &task_id,
                                        1,
                                        AgentStepStatus::Cancelled,
                                        &assistant_text,
                                        Some(reason.clone()),
                                    );
                                    append_event!(RunEvent::AgentStepCompleted { summary });
                                    return self.finish_cancelled(
                                        task_id,
                                        agent_profile_id,
                                        assistant_text,
                                        &mut context,
                                        reason,
                                        &mut event_sink,
                                    );
                                }
                                RunControlSignal::Paused(reason) => {
                                    let summary = agent_step_summary(
                                        &task_id,
                                        1,
                                        AgentStepStatus::Paused,
                                        &assistant_text,
                                        Some(reason.clone()),
                                    );
                                    append_event!(RunEvent::AgentStepCompleted { summary });
                                    return self.finish_paused(
                                        task_id,
                                        agent_profile_id,
                                        assistant_text,
                                        &mut context,
                                        reason,
                                        &mut event_sink,
                                    );
                                }
                            }
                        }
                        timed = tokio::time::timeout(timeout, stream.try_next()) => {
                            match timed {
                                Ok(Ok(result)) => result,
                                Ok(Err(error)) => {
                                    return self.finish_failed(
                                        task_id,
                                        agent_profile_id,
                                        assistant_text,
                                        &mut context,
                                        error,
                                        &mut event_sink,
                                    );
                                }
                                Err(_) => {
                                    let reason = format!("provider event timeout after {}ms", timeout.as_millis());
                                    let summary = agent_step_summary(
                                        &task_id,
                                        1,
                                        AgentStepStatus::Cancelled,
                                        &assistant_text,
                                        Some(reason.clone()),
                                    );
                                    append_event!(RunEvent::AgentStepCompleted { summary });
                                    return self.finish_cancelled(
                                        task_id,
                                        agent_profile_id,
                                        assistant_text,
                                        &mut context,
                                        reason,
                                        &mut event_sink,
                                    );
                                }
                            }
                        }
                    }
                }
                None if cancellation_token.is_some() || pause_token.is_some() => {
                    tokio::select! {
                        signal = next_run_control_signal(cancellation_token.as_ref(), pause_token.as_ref()) => {
                            match signal {
                                RunControlSignal::Cancelled(reason) => {
                                    let summary = agent_step_summary(
                                        &task_id,
                                        1,
                                        AgentStepStatus::Cancelled,
                                        &assistant_text,
                                        Some(reason.clone()),
                                    );
                                    append_event!(RunEvent::AgentStepCompleted { summary });
                                    return self.finish_cancelled(
                                        task_id,
                                        agent_profile_id,
                                        assistant_text,
                                        &mut context,
                                        reason,
                                        &mut event_sink,
                                    );
                                }
                                RunControlSignal::Paused(reason) => {
                                    let summary = agent_step_summary(
                                        &task_id,
                                        1,
                                        AgentStepStatus::Paused,
                                        &assistant_text,
                                        Some(reason.clone()),
                                    );
                                    append_event!(RunEvent::AgentStepCompleted { summary });
                                    return self.finish_paused(
                                        task_id,
                                        agent_profile_id,
                                        assistant_text,
                                        &mut context,
                                        reason,
                                        &mut event_sink,
                                    );
                                }
                            }
                        }
                        result = stream.try_next() => {
                            match result {
                                Ok(result) => result,
                                Err(error) => {
                                    return self.finish_failed(
                                        task_id,
                                        agent_profile_id,
                                        assistant_text,
                                        &mut context,
                                        error,
                                        &mut event_sink,
                                    );
                                }
                            }
                        }
                    }
                }
                None => match stream.try_next().await {
                    Ok(result) => result,
                    Err(error) => {
                        return self.finish_failed(
                            task_id,
                            agent_profile_id,
                            assistant_text,
                            &mut context,
                            error,
                            &mut event_sink,
                        );
                    }
                },
            };

            let Some(event) = next_event else {
                break;
            };

            let no_progress_signal = no_progress_detector.observe_event(&event);
            if let RunEvent::AssistantDelta { text, .. } = &event {
                assistant_text.push_str(text);
            }
            append_event!(event);
            if let Some(signal) = no_progress_signal {
                append_event!(RunEvent::NoProgressLoopDetected {
                    task_id: task_id.clone(),
                    signal: signal.clone(),
                });
                let summary = agent_step_summary(
                    &task_id,
                    1,
                    AgentStepStatus::StoppedNoProgress,
                    &assistant_text,
                    Some(signal.reason.clone()),
                );
                append_event!(RunEvent::AgentStepCompleted { summary });
                return self.finish_cancelled(
                    task_id,
                    agent_profile_id,
                    assistant_text,
                    &mut context,
                    format!("no progress: {}", signal.reason),
                    &mut event_sink,
                );
            }
        }

        append_event!(RunEvent::ProviderRequestCompleted {
            provider_id: request.provider_id,
        });
        let step_summary = agent_step_summary(
            &task_id,
            1,
            AgentStepStatus::Completed,
            &assistant_text,
            None,
        );
        append_event!(RunEvent::AgentStepCompleted {
            summary: step_summary,
        });
        let run_summary = agent_run_summary(
            &task_id,
            &agent_profile_id,
            TaskStatus::Completed,
            1,
            &assistant_text,
            None,
            Some(EventRange {
                start_seq: 1,
                end_seq: context.seq.saturating_sub(1),
            }),
        );
        append_event!(RunEvent::AgentRunCompleted {
            summary: run_summary.clone(),
        });
        let turn_id = context.turn_id.clone();
        append_event!(RunEvent::TurnCompleted { turn_id });
        append_event!(RunEvent::TaskCompleted {
            task_id: task_id.clone(),
        });
        append_event!(RunEvent::Done);

        Ok(AgentRunOutcome {
            trace_id,
            task_id,
            status: TaskStatus::Completed,
            summary: run_summary,
            assistant_text,
            store: self.store,
        })
    }

    fn finish_cancelled<F, R>(
        mut self,
        task_id: TaskId,
        agent_profile_id: AgentProfileId,
        assistant_text: String,
        context: &mut RunContext,
        reason: String,
        event_sink: &mut F,
    ) -> Result<AgentRunOutcome>
    where
        F: FnMut(&EventFrame) -> R,
        R: Into<EventSinkAction>,
    {
        let _ = self.append_contextual(
            context,
            RunEvent::TaskCancelled {
                task_id: task_id.clone(),
                reason: Some(reason.clone()),
            },
            event_sink,
        )?;
        let _ = self.append_contextual(context, RunEvent::Done, event_sink)?;
        let summary = agent_run_summary(
            &task_id,
            &agent_profile_id,
            TaskStatus::Cancelled,
            0,
            &assistant_text,
            Some(reason),
            Some(EventRange {
                start_seq: 1,
                end_seq: context.seq.saturating_sub(1),
            }),
        );

        Ok(AgentRunOutcome {
            trace_id: context.trace_id.clone(),
            task_id,
            status: TaskStatus::Cancelled,
            summary,
            assistant_text,
            store: self.store,
        })
    }

    fn finish_paused<F, R>(
        mut self,
        task_id: TaskId,
        agent_profile_id: AgentProfileId,
        assistant_text: String,
        context: &mut RunContext,
        reason: String,
        event_sink: &mut F,
    ) -> Result<AgentRunOutcome>
    where
        F: FnMut(&EventFrame) -> R,
        R: Into<EventSinkAction>,
    {
        let last_seq = context.seq.saturating_sub(1);
        let checkpoint = TaskPauseCheckpoint {
            checkpoint_id: TaskPauseCheckpointId::new(),
            task_id: task_id.clone(),
            trace_id: context.trace_id.clone(),
            last_seq,
            thread_id: Some(context.thread_id.clone()),
            turn_id: Some(context.turn_id.clone()),
            provider_id: context.provider_id.clone(),
            profile_id: context.profile_id.clone(),
            model: context.model.clone(),
            resume_mode: ResumeMode::FromTraceProjection,
            workspace_snapshot_id: None,
            transcript_event_range: Some(EventRange {
                start_seq: 1,
                end_seq: last_seq,
            }),
            context_handle_ids: Vec::new(),
            reason: Some(reason.clone()),
        };
        let _ = self.append_contextual(
            context,
            RunEvent::TaskPauseCheckpointCreated { checkpoint },
            event_sink,
        )?;
        let _ = self.append_contextual(
            context,
            RunEvent::TaskPaused {
                task_id: task_id.clone(),
                reason: Some(reason.clone()),
            },
            event_sink,
        )?;
        let _ = self.append_contextual(context, RunEvent::Done, event_sink)?;
        let summary = agent_run_summary(
            &task_id,
            &agent_profile_id,
            TaskStatus::Paused,
            0,
            &assistant_text,
            Some(reason),
            Some(EventRange {
                start_seq: 1,
                end_seq: context.seq.saturating_sub(1),
            }),
        );

        Ok(AgentRunOutcome {
            trace_id: context.trace_id.clone(),
            task_id,
            status: TaskStatus::Paused,
            summary,
            assistant_text,
            store: self.store,
        })
    }

    fn finish_failed<F, R>(
        mut self,
        task_id: TaskId,
        agent_profile_id: AgentProfileId,
        assistant_text: String,
        context: &mut RunContext,
        error: ProviderError,
        event_sink: &mut F,
    ) -> Result<AgentRunOutcome>
    where
        F: FnMut(&EventFrame) -> R,
        R: Into<EventSinkAction>,
    {
        let normalized = error.normalized();
        let _ = self.append_contextual(
            context,
            RunEvent::Error {
                error: normalized.clone(),
            },
            event_sink,
        )?;
        let summary = agent_step_summary(
            &task_id,
            1,
            AgentStepStatus::Failed,
            &assistant_text,
            Some(normalized.message.clone()),
        );
        let _ = self.append_contextual(
            context,
            RunEvent::AgentStepCompleted { summary },
            event_sink,
        )?;
        let _ = self.append_contextual(
            context,
            RunEvent::TaskFailed {
                task_id: task_id.clone(),
                error: normalized.clone(),
            },
            event_sink,
        )?;
        let _ = self.append_contextual(context, RunEvent::Done, event_sink)?;
        let _summary = agent_run_summary(
            &task_id,
            &agent_profile_id,
            TaskStatus::Failed,
            0,
            &assistant_text,
            Some(normalized.message),
            Some(EventRange {
                start_seq: 1,
                end_seq: context.seq.saturating_sub(1),
            }),
        );

        Err(CoreError::Provider(error))
    }

    fn append_contextual<F, R>(
        &mut self,
        context: &mut RunContext,
        event: RunEvent,
        event_sink: &mut F,
    ) -> Result<EventSinkAction>
    where
        F: FnMut(&EventFrame) -> R,
        R: Into<EventSinkAction>,
    {
        let item_id = event.item_id();
        let event_turn_id = event.turn_id();
        let event_task_id = event.task_id();
        let mut frame = EventFrame::new(&context.trace_id, context.seq, event)
            .with_thread_id(context.thread_id.clone())
            .with_turn_id(event_turn_id.unwrap_or_else(|| context.turn_id.clone()))
            .with_task_id(event_task_id.unwrap_or_else(|| context.task_id.clone()));

        if let Some(item_id) = item_id {
            frame = frame.with_item_id(item_id);
        }

        self.store.append(&frame)?;
        let action = event_sink(&frame).into();
        context.seq += 1;
        Ok(action)
    }
}

fn agent_step_summary(
    task_id: &TaskId,
    step_index: u32,
    status: AgentStepStatus,
    assistant_text: &str,
    stop_reason: Option<String>,
) -> AgentStepSummary {
    AgentStepSummary {
        task_id: task_id.clone(),
        step_index,
        status,
        assistant_text: assistant_text.to_string(),
        stop_reason,
    }
}

fn agent_run_summary(
    task_id: &TaskId,
    profile_id: &AgentProfileId,
    status: TaskStatus,
    steps_completed: u32,
    final_text: &str,
    stop_reason: Option<String>,
    evidence_event_range: Option<EventRange>,
) -> AgentRunSummary {
    AgentRunSummary {
        task_id: task_id.clone(),
        profile_id: profile_id.clone(),
        status,
        steps_completed,
        final_text: final_text.to_string(),
        stop_reason,
        evidence_event_range,
    }
}

impl<P> ConversationEngine<P>
where
    P: ChatProvider,
{
    pub fn new(provider: P, store: TraceStore) -> Self {
        Self { provider, store }
    }

    pub async fn run_chat(self, request: ConversationRequest) -> Result<ConversationOutcome> {
        self.run_chat_with_event_sink(request, |_| {}).await
    }

    pub async fn run_chat_with_event_sink<F, R>(
        self,
        request: ConversationRequest,
        event_sink: F,
    ) -> Result<ConversationOutcome>
    where
        F: FnMut(&EventFrame) -> R,
        R: Into<EventSinkAction>,
    {
        self.run_chat_with_controls_and_event_sink(request, RunControls::default(), event_sink)
            .await
    }

    pub async fn run_chat_with_controls_and_event_sink<F, R>(
        mut self,
        request: ConversationRequest,
        controls: RunControls,
        mut event_sink: F,
    ) -> Result<ConversationOutcome>
    where
        F: FnMut(&EventFrame) -> R,
        R: Into<EventSinkAction>,
    {
        let trace_id = request.trace_id.clone();
        let mut context = RunContext {
            trace_id: trace_id.clone(),
            thread_id: ThreadId::new(),
            turn_id: TurnId::new(),
            task_id: TaskId::new(),
            provider_id: request.provider_id.clone(),
            profile_id: request.profile_id.clone(),
            model: request.model.clone(),
            seq: 1,
        };
        let user_item_id = ItemId::new();
        let assistant_item_id = ItemId::new();
        let mut assistant_text = String::new();
        let mut no_progress_detector = NoProgressDetector::default();
        let provider_messages = request.provider_messages();
        let prompt = request.prompt.clone();
        let cancellation_token = controls.cancellation_token.clone();
        let pause_token = controls.pause_token.clone();

        macro_rules! append_event {
            ($event:expr) => {{
                let action = self.append_contextual(&mut context, $event, &mut event_sink)?;
                if let Some(reason) = action.cancel_reason() {
                    return self.finish_cancelled(
                        trace_id,
                        assistant_text,
                        &mut context,
                        reason,
                        &mut event_sink,
                    );
                }
            }};
        }

        let task_id = context.task_id.clone();
        append_event!(RunEvent::TaskCreated {
            task_id,
            kind: TaskKind::Chat,
        });
        let task_id = context.task_id.clone();
        append_event!(RunEvent::TaskStarted { task_id });
        let thread_id = context.thread_id.clone();
        append_event!(RunEvent::ThreadCreated { thread_id });
        let turn_id = context.turn_id.clone();
        append_event!(RunEvent::TurnStarted { turn_id });
        append_event!(RunEvent::UserMessageRecorded {
            item_id: user_item_id,
            text: prompt,
        });

        if let Some(reason) = cancellation_token
            .as_ref()
            .and_then(RunCancellationToken::cancellation_reason)
        {
            return self.finish_cancelled(
                trace_id,
                assistant_text,
                &mut context,
                reason,
                &mut event_sink,
            );
        }
        if let Some(reason) = pause_token.as_ref().and_then(RunPauseToken::pause_reason) {
            return self.finish_paused(
                trace_id,
                assistant_text,
                &mut context,
                reason,
                &mut event_sink,
            );
        }

        let capability = match self.provider.capability().await {
            Ok(capability) => capability,
            Err(error) => {
                return self.finish_failed(&mut context, error, &mut event_sink);
            }
        };

        if let Some(reason) = cancellation_token
            .as_ref()
            .and_then(RunCancellationToken::cancellation_reason)
        {
            return self.finish_cancelled(
                trace_id,
                assistant_text,
                &mut context,
                reason,
                &mut event_sink,
            );
        }
        if let Some(reason) = pause_token.as_ref().and_then(RunPauseToken::pause_reason) {
            return self.finish_paused(
                trace_id,
                assistant_text,
                &mut context,
                reason,
                &mut event_sink,
            );
        }

        let route_decision = ModelRouter::draft().route(ModelRouteRequest {
            requested_profile: Some(request.profile_id.clone()),
            default_profile: request.profile_id.clone(),
            requested_model: request.model.clone(),
            reasoning_level: None,
            provider_capability: Some(capability.clone()),
        });
        let selected_profile = route_decision.selected_profile.clone();
        let selected_model = route_decision.selected_model.clone();
        context.profile_id = selected_profile.clone();
        context.model = selected_model.clone();

        append_event!(RunEvent::ProviderCapabilityReported {
            provider_id: request.provider_id.clone(),
            capability,
        });
        append_event!(RunEvent::RouteDecisionRecorded {
            decision_id: RouteDecisionId::new(),
            decision: route_decision,
        });
        append_event!(RunEvent::ProviderRequestStarted {
            provider_id: request.provider_id.clone(),
            profile_id: selected_profile.clone(),
            model: selected_model.clone(),
        });

        let mut stream = match self
            .provider
            .stream_chat(ProviderRequest {
                provider_id: request.provider_id.clone(),
                profile_id: selected_profile,
                model: selected_model,
                prompt: request.prompt,
                messages: provider_messages,
                assistant_item_id,
            })
            .await
        {
            Ok(stream) => stream,
            Err(error) => {
                return self.finish_failed(&mut context, error, &mut event_sink);
            }
        };

        loop {
            if let Some(reason) = cancellation_token
                .as_ref()
                .and_then(RunCancellationToken::cancellation_reason)
            {
                return self.finish_cancelled(
                    trace_id,
                    assistant_text,
                    &mut context,
                    reason,
                    &mut event_sink,
                );
            }
            if let Some(reason) = pause_token.as_ref().and_then(RunPauseToken::pause_reason) {
                return self.finish_paused(
                    trace_id,
                    assistant_text,
                    &mut context,
                    reason,
                    &mut event_sink,
                );
            }

            let next_event = match controls.event_timeout {
                Some(timeout) => {
                    tokio::select! {
                        signal = next_run_control_signal(cancellation_token.as_ref(), pause_token.as_ref()) => {
                            match signal {
                                RunControlSignal::Cancelled(reason) => {
                                    return self.finish_cancelled(
                                        trace_id,
                                        assistant_text,
                                        &mut context,
                                        reason,
                                        &mut event_sink,
                                    );
                                }
                                RunControlSignal::Paused(reason) => {
                                    return self.finish_paused(
                                        trace_id,
                                        assistant_text,
                                        &mut context,
                                        reason,
                                        &mut event_sink,
                                    );
                                }
                            }
                        }
                        timed = tokio::time::timeout(timeout, stream.try_next()) => {
                            match timed {
                                Ok(Ok(result)) => result,
                                Ok(Err(error)) => {
                                    return self.finish_failed(&mut context, error, &mut event_sink);
                                }
                                Err(_) => {
                                    return self.finish_cancelled(
                                        trace_id,
                                        assistant_text,
                                        &mut context,
                                        format!("provider event timeout after {}ms", timeout.as_millis()),
                                        &mut event_sink,
                                    );
                                }
                            }
                        }
                    }
                }
                None if cancellation_token.is_some() || pause_token.is_some() => {
                    tokio::select! {
                        signal = next_run_control_signal(cancellation_token.as_ref(), pause_token.as_ref()) => {
                            match signal {
                                RunControlSignal::Cancelled(reason) => {
                                    return self.finish_cancelled(
                                        trace_id,
                                        assistant_text,
                                        &mut context,
                                        reason,
                                        &mut event_sink,
                                    );
                                }
                                RunControlSignal::Paused(reason) => {
                                    return self.finish_paused(
                                        trace_id,
                                        assistant_text,
                                        &mut context,
                                        reason,
                                        &mut event_sink,
                                    );
                                }
                            }
                        }
                        result = stream.try_next() => {
                            match result {
                                Ok(result) => result,
                                Err(error) => {
                                    return self.finish_failed(&mut context, error, &mut event_sink);
                                }
                            }
                        }
                    }
                }
                None => match stream.try_next().await {
                    Ok(result) => result,
                    Err(error) => {
                        return self.finish_failed(&mut context, error, &mut event_sink);
                    }
                },
            };

            let Some(event) = next_event else {
                break;
            };

            let no_progress_signal = no_progress_detector.observe_event(&event);
            if let RunEvent::AssistantDelta { text, .. } = &event {
                assistant_text.push_str(text);
            }
            append_event!(event);
            if let Some(signal) = no_progress_signal {
                let task_id = context.task_id.clone();
                append_event!(RunEvent::NoProgressLoopDetected {
                    task_id,
                    signal: signal.clone(),
                });
                return self.finish_cancelled(
                    trace_id,
                    assistant_text,
                    &mut context,
                    format!("no progress: {}", signal.reason),
                    &mut event_sink,
                );
            }
        }

        append_event!(RunEvent::ProviderRequestCompleted {
            provider_id: request.provider_id,
        });
        let turn_id = context.turn_id.clone();
        append_event!(RunEvent::TurnCompleted { turn_id });
        let task_id = context.task_id.clone();
        append_event!(RunEvent::TaskCompleted { task_id });
        append_event!(RunEvent::Done);

        Ok(ConversationOutcome {
            trace_id,
            assistant_text,
            store: self.store,
        })
    }

    fn finish_cancelled<F, R>(
        mut self,
        trace_id: String,
        assistant_text: String,
        context: &mut RunContext,
        reason: String,
        event_sink: &mut F,
    ) -> Result<ConversationOutcome>
    where
        F: FnMut(&EventFrame) -> R,
        R: Into<EventSinkAction>,
    {
        let task_id = context.task_id.clone();
        let _ = self.append_contextual(
            context,
            RunEvent::TaskCancelled {
                task_id,
                reason: Some(reason),
            },
            event_sink,
        )?;
        let _ = self.append_contextual(context, RunEvent::Done, event_sink)?;

        Ok(ConversationOutcome {
            trace_id,
            assistant_text,
            store: self.store,
        })
    }

    fn finish_paused<F, R>(
        mut self,
        trace_id: String,
        assistant_text: String,
        context: &mut RunContext,
        reason: String,
        event_sink: &mut F,
    ) -> Result<ConversationOutcome>
    where
        F: FnMut(&EventFrame) -> R,
        R: Into<EventSinkAction>,
    {
        let last_seq = context.seq.saturating_sub(1);
        let task_id = context.task_id.clone();
        let checkpoint = TaskPauseCheckpoint {
            checkpoint_id: TaskPauseCheckpointId::new(),
            task_id: task_id.clone(),
            trace_id: context.trace_id.clone(),
            last_seq,
            thread_id: Some(context.thread_id.clone()),
            turn_id: Some(context.turn_id.clone()),
            provider_id: context.provider_id.clone(),
            profile_id: context.profile_id.clone(),
            model: context.model.clone(),
            resume_mode: ResumeMode::FromTraceProjection,
            workspace_snapshot_id: None,
            transcript_event_range: Some(EventRange {
                start_seq: 1,
                end_seq: last_seq,
            }),
            context_handle_ids: Vec::new(),
            reason: Some(reason.clone()),
        };
        let _ = self.append_contextual(
            context,
            RunEvent::TaskPauseCheckpointCreated { checkpoint },
            event_sink,
        )?;
        let _ = self.append_contextual(
            context,
            RunEvent::TaskPaused {
                task_id,
                reason: Some(reason),
            },
            event_sink,
        )?;
        let _ = self.append_contextual(context, RunEvent::Done, event_sink)?;

        Ok(ConversationOutcome {
            trace_id,
            assistant_text,
            store: self.store,
        })
    }

    fn finish_failed<F, R>(
        mut self,
        context: &mut RunContext,
        error: ProviderError,
        event_sink: &mut F,
    ) -> Result<ConversationOutcome>
    where
        F: FnMut(&EventFrame) -> R,
        R: Into<EventSinkAction>,
    {
        let normalized = error.normalized();
        let _ = self.append_contextual(
            context,
            RunEvent::Error {
                error: normalized.clone(),
            },
            event_sink,
        )?;
        let task_id = context.task_id.clone();
        let _ = self.append_contextual(
            context,
            RunEvent::TaskFailed {
                task_id,
                error: normalized,
            },
            event_sink,
        )?;
        let _ = self.append_contextual(context, RunEvent::Done, event_sink)?;

        Err(CoreError::Provider(error))
    }

    fn append_contextual<F, R>(
        &mut self,
        context: &mut RunContext,
        event: RunEvent,
        event_sink: &mut F,
    ) -> Result<EventSinkAction>
    where
        F: FnMut(&EventFrame) -> R,
        R: Into<EventSinkAction>,
    {
        let item_id = event.item_id();
        let event_turn_id = event.turn_id();
        let event_task_id = event.task_id();
        let mut frame = EventFrame::new(&context.trace_id, context.seq, event)
            .with_thread_id(context.thread_id.clone())
            .with_turn_id(event_turn_id.unwrap_or_else(|| context.turn_id.clone()))
            .with_task_id(event_task_id.unwrap_or_else(|| context.task_id.clone()));

        if let Some(item_id) = item_id {
            frame = frame.with_item_id(item_id);
        }

        self.store.append(&frame)?;
        let action = event_sink(&frame).into();
        context.seq += 1;
        Ok(action)
    }
}
