use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

pub const SCHEMA_VERSION: u32 = 1;

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    SnapshotCreated {
        checkpoint: WorkspaceCheckpoint,
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
            Self::NoProgressLoopDetected { .. } => "no_progress_loop_detected",
            Self::DiagnosticsReported { .. } => "diagnostics_reported",
            Self::MemoryWriteProposed { .. } => "memory_write_proposed",
            Self::MemoryWriteApplied { .. } => "memory_write_applied",
            Self::MemoryWriteRejected { .. } => "memory_write_rejected",
            Self::ArtifactCreated { .. } => "artifact_created",
            Self::SnapshotCreated { .. } => "snapshot_created",
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
            | Self::NoProgressLoopDetected { task_id, .. } => Some(task_id.clone()),
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
            Self::SnapshotCreated { checkpoint } => {
                json!({ "checkpoint": checkpoint })
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
