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
        #[serde(transparent)]
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CostEstimate {
    pub amount: f64,
    pub currency: String,
    pub input_cost: Option<f64>,
    pub output_cost: Option<f64>,
    pub cache_read_cost: Option<f64>,
    pub cache_write_cost: Option<f64>,
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
    ArtifactCreated {
        artifact_id: ArtifactId,
        kind: ArtifactKind,
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
            Self::ArtifactCreated { .. } => "artifact_created",
            Self::Error { .. } => "error",
            Self::Done => "done",
        }
    }

    pub fn item_id(&self) -> Option<ItemId> {
        match self {
            Self::UserMessageRecorded { item_id }
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
            | Self::TaskCancelled { task_id, .. } => Some(task_id.clone()),
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
            Self::UserMessageRecorded { item_id } => json!({ "item_id": item_id }),
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
            Self::ArtifactCreated { artifact_id, kind } => {
                json!({ "artifact_id": artifact_id, "kind": kind })
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
