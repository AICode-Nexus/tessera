use futures::TryStreamExt;
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tessera_protocol::{
    AgentProfile, AgentProfileId, ArtifactId, ArtifactKind, ContextBudget, ContextId,
    ContextPlacement, ContextReference, Diagnostic, DiagnosticReport, DiagnosticReportId,
    EventFrame, EventRange, ExtensionMap, ItemId, ModelProfileId, NoProgressAction, NoProgressLoop,
    NoProgressSignalKind, OsSandboxFilesystem, OsSandboxMode, OsSandboxNetwork, OsSandboxProfile,
    OsSandboxProfileId, OsSandboxShell, PolicyDecisionId, PolicyOutcome, ProviderCapability,
    ProviderId, ResumeMode, RouteDecision, RouteDecisionId, RouteStrategy, RunEvent,
    SandboxDecision, SandboxDecisionId, SandboxDecisionKind, SkillId, SkillManifest, SnapshotId,
    SnapshotKind, TaskId, TaskKind, TaskPauseCheckpoint, TaskPauseCheckpointId, TaskStatus,
    ThreadId, Timestamp, ToolCallRequest, ToolDescriptor, ToolDispatch, ToolId, ToolPermission,
    ToolPolicyDecision, ToolRepairId, ToolRepairKind, ToolRepairReport, ToolResult, ToolSideEffect,
    TraceRecord, TurnId, WorkspaceAccess, WorkspaceCheckpoint, WorkspaceGuardrail, WorkspaceScope,
};
use tessera_providers::{ChatProvider, ProviderError, ProviderMessage, ProviderRequest};
use tessera_storage::TraceStore;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
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
