use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

pub const SCHEMA_VERSION: u32 = 1;
pub const RUNTIME_API_PROTOCOL_VERSION: &str = "v0";

pub type ExtensionMap = BTreeMap<String, Value>;

macro_rules! id_type {
    ($name:ident, $prefix:literal) => {
        #[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
        #[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
        #[serde(transparent)]
        #[cfg_attr(feature = "bindings", ts(type = "string"))]
        pub struct $name(String);

        impl $name {
            pub fn new() -> Self {
                Self(format!("{}_{}", $prefix, Uuid::new_v4().simple()))
            }

            pub fn from_static(value: &'static str) -> Self {
                Self(value.to_string())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_string())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

id_type!(ThreadId, "thread");
id_type!(TurnId, "turn");
id_type!(ItemId, "item");
id_type!(TaskId, "task");
id_type!(TaskPauseCheckpointId, "task_pause_checkpoint");
id_type!(RuntimeInstanceId, "runtime");
id_type!(ClientInstanceId, "client");
id_type!(TaskOwnershipId, "task_owner");
id_type!(ArtifactId, "artifact");
id_type!(EventId, "evt");
id_type!(ProviderId, "provider");
id_type!(ModelProfileId, "profile");
id_type!(WindowId, "window");
id_type!(RouteDecisionId, "route");
id_type!(SkillId, "skill");
id_type!(ToolId, "tool");
id_type!(ToolCallId, "tool_call");
id_type!(ToolDispatchId, "tool_dispatch");
id_type!(ToolResultId, "tool_result");
id_type!(ToolRepairId, "tool_repair");
id_type!(ApprovalId, "approval");
id_type!(PolicyDecisionId, "policy");
id_type!(SandboxDecisionId, "sandbox");
id_type!(OsSandboxProfileId, "os_sandbox");
id_type!(SnapshotId, "snapshot");
id_type!(ContextId, "context");
id_type!(DiagnosticReportId, "diagnostics");
id_type!(MemoryProposalId, "memory_proposal");
id_type!(AgentProfileId, "agent_profile");
id_type!(AgentHandoffId, "agent_handoff");
id_type!(ReviewerGateId, "reviewer_gate");
id_type!(SubagentSessionId, "subagent_session");
id_type!(CodingWorkflowId, "coding_workflow");
id_type!(PatchProposalId, "patch_proposal");
id_type!(MutationRequestId, "mutation_request");
id_type!(WorkspaceWorktreeId, "workspace_worktree");
id_type!(ApplyPatchPreflightId, "apply_patch_preflight");
id_type!(ApplyPatchExecutionId, "apply_patch_execution");
id_type!(TestPlanId, "test_plan");
id_type!(TestRunId, "test_run");
id_type!(ReviewBundleId, "review_bundle");
id_type!(RestorePlanId, "restore_plan");

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(transparent)]
#[cfg_attr(feature = "bindings", ts(type = "string"))]
pub struct Timestamp(String);

impl Timestamp {
    pub fn now_utc() -> Self {
        let timestamp = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
        Self(timestamp)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderCapability {
    pub provider_id: ProviderId,
    pub supports_streaming: bool,
    pub supports_reasoning_delta: bool,
    pub supports_cache_telemetry: bool,
    pub supports_cost_estimate: bool,
    pub supports_tool_calling: bool,
    pub max_context_tokens: Option<u64>,
    pub extension: Option<ExtensionMap>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadStatus {
    Active,
    Archived,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Thread {
    pub id: ThreadId,
    pub title: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub active_model_profile: Option<ModelProfileId>,
    pub status: ThreadStatus,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Turn {
    pub id: TurnId,
    pub thread_id: ThreadId,
    pub started_at: Timestamp,
    pub completed_at: Option<Timestamp>,
    pub status: TurnStatus,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    UserMessage,
    AssistantMessage,
    ProviderEvent,
    Usage,
    Error,
    ArtifactRef,
    ToolCall,
    ToolResult,
    Approval,
    MemoryRecall,
    MemoryProposal,
    SkillEvent,
    AgentEvent,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    Created,
    Streaming,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub thread_id: ThreadId,
    pub turn_id: Option<TurnId>,
    pub kind: ItemKind,
    pub status: ItemStatus,
    pub created_at: Timestamp,
    pub completed_at: Option<Timestamp>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    Chat,
    Replay,
    ToolRun,
    AgentRun,
    MultiAgentRun,
    SwarmRun,
    LearningJob,
}

impl TaskKind {
    pub fn from_snake_case(value: &str) -> Option<Self> {
        match value {
            "chat" => Some(Self::Chat),
            "replay" => Some(Self::Replay),
            "tool_run" => Some(Self::ToolRun),
            "agent_run" => Some(Self::AgentRun),
            "multi_agent_run" => Some(Self::MultiAgentRun),
            "swarm_run" => Some(Self::SwarmRun),
            "learning_job" => Some(Self::LearningJob),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    WaitingForApproval,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub thread_id: Option<ThreadId>,
    pub turn_id: Option<TurnId>,
    pub kind: TaskKind,
    pub status: TaskStatus,
    pub created_at: Timestamp,
    pub started_at: Option<Timestamp>,
    pub completed_at: Option<Timestamp>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct EventRange {
    pub start_seq: u64,
    pub end_seq: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ResumeMode {
    BeforeProviderRequest,
    AfterCompletedProviderTurn,
    FromTraceProjection,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct TaskPauseCheckpoint {
    pub checkpoint_id: TaskPauseCheckpointId,
    pub task_id: TaskId,
    pub trace_id: String,
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TaskOwnerKind {
    Execution,
    Observer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TaskOwnerStatus {
    Attached,
    Heartbeat,
    Detached,
    Lost,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TaskReattachMode {
    ObserveExistingOwner,
    ResumeFromCheckpoint,
    TerminalProjection,
    OwnerLost,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct RuntimeInstance {
    pub runtime_id: RuntimeInstanceId,
    pub process_id: Option<u32>,
    pub started_at: Timestamp,
    pub hostname: Option<String>,
    pub working_directory: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum RuntimeApiBindKind {
    LocalhostTcp,
    UnixSocket,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct RuntimeApiBindConfig {
    pub kind: RuntimeApiBindKind,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub socket_path: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum RuntimeApiAuthMode {
    LoopbackDevToken,
    OsUserSession,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct RuntimeApiAuthPolicy {
    pub mode: RuntimeApiAuthMode,
    pub token_required: bool,
    pub token_source_label: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum RuntimeApiQueueOverflow {
    RejectNew,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct RuntimeApiQueuePolicy {
    pub event_buffer_capacity: usize,
    pub client_buffer_capacity: usize,
    pub overflow: RuntimeApiQueueOverflow,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct RuntimeApiServerConfig {
    pub version: String,
    pub bind: RuntimeApiBindConfig,
    pub auth: RuntimeApiAuthPolicy,
    pub queue: RuntimeApiQueuePolicy,
}

impl RuntimeApiServerConfig {
    pub fn localhost_default() -> Self {
        Self {
            version: RUNTIME_API_PROTOCOL_VERSION.to_string(),
            bind: RuntimeApiBindConfig {
                kind: RuntimeApiBindKind::LocalhostTcp,
                host: Some("127.0.0.1".to_string()),
                port: None,
                socket_path: None,
            },
            auth: RuntimeApiAuthPolicy {
                mode: RuntimeApiAuthMode::LoopbackDevToken,
                token_required: true,
                token_source_label: Some("TESSERA_RUNTIME_API_TOKEN".to_string()),
            },
            queue: RuntimeApiQueuePolicy {
                event_buffer_capacity: 1024,
                client_buffer_capacity: 128,
                overflow: RuntimeApiQueueOverflow::RejectNew,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct RuntimeApiEventStreamRequest {
    pub trace_id: String,
    pub since_seq: Option<u64>,
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case", tag = "command", content = "payload")]
pub enum RuntimeApiCommand {
    ListEvents(RuntimeApiEventStreamRequest),
    SubscribeEvents(RuntimeApiEventStreamRequest),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct RuntimeApiCommandEnvelope {
    pub command_id: String,
    pub client_id: Option<ClientInstanceId>,
    pub trace_id: Option<String>,
    pub since_seq: Option<u64>,
    #[serde(flatten)]
    pub command: RuntimeApiCommand,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum RuntimeApiCommandStatus {
    Accepted,
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct RuntimeApiCommandAck {
    pub command_id: String,
    pub status: RuntimeApiCommandStatus,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct TaskOwnerLease {
    pub lease_id: TaskOwnershipId,
    pub task_id: TaskId,
    pub trace_id: String,
    pub runtime_id: RuntimeInstanceId,
    pub client_id: Option<ClientInstanceId>,
    pub owner_kind: TaskOwnerKind,
    pub status: TaskOwnerStatus,
    pub acquired_at: Timestamp,
    pub heartbeat_interval_ms: u64,
    pub expires_at: Option<Timestamp>,
    pub last_heartbeat_at: Option<Timestamp>,
    pub last_seq: Option<u64>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct TaskOwnerHeartbeat {
    pub lease_id: TaskOwnershipId,
    pub task_id: TaskId,
    pub runtime_id: RuntimeInstanceId,
    pub heartbeat_at: Timestamp,
    pub expires_at: Timestamp,
    pub last_seq: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct TaskReattachRecord {
    pub task_id: TaskId,
    pub mode: TaskReattachMode,
    pub previous_lease_id: Option<TaskOwnershipId>,
    pub new_lease_id: Option<TaskOwnershipId>,
    pub checkpoint_id: Option<TaskPauseCheckpointId>,
    pub since_seq: Option<u64>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Trace,
    Export,
    ProviderRawMetadata,
    ToolOutput,
    Patch,
    TestReport,
    AgentTranscript,
}

impl ArtifactKind {
    pub fn from_snake_case(value: &str) -> Option<Self> {
        match value {
            "trace" => Some(Self::Trace),
            "export" => Some(Self::Export),
            "provider_raw_metadata" => Some(Self::ProviderRawMetadata),
            "tool_output" => Some(Self::ToolOutput),
            "patch" => Some(Self::Patch),
            "test_report" => Some(Self::TestReport),
            "agent_transcript" => Some(Self::AgentTranscript),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ArtifactBodyRedactionStatus {
    Clean,
    Redacted,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ArtifactBodyRecord {
    pub artifact_id: ArtifactId,
    pub kind: ArtifactKind,
    pub task_id: Option<TaskId>,
    pub media_type: String,
    pub byte_len: u64,
    pub storage_uri: String,
    pub redaction_status: ArtifactBodyRedactionStatus,
    pub summary: Option<String>,
    pub metadata: Option<ExtensionMap>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Artifact {
    pub id: ArtifactId,
    pub thread_id: Option<ThreadId>,
    pub turn_id: Option<TurnId>,
    pub task_id: Option<TaskId>,
    pub kind: ArtifactKind,
    pub uri: String,
    pub media_type: Option<String>,
    pub size_bytes: Option<u64>,
    pub created_at: Timestamp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticRange {
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: u32,
    pub end_character: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub code: Option<String>,
    pub message: String,
    pub uri: Option<String>,
    pub range: Option<DiagnosticRange>,
    pub metadata: Option<ExtensionMap>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub report_id: DiagnosticReportId,
    pub source: String,
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
    pub metadata: Option<ExtensionMap>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryProposalStatus {
    Pending,
    Applied,
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemoryProposal {
    pub proposal_id: MemoryProposalId,
    pub status: MemoryProposalStatus,
    pub title: String,
    pub summary: String,
    pub source_item_id: Option<ItemId>,
    pub reason: Option<String>,
    pub metadata: Option<ExtensionMap>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextSourceKind {
    File,
    Directory,
    Workspace,
    Artifact,
    Trace,
    Inline,
    Url,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextSource {
    pub kind: ContextSourceKind,
    pub uri: Option<String>,
    pub label: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextPlacement {
    StablePrefix,
    AppendOnlyTranscript,
    VolatileScratch,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextReference {
    pub id: ContextId,
    pub source: ContextSource,
    pub placement: ContextPlacement,
    pub estimated_tokens: u64,
    pub pinned: bool,
    pub summary: Option<String>,
    pub metadata: Option<ExtensionMap>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstructionSourceKind {
    AgentsMd,
    ClaudeMd,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstructionLoadStatus {
    Loaded,
    SkippedLowerPrecedence,
    SkippedSymlink,
    SkippedOutsideWorkspace,
    SkippedNonUtf8,
    SkippedTooLarge,
    ReadFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstructionRedactionStatus {
    Clean,
    Redacted,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InstructionSource {
    pub source_id: ContextId,
    pub kind: InstructionSourceKind,
    pub path: String,
    pub relative_path: String,
    pub precedence: u32,
    pub placement: ContextPlacement,
    pub status: InstructionLoadStatus,
    pub original_bytes: u64,
    pub loaded_bytes: u64,
    pub sha256: Option<String>,
    pub redaction_status: InstructionRedactionStatus,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextBudget {
    pub max_tokens: u64,
    pub reserved_output_tokens: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillSourceKind {
    BuiltIn,
    Workspace,
    User,
    Bundled,
    External,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillSource {
    pub kind: SkillSourceKind,
    pub uri: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillEntrypointFormat {
    SkillMd,
    SkillToml,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillEntrypoint {
    pub format: SkillEntrypointFormat,
    pub path: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillRequirements {
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub context: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillPolicy {
    pub default_permission: String,
    pub network: String,
    pub write_files: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillManifest {
    pub id: SkillId,
    pub name: String,
    pub version: Option<String>,
    pub description: String,
    pub source: SkillSource,
    pub entrypoint: SkillEntrypoint,
    pub requirements: SkillRequirements,
    pub policy: SkillPolicy,
    pub metadata: Option<ExtensionMap>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillLoadStatus {
    Loaded,
    SkippedDuplicate,
    SkippedSymlink,
    SkippedOutsideWorkspace,
    SkippedNonUtf8,
    SkippedTooLarge,
    InvalidManifest,
    ReadFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillActivationStatus {
    Activated,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillRedactionStatus {
    Clean,
    Redacted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillStepKind {
    DiscoverEntrypoint,
    LoadEntrypoint,
    LoadReference,
    RenderContext,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillStepStatus {
    Completed,
    Skipped,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillReferenceSource {
    pub source_id: ContextId,
    pub path: String,
    pub relative_path: String,
    pub status: SkillLoadStatus,
    pub original_bytes: u64,
    pub loaded_bytes: u64,
    pub sha256: Option<String>,
    pub redaction_status: SkillRedactionStatus,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillActivationStep {
    pub step_index: u32,
    pub kind: SkillStepKind,
    pub status: SkillStepStatus,
    pub source_id: Option<ContextId>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillActivation {
    pub task_id: TaskId,
    pub skill_id: SkillId,
    pub manifest: SkillManifest,
    pub status: SkillActivationStatus,
    pub entrypoint: SkillReferenceSource,
    #[serde(default)]
    pub references: Vec<SkillReferenceSource>,
    #[serde(default)]
    pub steps: Vec<SkillActivationStep>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentProfile {
    pub id: AgentProfileId,
    pub name: String,
    pub role: String,
    pub model_profile: ModelProfileId,
    #[serde(default)]
    pub skills: Vec<SkillId>,
    #[serde(default)]
    pub memory_scopes: Vec<String>,
    #[serde(default)]
    pub context_scopes: Vec<String>,
    #[serde(default)]
    pub tool_permissions: Vec<ToolPermission>,
    pub max_steps: u32,
    pub metadata: Option<ExtensionMap>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum AgentStepStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
    Paused,
    StoppedNoProgress,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct AgentStepSummary {
    pub task_id: TaskId,
    pub step_index: u32,
    pub status: AgentStepStatus,
    pub assistant_text: String,
    pub stop_reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct AgentRunSummary {
    pub task_id: TaskId,
    pub profile_id: AgentProfileId,
    pub status: TaskStatus,
    pub steps_completed: u32,
    pub final_text: String,
    pub stop_reason: Option<String>,
    pub evidence_event_range: Option<EventRange>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum AgentHandoffStatus {
    Completed,
    Failed,
    Paused,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum HandoffEvidenceKind {
    TraceRange,
    TranscriptArtifact,
    SummaryArtifact,
    DiffArtifact,
    DiagnosticArtifact,
    TestOutputArtifact,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct HandoffEvidenceRef {
    pub kind: HandoffEvidenceKind,
    pub artifact_id: Option<ArtifactId>,
    pub trace_id: Option<String>,
    pub event_range: Option<EventRange>,
    pub label: Option<String>,
    pub summary: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct AgentHandoffMetrics {
    pub steps_completed: u32,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub estimated_cost: Option<CostEstimate>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct AgentHandoffSummary {
    pub handoff_id: AgentHandoffId,
    pub parent_task_id: TaskId,
    pub child_task_id: Option<TaskId>,
    pub status: AgentHandoffStatus,
    pub objective: String,
    pub summary: String,
    #[serde(default)]
    pub evidence: Vec<HandoffEvidenceRef>,
    pub metrics: AgentHandoffMetrics,
    pub evidence_event_range: Option<EventRange>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ReviewerDecisionKind {
    Accept,
    Reject,
    RequestRevision,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ReviewerGateRequest {
    pub gate_id: ReviewerGateId,
    pub handoff_id: AgentHandoffId,
    pub parent_task_id: TaskId,
    #[serde(default)]
    pub requested_decisions: Vec<ReviewerDecisionKind>,
    #[serde(default)]
    pub evidence: Vec<HandoffEvidenceRef>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ReviewerGateDecision {
    pub gate_id: ReviewerGateId,
    pub handoff_id: AgentHandoffId,
    pub decision: ReviewerDecisionKind,
    pub reviewer: String,
    pub reason_code: String,
    pub comment: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum MutationMode {
    WorktreeFirst,
    ExplicitLocal,
    ReadOnlyProposal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PatchApplicationOutcome {
    Planned,
    Applied,
    Conflict,
    Rejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TestRunStatus {
    Planned,
    Passed,
    Failed,
    Cancelled,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum CodingWorkflowEvidenceRedactionStatus {
    Clean,
    Redacted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum MutationRequestOperationKind {
    PatchApplication,
    TestRun,
    CheckpointRestore,
    VersionControlStage,
    VersionControlCommit,
    VersionControlPush,
    PullRequestPublication,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum MutationRequestStatus {
    Proposed,
    PolicyPending,
    PolicyBlocked,
    ReviewerPending,
    Approved,
    Rejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ApplyPatchPreflightStatus {
    Blocked,
    DryRunReady,
    ExecutorReady,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ApplyPatchPreflightBlocker {
    MissingMutationRequest,
    MissingPatchProposal,
    OperationMismatch,
    MutationRequestNotApproved,
    ScopeMismatch,
    WorktreeIsolationRequired,
    MissingPolicyDecision,
    PolicyNotAllowed,
    MissingReviewerDecision,
    ReviewerNotAccepted,
    MissingCheckpointLifecycle,
    CheckpointNotCreated,
    MissingSandboxProfile,
    MissingPatchBody,
    PatchBodyTooLarge,
    UnsafePatchPath,
    UnsupportedPatchOperation,
    ExecutorUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ApplyPatchDryRunOperationKind {
    Create,
    Modify,
    DeleteUnsupported,
    RenameUnsupported,
    BinaryUnsupported,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ApplyPatchDryRunOperationSummary {
    pub path: String,
    pub operation: ApplyPatchDryRunOperationKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ApplyPatchPreflightRecord {
    pub preflight_id: ApplyPatchPreflightId,
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    pub request_id: MutationRequestId,
    pub patch_id: PatchProposalId,
    pub status: ApplyPatchPreflightStatus,
    #[serde(default)]
    pub blockers: Vec<ApplyPatchPreflightBlocker>,
    #[serde(default)]
    pub affected_paths: Vec<String>,
    #[serde(default)]
    pub operations: Vec<ApplyPatchDryRunOperationSummary>,
    pub executor_blocked: bool,
    pub executor_block_reason: String,
    #[serde(default)]
    pub evidence: Vec<HandoffEvidenceRef>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ApplyPatchExecutionStatus {
    Planned,
    Applied,
    Conflict,
    Rejected,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ApplyPatchExecutionBlocker {
    PreflightNotReady,
    ExecutorUnavailable,
    PrimaryRootRejected,
    MissingCheckpointLifecycle,
    CheckpointNotCreated,
    MissingPolicyDecision,
    PolicyNotAllowed,
    MissingReviewerDecision,
    ReviewerNotAccepted,
    MissingSandboxProfile,
    MissingIsolatedRoot,
    UnsafePath,
    SymlinkRejected,
    UnsupportedPatchOperation,
    MultipleFilesUnsupported,
    HunkConflict,
    WriteFailed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ApplyPatchExecutionRecord {
    pub execution_id: ApplyPatchExecutionId,
    pub preflight_id: ApplyPatchPreflightId,
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    pub request_id: MutationRequestId,
    pub patch_id: PatchProposalId,
    pub checkpoint_id: Option<SnapshotId>,
    pub reviewer_gate_id: Option<ReviewerGateId>,
    pub policy_decision_id: Option<PolicyDecisionId>,
    pub sandbox_profile_label: Option<String>,
    pub isolated_root_label: String,
    pub executor_label: String,
    pub status: ApplyPatchExecutionStatus,
    #[serde(default)]
    pub blockers: Vec<ApplyPatchExecutionBlocker>,
    #[serde(default)]
    pub affected_paths: Vec<String>,
    #[serde(default)]
    pub conflict_paths: Vec<String>,
    #[serde(default)]
    pub artifact_refs: Vec<ArtifactId>,
    #[serde(default)]
    pub evidence: Vec<HandoffEvidenceRef>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceWorktreeLifecycleStatus {
    Planned,
    Created,
    CreationFailed,
    Retained,
    CleanupStarted,
    CleanupCompleted,
    CleanupFailed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct WorkspaceWorktreeLifecycleRecord {
    pub worktree_id: WorkspaceWorktreeId,
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    pub trace_id: String,
    pub source_commit: String,
    pub source_branch_label: Option<String>,
    pub worktree_root_label: String,
    pub worktree_base_key: String,
    pub lifecycle_status: WorkspaceWorktreeLifecycleStatus,
    pub reason: String,
    pub created_for_request_id: Option<MutationRequestId>,
    pub created_for_patch_id: Option<PatchProposalId>,
    #[serde(default)]
    pub evidence: Vec<HandoffEvidenceRef>,
    pub metadata: Option<ExtensionMap>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct WorkspaceMutationScope {
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    pub root_label: String,
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    #[serde(default)]
    pub denied_paths: Vec<String>,
    pub mutation_mode: MutationMode,
    pub worktree_required: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct MutationRequestProposal {
    pub request_id: MutationRequestId,
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    pub operation: MutationRequestOperationKind,
    pub status: MutationRequestStatus,
    pub summary: String,
    #[serde(default)]
    pub requested_paths: Vec<String>,
    pub required_checkpoint_id: Option<SnapshotId>,
    pub reviewer_gate_id: Option<ReviewerGateId>,
    pub policy_decision_id: Option<PolicyDecisionId>,
    pub sandbox_profile_label: Option<String>,
    pub worktree_required: bool,
    #[serde(default)]
    pub evidence: Vec<HandoffEvidenceRef>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct PatchProposal {
    pub patch_id: PatchProposalId,
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    pub summary: String,
    #[serde(default)]
    pub touched_paths: Vec<String>,
    #[serde(default)]
    pub diff_artifacts: Vec<HandoffEvidenceRef>,
    #[serde(default)]
    pub risk_labels: Vec<String>,
    pub required_checkpoint_id: Option<SnapshotId>,
    pub reviewer_gate_id: Option<ReviewerGateId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct PatchApplicationRecord {
    pub patch_id: PatchProposalId,
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    pub checkpoint_id: Option<SnapshotId>,
    pub outcome: PatchApplicationOutcome,
    #[serde(default)]
    pub conflict_paths: Vec<String>,
    #[serde(default)]
    pub applied_paths: Vec<String>,
    #[serde(default)]
    pub artifact_refs: Vec<ArtifactId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct TestPlanRecord {
    pub test_plan_id: TestPlanId,
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    #[serde(default)]
    pub command_labels: Vec<String>,
    #[serde(default)]
    pub affected_paths: Vec<String>,
    #[serde(default)]
    pub required_artifact_kinds: Vec<ArtifactKind>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct TestRunRecord {
    pub test_run_id: TestRunId,
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    pub test_plan_id: Option<TestPlanId>,
    pub command_label: String,
    pub status: TestRunStatus,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
    pub stdout_artifact_id: Option<ArtifactId>,
    pub stderr_artifact_id: Option<ArtifactId>,
    #[serde(default)]
    pub diagnostics: Vec<HandoffEvidenceRef>,
    pub redaction_status: CodingWorkflowEvidenceRedactionStatus,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ReviewBundle {
    pub review_bundle_id: ReviewBundleId,
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    pub reviewer_gate_id: ReviewerGateId,
    #[serde(default)]
    pub patch_ids: Vec<PatchProposalId>,
    #[serde(default)]
    pub test_run_ids: Vec<TestRunId>,
    #[serde(default)]
    pub evidence: Vec<HandoffEvidenceRef>,
    pub summary: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct RestorePlanRecord {
    pub restore_plan_id: RestorePlanId,
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    pub checkpoint_id: SnapshotId,
    #[serde(default)]
    pub target_paths: Vec<String>,
    pub reason: String,
    pub execution_blocked: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SubagentSessionStatus {
    Planned,
    Active,
    WaitingForApproval,
    Inactive,
    Completed,
    Failed,
    Cancelled,
    HandedOff,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SubagentInactivePolicy {
    PauseParent,
    QueueDecision,
    RequireReviewer,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct SubagentSessionCaps {
    pub max_steps: u32,
    pub max_depth: u32,
    pub timeout_ms: Option<u64>,
    pub max_child_sessions: u32,
    pub max_estimated_cost: Option<CostEstimate>,
    pub concurrency_slot: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct SubagentApprovalForwarding {
    pub inactive_policy: SubagentInactivePolicy,
    pub reviewer_gate_id: Option<ReviewerGateId>,
    pub approval_id: Option<ApprovalId>,
    pub forwarded_from_parent: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct SubagentSessionDescriptor {
    pub session_id: SubagentSessionId,
    pub parent_task_id: TaskId,
    pub child_task_id: Option<TaskId>,
    pub profile_id: AgentProfileId,
    pub objective: String,
    pub status: SubagentSessionStatus,
    #[serde(default)]
    pub scope_labels: Vec<String>,
    #[serde(default)]
    pub tool_permission_labels: Vec<String>,
    #[serde(default)]
    pub memory_scope_labels: Vec<String>,
    pub transcript_artifact_id: Option<ArtifactId>,
    pub caps: SubagentSessionCaps,
    pub approval_forwarding: Option<SubagentApprovalForwarding>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SubagentRuntimeDecisionKind {
    StartAllowed,
    StartDenied,
    QueueOnly,
    RequireReviewer,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct SubagentRuntimeDecision {
    pub session_id: SubagentSessionId,
    pub parent_task_id: TaskId,
    pub child_task_id: Option<TaskId>,
    pub kind: SubagentRuntimeDecisionKind,
    pub reason: String,
    pub caps_snapshot: SubagentSessionCaps,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct SubagentTranscriptArtifactRecord {
    pub session_id: SubagentSessionId,
    pub parent_task_id: TaskId,
    pub child_task_id: Option<TaskId>,
    pub artifact_id: ArtifactId,
    pub event_range: EventRange,
    pub summary_label: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SubagentTranscriptArtifactStatus {
    Reserved,
    Published,
    Sealed,
    Abandoned,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct SubagentTranscriptArtifactLifecycleRecord {
    pub session_id: SubagentSessionId,
    pub parent_task_id: TaskId,
    pub child_task_id: Option<TaskId>,
    pub artifact_id: ArtifactId,
    pub status: SubagentTranscriptArtifactStatus,
    pub event_range: Option<EventRange>,
    pub summary_label: Option<String>,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SubagentApprovalForwardingStatus {
    QueuedForReviewer,
    ForwardedToParent,
    DeniedByPolicy,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct SubagentApprovalForwardingRecord {
    pub session_id: SubagentSessionId,
    pub parent_task_id: TaskId,
    pub approval_id: ApprovalId,
    pub reviewer_gate_id: Option<ReviewerGateId>,
    pub status: SubagentApprovalForwardingStatus,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SubagentInactiveParentAction {
    PauseParent,
    QueueDecision,
    RequireReviewer,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct SubagentInactivePolicyRecord {
    pub session_id: SubagentSessionId,
    pub parent_task_id: TaskId,
    pub policy: SubagentInactivePolicy,
    pub parent_action: SubagentInactiveParentAction,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SubagentCancellationCascade {
    CancelChild,
    ObserveOnly,
    QueueCancellation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct SubagentCancellationRecord {
    pub session_id: SubagentSessionId,
    pub parent_task_id: TaskId,
    pub source_task_id: TaskId,
    pub reason: String,
    pub cascade: SubagentCancellationCascade,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolPermission {
    FilesystemRead,
    FilesystemWrite,
    Network,
    Shell,
    Git,
    EnvRead,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSideEffect {
    ReadOnly,
    WritesWorkspace,
    WritesOutsideWorkspace,
    Network,
    Shell,
    PersistentState,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub id: ToolId,
    pub display_name: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
    #[serde(default)]
    pub required_permissions: Vec<ToolPermission>,
    #[serde(default)]
    pub side_effects: Vec<ToolSideEffect>,
    #[serde(default)]
    pub parallel_safe: bool,
    pub metadata: Option<ExtensionMap>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub call_id: ToolCallId,
    pub tool_id: ToolId,
    pub input: Value,
    pub metadata: Option<ExtensionMap>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyOutcome {
    Allow,
    Deny,
    AskUser,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolPolicyDecision {
    pub decision_id: PolicyDecisionId,
    pub call_id: ToolCallId,
    pub tool_id: ToolId,
    pub outcome: PolicyOutcome,
    pub reason: String,
    #[serde(default)]
    pub required_permissions: Vec<ToolPermission>,
    #[serde(default)]
    pub side_effects: Vec<ToolSideEffect>,
    pub approval_id: Option<ApprovalId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolApproval {
    pub approval_id: ApprovalId,
    pub call_id: ToolCallId,
    pub tool_id: ToolId,
    pub status: ApprovalStatus,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolDispatch {
    pub dispatch_id: ToolDispatchId,
    pub call_id: ToolCallId,
    pub tool_id: ToolId,
    pub declared_index: u32,
    #[serde(default)]
    pub parallel_safe: bool,
    pub metadata: Option<ExtensionMap>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolResultStatus {
    Succeeded,
    Failed,
    Skipped,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    pub result_id: ToolResultId,
    pub call_id: ToolCallId,
    pub tool_id: ToolId,
    pub declared_index: u32,
    pub status: ToolResultStatus,
    pub output: Value,
    pub error: Option<NormalizedError>,
    #[serde(default)]
    pub artifact_refs: Vec<ArtifactId>,
    pub metadata: Option<ExtensionMap>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolRepairKind {
    FlattenedNestedCalls,
    ScavengedJson,
    TruncatedArguments,
    CallStormDetected,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolRepairReport {
    pub repair_id: ToolRepairId,
    pub call_id: Option<ToolCallId>,
    pub tool_id: Option<ToolId>,
    pub kind: ToolRepairKind,
    pub reason: String,
    pub original_call_count: Option<u32>,
    pub repaired_call_count: Option<u32>,
    pub truncated_bytes: Option<u64>,
    pub metadata: Option<ExtensionMap>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceScope {
    pub workspace_root: String,
    #[serde(default)]
    pub allowed_roots: Vec<String>,
    #[serde(default)]
    pub denied_roots: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceAccess {
    Read,
    Write,
    Execute,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceGuardrail {
    pub scope: WorkspaceScope,
    pub requested_path: Option<String>,
    pub resolved_path: Option<String>,
    pub access: WorkspaceAccess,
    pub within_workspace: bool,
    #[serde(default)]
    pub required_permissions: Vec<ToolPermission>,
    #[serde(default)]
    pub side_effects: Vec<ToolSideEffect>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxDecisionKind {
    Allow,
    Deny,
    AskUser,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SandboxDecision {
    pub decision_id: SandboxDecisionId,
    pub call_id: Option<ToolCallId>,
    pub tool_id: Option<ToolId>,
    pub kind: SandboxDecisionKind,
    pub reason: String,
    pub guardrail: WorkspaceGuardrail,
    pub metadata: Option<ExtensionMap>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OsSandboxMode {
    ReadOnly,
    WorkspaceWrite,
    NetworkRequired,
    Denied,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OsSandboxFilesystem {
    ReadOnly,
    WorkspaceWrite,
    Denied,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OsSandboxNetwork {
    Disabled,
    Requested,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OsSandboxShell {
    Denied,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OsSandboxProfile {
    pub profile_id: OsSandboxProfileId,
    pub mode: OsSandboxMode,
    pub workspace_root: Option<String>,
    pub filesystem: OsSandboxFilesystem,
    pub network: OsSandboxNetwork,
    pub shell: OsSandboxShell,
    pub requires_checkpoint: bool,
    pub reason: String,
    pub metadata: Option<ExtensionMap>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotKind {
    SideGit,
    FileArchive,
    External,
}

impl SnapshotKind {
    pub fn from_snake_case(value: &str) -> Option<Self> {
        match value {
            "side_git" => Some(Self::SideGit),
            "file_archive" => Some(Self::FileArchive),
            "external" => Some(Self::External),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceCheckpoint {
    pub id: SnapshotId,
    pub kind: SnapshotKind,
    pub storage_uri: String,
    pub workspace_root: Option<String>,
    pub parent_snapshot_id: Option<SnapshotId>,
    pub summary: Option<String>,
    pub metadata: Option<ExtensionMap>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceCheckpointLifecycleStatus {
    Planned,
    Created,
    RestoreBlocked,
    Abandoned,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct WorkspaceCheckpointLifecycleRecord {
    pub checkpoint_id: SnapshotId,
    pub task_id: TaskId,
    pub status: WorkspaceCheckpointLifecycleStatus,
    pub reason: String,
    pub restore_plan_id: Option<RestorePlanId>,
    pub execution_blocked: bool,
    #[serde(default)]
    pub evidence: Vec<HandoffEvidenceRef>,
    pub metadata: Option<ExtensionMap>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct CostEstimate {
    pub amount: f64,
    pub currency: String,
    pub input_cost: Option<f64>,
    pub output_cost: Option<f64>,
    pub cache_read_cost: Option<f64>,
    pub cache_write_cost: Option<f64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoProgressSignalKind {
    RepeatedReadOnly,
    RepeatedRepair,
    NoOutput,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoProgressAction {
    Stop,
    AskUser,
    Summarize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NoProgressLoop {
    pub kind: NoProgressSignalKind,
    pub consecutive_count: u32,
    pub threshold: u32,
    pub action: NoProgressAction,
    pub reason: String,
    #[serde(default)]
    pub route_escalation_allowed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteStrategy {
    Manual,
    DefaultProfile,
    AutoRouter,
    LocalHeuristicFallback,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RouteDecision {
    pub requested_profile: Option<ModelProfileId>,
    pub selected_profile: ModelProfileId,
    pub selected_model: String,
    pub reasoning_level: Option<String>,
    pub strategy: RouteStrategy,
    #[serde(default)]
    pub decision_reason: Option<String>,
    pub fallback_reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSource {
    Provider,
    Config,
    Storage,
    Core,
    Cli,
    Tui,
    Tool,
    Policy,
    Agent,
    Memory,
    Skill,
    RuntimeApi,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NormalizedError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub source: ErrorSource,
    pub details: Option<ExtensionMap>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunEvent {
    ThreadCreated {
        thread_id: ThreadId,
    },
    TurnStarted {
        turn_id: TurnId,
    },
    UserMessageRecorded {
        item_id: ItemId,
        text: String,
    },
    InstructionsDiscovered {
        task_id: TaskId,
        sources: Vec<InstructionSource>,
    },
    SkillActivated {
        task_id: TaskId,
        activation: Box<SkillActivation>,
    },
    ProviderRequestStarted {
        provider_id: ProviderId,
        profile_id: ModelProfileId,
        model: String,
    },
    AssistantMessageStarted {
        item_id: ItemId,
    },
    AssistantDelta {
        item_id: ItemId,
        text: String,
    },
    AssistantReasoningDelta {
        item_id: ItemId,
        text: String,
    },
    AssistantMessageCompleted {
        item_id: ItemId,
    },
    UsageReported {
        input_tokens: Option<u64>,
        output_tokens: Option<u64>,
        total_tokens: Option<u64>,
        cache_read_tokens: Option<u64>,
        cache_write_tokens: Option<u64>,
        cache_miss_tokens: Option<u64>,
        estimated_cost: Option<CostEstimate>,
        latency_ms: Option<u64>,
    },
    ProviderCapabilityReported {
        provider_id: ProviderId,
        capability: ProviderCapability,
    },
    RouteDecisionRecorded {
        decision_id: RouteDecisionId,
        decision: RouteDecision,
    },
    ProviderRequestCompleted {
        provider_id: ProviderId,
    },
    TurnCompleted {
        turn_id: TurnId,
    },
    TaskCreated {
        task_id: TaskId,
        kind: TaskKind,
    },
    TaskStarted {
        task_id: TaskId,
    },
    TaskCompleted {
        task_id: TaskId,
    },
    TaskFailed {
        task_id: TaskId,
        error: NormalizedError,
    },
    TaskCancelled {
        task_id: TaskId,
        reason: Option<String>,
    },
    TaskPaused {
        task_id: TaskId,
        reason: Option<String>,
    },
    TaskResumed {
        task_id: TaskId,
        reason: Option<String>,
    },
    TaskPauseCheckpointCreated {
        checkpoint: TaskPauseCheckpoint,
    },
    RuntimeInstanceStarted {
        instance: RuntimeInstance,
    },
    TaskOwnerAttached {
        lease: Box<TaskOwnerLease>,
    },
    TaskOwnerHeartbeat {
        heartbeat: TaskOwnerHeartbeat,
    },
    TaskOwnerDetached {
        lease_id: TaskOwnershipId,
        task_id: TaskId,
        reason: Option<String>,
    },
    TaskOwnerLost {
        lease_id: TaskOwnershipId,
        task_id: TaskId,
        reason: Option<String>,
    },
    TaskReattachRecorded {
        record: TaskReattachRecord,
    },
    AgentRunStarted {
        task_id: TaskId,
        profile_id: AgentProfileId,
        objective: String,
    },
    AgentStepStarted {
        task_id: TaskId,
        step_index: u32,
    },
    AgentStepCompleted {
        summary: AgentStepSummary,
    },
    AgentRunCompleted {
        summary: AgentRunSummary,
    },
    AgentHandoffRecorded {
        summary: AgentHandoffSummary,
    },
    ReviewerGateRequested {
        request: ReviewerGateRequest,
    },
    ReviewerGateResolved {
        decision: ReviewerGateDecision,
    },
    CodingWorkflowStarted {
        workflow_id: CodingWorkflowId,
        task_id: TaskId,
        objective: String,
    },
    WorkspaceMutationScopeRecorded {
        scope: WorkspaceMutationScope,
    },
    MutationRequestProposalRecorded {
        proposal: MutationRequestProposal,
    },
    ApplyPatchPreflightRecorded {
        record: ApplyPatchPreflightRecord,
    },
    ApplyPatchExecutionRecorded {
        record: ApplyPatchExecutionRecord,
    },
    WorkspaceWorktreeLifecycleRecorded {
        record: WorkspaceWorktreeLifecycleRecord,
    },
    PatchProposalRecorded {
        proposal: PatchProposal,
    },
    PatchApplicationRecorded {
        record: PatchApplicationRecord,
    },
    TestPlanRecorded {
        plan: TestPlanRecord,
    },
    TestRunRecorded {
        record: TestRunRecord,
    },
    ReviewBundleRecorded {
        bundle: ReviewBundle,
    },
    RestorePlanRecorded {
        plan: RestorePlanRecord,
    },
    SubagentSessionPlanned {
        session: SubagentSessionDescriptor,
    },
    SubagentSessionStarted {
        session: SubagentSessionDescriptor,
    },
    SubagentSessionWaitingForApproval {
        session: SubagentSessionDescriptor,
    },
    SubagentSessionInactive {
        session: SubagentSessionDescriptor,
    },
    SubagentSessionCompleted {
        session: SubagentSessionDescriptor,
    },
    SubagentRuntimeDecisionRecorded {
        decision: SubagentRuntimeDecision,
    },
    SubagentTranscriptArtifactRecorded {
        transcript: SubagentTranscriptArtifactRecord,
    },
    SubagentTranscriptArtifactLifecycleRecorded {
        lifecycle: SubagentTranscriptArtifactLifecycleRecord,
    },
    SubagentApprovalForwardingRecorded {
        forwarding: SubagentApprovalForwardingRecord,
    },
    SubagentInactivePolicyRecorded {
        inactive: SubagentInactivePolicyRecord,
    },
    SubagentCancellationRecorded {
        cancellation: SubagentCancellationRecord,
    },
    NoProgressLoopDetected {
        task_id: TaskId,
        signal: NoProgressLoop,
    },
    DiagnosticsReported {
        report: DiagnosticReport,
    },
    MemoryWriteProposed {
        proposal: MemoryProposal,
    },
    MemoryWriteApplied {
        proposal: MemoryProposal,
    },
    MemoryWriteRejected {
        proposal: MemoryProposal,
    },
    ArtifactCreated {
        artifact_id: ArtifactId,
        kind: ArtifactKind,
    },
    ArtifactBodyRecorded {
        record: ArtifactBodyRecord,
    },
    SnapshotCreated {
        checkpoint: WorkspaceCheckpoint,
    },
    SnapshotLifecycleRecorded {
        lifecycle: WorkspaceCheckpointLifecycleRecord,
    },
    ToolCallRequested {
        request: ToolCallRequest,
    },
    ToolPolicyDecisionRecorded {
        decision: ToolPolicyDecision,
    },
    SandboxDecisionRecorded {
        decision: SandboxDecision,
    },
    OsSandboxProfileSelected {
        profile: OsSandboxProfile,
    },
    ToolDispatchStarted {
        dispatch: ToolDispatch,
    },
    ToolDispatchCompleted {
        result: ToolResult,
    },
    ToolResultRecorded {
        result: ToolResult,
    },
    ToolRepairReported {
        report: ToolRepairReport,
    },
    ToolCallApproved {
        approval: ToolApproval,
    },
    ToolCallDenied {
        approval: ToolApproval,
    },
    Error {
        error: NormalizedError,
    },
    Done,
}

impl RunEvent {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::ThreadCreated { .. } => "thread_created",
            Self::TurnStarted { .. } => "turn_started",
            Self::UserMessageRecorded { .. } => "user_message_recorded",
            Self::InstructionsDiscovered { .. } => "instructions_discovered",
            Self::SkillActivated { .. } => "skill_activated",
            Self::ProviderRequestStarted { .. } => "provider_request_started",
            Self::AssistantMessageStarted { .. } => "assistant_message_started",
            Self::AssistantDelta { .. } => "assistant_delta",
            Self::AssistantReasoningDelta { .. } => "assistant_reasoning_delta",
            Self::AssistantMessageCompleted { .. } => "assistant_message_completed",
            Self::UsageReported { .. } => "usage_reported",
            Self::ProviderCapabilityReported { .. } => "provider_capability_reported",
            Self::RouteDecisionRecorded { .. } => "route_decision_recorded",
            Self::ProviderRequestCompleted { .. } => "provider_request_completed",
            Self::TurnCompleted { .. } => "turn_completed",
            Self::TaskCreated { .. } => "task_created",
            Self::TaskStarted { .. } => "task_started",
            Self::TaskCompleted { .. } => "task_completed",
            Self::TaskFailed { .. } => "task_failed",
            Self::TaskCancelled { .. } => "task_cancelled",
            Self::TaskPaused { .. } => "task_paused",
            Self::TaskResumed { .. } => "task_resumed",
            Self::TaskPauseCheckpointCreated { .. } => "task_pause_checkpoint_created",
            Self::RuntimeInstanceStarted { .. } => "runtime_instance_started",
            Self::TaskOwnerAttached { .. } => "task_owner_attached",
            Self::TaskOwnerHeartbeat { .. } => "task_owner_heartbeat",
            Self::TaskOwnerDetached { .. } => "task_owner_detached",
            Self::TaskOwnerLost { .. } => "task_owner_lost",
            Self::TaskReattachRecorded { .. } => "task_reattach_recorded",
            Self::AgentRunStarted { .. } => "agent_run_started",
            Self::AgentStepStarted { .. } => "agent_step_started",
            Self::AgentStepCompleted { .. } => "agent_step_completed",
            Self::AgentRunCompleted { .. } => "agent_run_completed",
            Self::AgentHandoffRecorded { .. } => "agent_handoff_recorded",
            Self::ReviewerGateRequested { .. } => "reviewer_gate_requested",
            Self::ReviewerGateResolved { .. } => "reviewer_gate_resolved",
            Self::CodingWorkflowStarted { .. } => "coding_workflow_started",
            Self::WorkspaceMutationScopeRecorded { .. } => "workspace_mutation_scope_recorded",
            Self::MutationRequestProposalRecorded { .. } => "mutation_request_proposal_recorded",
            Self::ApplyPatchPreflightRecorded { .. } => "apply_patch_preflight_recorded",
            Self::ApplyPatchExecutionRecorded { .. } => "apply_patch_execution_recorded",
            Self::WorkspaceWorktreeLifecycleRecorded { .. } => {
                "workspace_worktree_lifecycle_recorded"
            }
            Self::PatchProposalRecorded { .. } => "patch_proposal_recorded",
            Self::PatchApplicationRecorded { .. } => "patch_application_recorded",
            Self::TestPlanRecorded { .. } => "test_plan_recorded",
            Self::TestRunRecorded { .. } => "test_run_recorded",
            Self::ReviewBundleRecorded { .. } => "review_bundle_recorded",
            Self::RestorePlanRecorded { .. } => "restore_plan_recorded",
            Self::SubagentSessionPlanned { .. } => "subagent_session_planned",
            Self::SubagentSessionStarted { .. } => "subagent_session_started",
            Self::SubagentSessionWaitingForApproval { .. } => {
                "subagent_session_waiting_for_approval"
            }
            Self::SubagentSessionInactive { .. } => "subagent_session_inactive",
            Self::SubagentSessionCompleted { .. } => "subagent_session_completed",
            Self::SubagentRuntimeDecisionRecorded { .. } => "subagent_runtime_decision_recorded",
            Self::SubagentTranscriptArtifactRecorded { .. } => {
                "subagent_transcript_artifact_recorded"
            }
            Self::SubagentTranscriptArtifactLifecycleRecorded { .. } => {
                "subagent_transcript_artifact_lifecycle_recorded"
            }
            Self::SubagentApprovalForwardingRecorded { .. } => {
                "subagent_approval_forwarding_recorded"
            }
            Self::SubagentInactivePolicyRecorded { .. } => "subagent_inactive_policy_recorded",
            Self::SubagentCancellationRecorded { .. } => "subagent_cancellation_recorded",
            Self::NoProgressLoopDetected { .. } => "no_progress_loop_detected",
            Self::DiagnosticsReported { .. } => "diagnostics_reported",
            Self::MemoryWriteProposed { .. } => "memory_write_proposed",
            Self::MemoryWriteApplied { .. } => "memory_write_applied",
            Self::MemoryWriteRejected { .. } => "memory_write_rejected",
            Self::ArtifactCreated { .. } => "artifact_created",
            Self::ArtifactBodyRecorded { .. } => "artifact_body_recorded",
            Self::SnapshotCreated { .. } => "snapshot_created",
            Self::SnapshotLifecycleRecorded { .. } => "snapshot_lifecycle_recorded",
            Self::ToolCallRequested { .. } => "tool_call_requested",
            Self::ToolPolicyDecisionRecorded { .. } => "tool_policy_decision_recorded",
            Self::SandboxDecisionRecorded { .. } => "sandbox_decision_recorded",
            Self::OsSandboxProfileSelected { .. } => "os_sandbox_profile_selected",
            Self::ToolDispatchStarted { .. } => "tool_dispatch_started",
            Self::ToolDispatchCompleted { .. } => "tool_dispatch_completed",
            Self::ToolResultRecorded { .. } => "tool_result",
            Self::ToolRepairReported { .. } => "tool_repair_reported",
            Self::ToolCallApproved { .. } => "tool_call_approved",
            Self::ToolCallDenied { .. } => "tool_call_denied",
            Self::Error { .. } => "error",
            Self::Done => "done",
        }
    }

    pub fn item_id(&self) -> Option<ItemId> {
        match self {
            Self::UserMessageRecorded { item_id, .. }
            | Self::AssistantMessageStarted { item_id }
            | Self::AssistantDelta { item_id, .. }
            | Self::AssistantReasoningDelta { item_id, .. }
            | Self::AssistantMessageCompleted { item_id } => Some(item_id.clone()),
            _ => None,
        }
    }

    pub fn task_id(&self) -> Option<TaskId> {
        match self {
            Self::TaskCreated { task_id, .. }
            | Self::TaskStarted { task_id }
            | Self::TaskCompleted { task_id }
            | Self::TaskFailed { task_id, .. }
            | Self::TaskCancelled { task_id, .. }
            | Self::TaskPaused { task_id, .. }
            | Self::TaskResumed { task_id, .. }
            | Self::InstructionsDiscovered { task_id, .. }
            | Self::SkillActivated { task_id, .. }
            | Self::TaskOwnerDetached { task_id, .. }
            | Self::TaskOwnerLost { task_id, .. }
            | Self::AgentRunStarted { task_id, .. }
            | Self::AgentStepStarted { task_id, .. }
            | Self::CodingWorkflowStarted { task_id, .. }
            | Self::NoProgressLoopDetected { task_id, .. } => Some(task_id.clone()),
            Self::ArtifactBodyRecorded { record } => record.task_id.clone(),
            Self::SnapshotLifecycleRecorded { lifecycle } => Some(lifecycle.task_id.clone()),
            Self::TaskPauseCheckpointCreated { checkpoint } => Some(checkpoint.task_id.clone()),
            Self::TaskOwnerAttached { lease } => Some(lease.task_id.clone()),
            Self::TaskOwnerHeartbeat { heartbeat } => Some(heartbeat.task_id.clone()),
            Self::TaskReattachRecorded { record } => Some(record.task_id.clone()),
            Self::AgentStepCompleted { summary } => Some(summary.task_id.clone()),
            Self::AgentRunCompleted { summary } => Some(summary.task_id.clone()),
            Self::AgentHandoffRecorded { summary } => Some(summary.parent_task_id.clone()),
            Self::ReviewerGateRequested { request } => Some(request.parent_task_id.clone()),
            Self::WorkspaceMutationScopeRecorded { scope } => Some(scope.task_id.clone()),
            Self::MutationRequestProposalRecorded { proposal } => Some(proposal.task_id.clone()),
            Self::ApplyPatchPreflightRecorded { record } => Some(record.task_id.clone()),
            Self::ApplyPatchExecutionRecorded { record } => Some(record.task_id.clone()),
            Self::WorkspaceWorktreeLifecycleRecorded { record } => Some(record.task_id.clone()),
            Self::PatchProposalRecorded { proposal } => Some(proposal.task_id.clone()),
            Self::PatchApplicationRecorded { record } => Some(record.task_id.clone()),
            Self::TestPlanRecorded { plan } => Some(plan.task_id.clone()),
            Self::TestRunRecorded { record } => Some(record.task_id.clone()),
            Self::ReviewBundleRecorded { bundle } => Some(bundle.task_id.clone()),
            Self::RestorePlanRecorded { plan } => Some(plan.task_id.clone()),
            Self::SubagentSessionPlanned { session }
            | Self::SubagentSessionStarted { session }
            | Self::SubagentSessionWaitingForApproval { session }
            | Self::SubagentSessionInactive { session }
            | Self::SubagentSessionCompleted { session } => Some(session.parent_task_id.clone()),
            Self::SubagentRuntimeDecisionRecorded { decision } => {
                Some(decision.parent_task_id.clone())
            }
            Self::SubagentTranscriptArtifactRecorded { transcript } => {
                Some(transcript.parent_task_id.clone())
            }
            Self::SubagentTranscriptArtifactLifecycleRecorded { lifecycle } => {
                Some(lifecycle.parent_task_id.clone())
            }
            Self::SubagentApprovalForwardingRecorded { forwarding } => {
                Some(forwarding.parent_task_id.clone())
            }
            Self::SubagentInactivePolicyRecorded { inactive } => {
                Some(inactive.parent_task_id.clone())
            }
            Self::SubagentCancellationRecorded { cancellation } => {
                Some(cancellation.parent_task_id.clone())
            }
            _ => None,
        }
    }

    pub fn turn_id(&self) -> Option<TurnId> {
        match self {
            Self::TurnStarted { turn_id } | Self::TurnCompleted { turn_id } => {
                Some(turn_id.clone())
            }
            _ => None,
        }
    }

    pub fn payload(&self) -> Value {
        match self {
            Self::ThreadCreated { thread_id } => json!({ "thread_id": thread_id }),
            Self::TurnStarted { turn_id } => json!({ "turn_id": turn_id }),
            Self::UserMessageRecorded { item_id, text } => {
                json!({ "item_id": item_id, "text": text })
            }
            Self::InstructionsDiscovered { task_id, sources } => {
                json!({ "task_id": task_id, "sources": sources })
            }
            Self::SkillActivated {
                task_id,
                activation,
            } => {
                json!({ "task_id": task_id, "activation": activation })
            }
            Self::ProviderRequestStarted {
                provider_id,
                profile_id,
                model,
            } => json!({
                "provider_id": provider_id,
                "profile_id": profile_id,
                "model": model,
            }),
            Self::AssistantMessageStarted { item_id } => json!({ "item_id": item_id }),
            Self::AssistantDelta { item_id, text }
            | Self::AssistantReasoningDelta { item_id, text } => {
                json!({ "item_id": item_id, "text": text })
            }
            Self::AssistantMessageCompleted { item_id } => json!({ "item_id": item_id }),
            Self::UsageReported {
                input_tokens,
                output_tokens,
                total_tokens,
                cache_read_tokens,
                cache_write_tokens,
                cache_miss_tokens,
                estimated_cost,
                latency_ms,
            } => json!({
                "input_tokens": input_tokens,
                "output_tokens": output_tokens,
                "total_tokens": total_tokens,
                "cache_read_tokens": cache_read_tokens,
                "cache_write_tokens": cache_write_tokens,
                "cache_miss_tokens": cache_miss_tokens,
                "estimated_cost": estimated_cost,
                "latency_ms": latency_ms,
            }),
            Self::ProviderCapabilityReported {
                provider_id,
                capability,
            } => json!({
                "provider_id": provider_id,
                "capability": capability,
            }),
            Self::RouteDecisionRecorded {
                decision_id,
                decision,
            } => json!({
                "decision_id": decision_id,
                "decision": decision,
            }),
            Self::ProviderRequestCompleted { provider_id } => {
                json!({ "provider_id": provider_id })
            }
            Self::TurnCompleted { turn_id } => json!({ "turn_id": turn_id }),
            Self::TaskCreated { task_id, kind } => {
                json!({ "task_id": task_id, "kind": kind })
            }
            Self::TaskStarted { task_id } => json!({ "task_id": task_id }),
            Self::TaskCompleted { task_id } => json!({ "task_id": task_id }),
            Self::TaskFailed { task_id, error } => {
                json!({ "task_id": task_id, "error": error })
            }
            Self::TaskCancelled { task_id, reason } => {
                json!({ "task_id": task_id, "reason": reason })
            }
            Self::TaskPaused { task_id, reason } | Self::TaskResumed { task_id, reason } => {
                json!({ "task_id": task_id, "reason": reason })
            }
            Self::TaskPauseCheckpointCreated { checkpoint } => {
                json!({ "checkpoint": checkpoint })
            }
            Self::RuntimeInstanceStarted { instance } => {
                json!({ "instance": instance })
            }
            Self::TaskOwnerAttached { lease } => {
                json!({ "lease": lease })
            }
            Self::TaskOwnerHeartbeat { heartbeat } => {
                json!({ "heartbeat": heartbeat })
            }
            Self::TaskOwnerDetached {
                lease_id,
                task_id,
                reason,
            }
            | Self::TaskOwnerLost {
                lease_id,
                task_id,
                reason,
            } => {
                json!({ "lease_id": lease_id, "task_id": task_id, "reason": reason })
            }
            Self::TaskReattachRecorded { record } => {
                json!({ "record": record })
            }
            Self::AgentRunStarted {
                task_id,
                profile_id,
                objective,
            } => json!({
                "task_id": task_id,
                "profile_id": profile_id,
                "objective": objective,
            }),
            Self::AgentStepStarted {
                task_id,
                step_index,
            } => json!({
                "task_id": task_id,
                "step_index": step_index,
            }),
            Self::AgentStepCompleted { summary } => json!({ "summary": summary }),
            Self::AgentRunCompleted { summary } => json!({ "summary": summary }),
            Self::AgentHandoffRecorded { summary } => json!({ "summary": summary }),
            Self::ReviewerGateRequested { request } => json!({ "request": request }),
            Self::ReviewerGateResolved { decision } => json!({ "decision": decision }),
            Self::CodingWorkflowStarted {
                workflow_id,
                task_id,
                objective,
            } => json!({
                "workflow_id": workflow_id,
                "task_id": task_id,
                "objective": objective,
            }),
            Self::WorkspaceMutationScopeRecorded { scope } => json!({ "scope": scope }),
            Self::MutationRequestProposalRecorded { proposal } => {
                json!({ "proposal": proposal })
            }
            Self::ApplyPatchPreflightRecorded { record } => json!({ "record": record }),
            Self::ApplyPatchExecutionRecorded { record } => json!({ "record": record }),
            Self::WorkspaceWorktreeLifecycleRecorded { record } => json!({ "record": record }),
            Self::PatchProposalRecorded { proposal } => json!({ "proposal": proposal }),
            Self::PatchApplicationRecorded { record } => json!({ "record": record }),
            Self::TestPlanRecorded { plan } => json!({ "plan": plan }),
            Self::TestRunRecorded { record } => json!({ "record": record }),
            Self::ReviewBundleRecorded { bundle } => json!({ "bundle": bundle }),
            Self::RestorePlanRecorded { plan } => json!({ "plan": plan }),
            Self::SubagentSessionPlanned { session }
            | Self::SubagentSessionStarted { session }
            | Self::SubagentSessionWaitingForApproval { session }
            | Self::SubagentSessionInactive { session }
            | Self::SubagentSessionCompleted { session } => json!({ "session": session }),
            Self::SubagentRuntimeDecisionRecorded { decision } => {
                json!({ "decision": decision })
            }
            Self::SubagentTranscriptArtifactRecorded { transcript } => {
                json!({ "transcript": transcript })
            }
            Self::SubagentTranscriptArtifactLifecycleRecorded { lifecycle } => {
                json!({ "lifecycle": lifecycle })
            }
            Self::SubagentApprovalForwardingRecorded { forwarding } => {
                json!({ "forwarding": forwarding })
            }
            Self::SubagentInactivePolicyRecorded { inactive } => {
                json!({ "inactive": inactive })
            }
            Self::SubagentCancellationRecorded { cancellation } => {
                json!({ "cancellation": cancellation })
            }
            Self::NoProgressLoopDetected { task_id, signal } => {
                json!({ "task_id": task_id, "signal": signal })
            }
            Self::DiagnosticsReported { report } => json!({ "report": report }),
            Self::MemoryWriteProposed { proposal }
            | Self::MemoryWriteApplied { proposal }
            | Self::MemoryWriteRejected { proposal } => json!({ "proposal": proposal }),
            Self::ArtifactCreated { artifact_id, kind } => {
                json!({ "artifact_id": artifact_id, "kind": kind })
            }
            Self::ArtifactBodyRecorded { record } => {
                json!({ "record": record })
            }
            Self::SnapshotCreated { checkpoint } => {
                json!({ "checkpoint": checkpoint })
            }
            Self::SnapshotLifecycleRecorded { lifecycle } => {
                json!({ "lifecycle": lifecycle })
            }
            Self::ToolCallRequested { request } => json!({ "request": request }),
            Self::ToolPolicyDecisionRecorded { decision } => json!({ "decision": decision }),
            Self::SandboxDecisionRecorded { decision } => json!({ "decision": decision }),
            Self::OsSandboxProfileSelected { profile } => json!({ "profile": profile }),
            Self::ToolDispatchStarted { dispatch } => json!({ "dispatch": dispatch }),
            Self::ToolDispatchCompleted { result } | Self::ToolResultRecorded { result } => {
                json!({ "result": result })
            }
            Self::ToolRepairReported { report } => json!({ "report": report }),
            Self::ToolCallApproved { approval } | Self::ToolCallDenied { approval } => {
                json!({ "approval": approval })
            }
            Self::Error { error } => json!({ "error": error }),
            Self::Done => json!({}),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventFrame {
    pub schema_version: u32,
    pub event_id: EventId,
    pub trace_id: String,
    pub seq: u64,
    pub timestamp: Timestamp,
    pub thread_id: Option<ThreadId>,
    pub turn_id: Option<TurnId>,
    pub item_id: Option<ItemId>,
    pub task_id: Option<TaskId>,
    pub event: RunEvent,
    pub extension: Option<ExtensionMap>,
    pub artifact_refs: Vec<ArtifactId>,
}

impl EventFrame {
    pub fn new(trace_id: impl Into<String>, seq: u64, event: RunEvent) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            event_id: EventId::new(),
            trace_id: trace_id.into(),
            seq,
            timestamp: Timestamp::now_utc(),
            thread_id: None,
            turn_id: None,
            item_id: None,
            task_id: None,
            event,
            extension: None,
            artifact_refs: Vec::new(),
        }
    }

    pub fn with_thread_id(mut self, thread_id: ThreadId) -> Self {
        self.thread_id = Some(thread_id);
        self
    }

    pub fn with_turn_id(mut self, turn_id: TurnId) -> Self {
        self.turn_id = Some(turn_id);
        self
    }

    pub fn with_item_id(mut self, item_id: ItemId) -> Self {
        self.item_id = Some(item_id);
        self
    }

    pub fn with_task_id(mut self, task_id: TaskId) -> Self {
        self.task_id = Some(task_id);
        self
    }

    pub fn with_artifact_ref(mut self, artifact_id: ArtifactId) -> Self {
        self.artifact_refs.push(artifact_id);
        self
    }

    pub fn to_trace_record(&self) -> TraceRecord {
        TraceRecord {
            schema_version: self.schema_version,
            trace_id: self.trace_id.clone(),
            seq: self.seq,
            event_id: self.event_id.clone(),
            timestamp: self.timestamp.clone(),
            thread_id: self.thread_id.clone(),
            turn_id: self.turn_id.clone(),
            item_id: self.item_id.clone(),
            task_id: self.task_id.clone(),
            event_kind: self.event.kind().to_string(),
            payload: self.event.payload(),
            extension: self.extension.clone(),
            artifact_refs: self.artifact_refs.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct TraceRecord {
    pub schema_version: u32,
    pub trace_id: String,
    pub seq: u64,
    pub event_id: EventId,
    pub timestamp: Timestamp,
    pub thread_id: Option<ThreadId>,
    pub turn_id: Option<TurnId>,
    pub item_id: Option<ItemId>,
    pub task_id: Option<TaskId>,
    pub event_kind: String,
    pub payload: Value,
    pub extension: Option<ExtensionMap>,
    pub artifact_refs: Vec<ArtifactId>,
}
