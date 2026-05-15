# Tessera Protocol v0

日期：2026-05-14

## 1. 目标

Protocol v0 定义 Tessera v0.1 的最小公共运行时语义。它不是 UI 状态，也不是 provider SDK 的翻译层。

Protocol v0 要服务五个入口：

- CLI。
- TUI。
- Storage。
- Replay。
- 未来 runtime API。

任何入口都不应该各自发明 Thread、Turn、Item、Task、Artifact 或事件生命周期。

## 2. 设计原则

- Provider-neutral：不能暴露 OpenAI、Ollama、Anthropic、Gemini 等私有响应结构。
- UI-neutral：不能出现 Ratatui widget、pane、cursor 等 UI 私有状态。
- Append-friendly：所有可观察行为都能表示成单调递增事件。
- Versioned：所有可持久化结构必须带 schema version 或可由外层 EventFrame 标记版本。
- Extensible：provider 专属能力只能进入 extension metadata，不能污染核心字段。
- Replayable：离线 replay 不应需要 API key 或真实 provider。

## 3. 基础 ID

v0.1 应定义强类型 ID，避免在代码里到处传裸字符串。

```rust
pub struct ThreadId(String);
pub struct TurnId(String);
pub struct ItemId(String);
pub struct TaskId(String);
pub struct ArtifactId(String);
pub struct EventId(String);
pub struct ProviderId(String);
pub struct ModelProfileId(String);
pub struct WindowId(String);
pub struct RouteDecisionId(String);
```

ID 生成策略：

- 本地生成。
- 全局唯一。
- 可排序不是 v0.1 必需条件。
- 持久化时使用字符串。

## 4. Runtime Objects

### 4.0 Provider Capability

Provider capability 描述 provider 能力，而不是 provider 私有响应结构。

```rust
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
```

v0.1 只要求 capability 可被 `doctor --json`、trace 和 TUI 状态展示使用，不要求实现 Auto router。

### 4.1 Thread

Thread 是一次可恢复的会话或工作流容器。

```rust
pub struct Thread {
    pub id: ThreadId,
    pub title: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub active_model_profile: Option<ModelProfileId>,
    pub status: ThreadStatus,
}
```

```rust
pub enum ThreadStatus {
    Active,
    Archived,
}
```

v0.1 中，Thread 至少支持普通 chat session。后续 agent、replay、tool run 都应挂到同一套 Thread 语义上。

### 4.2 Turn

Turn 是一次用户输入触发的运行。

```rust
pub struct Turn {
    pub id: TurnId,
    pub thread_id: ThreadId,
    pub started_at: Timestamp,
    pub completed_at: Option<Timestamp>,
    pub status: TurnStatus,
}
```

```rust
pub enum TurnStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}
```

Turn 不等于 assistant message。一次 Turn 可以包含用户消息、assistant delta、provider metadata、usage、error、artifact 等多个 Item。

### 4.3 Item

Item 是 Turn 内的可观察单元。

```rust
pub struct Item {
    pub id: ItemId,
    pub thread_id: ThreadId,
    pub turn_id: Option<TurnId>,
    pub kind: ItemKind,
    pub status: ItemStatus,
    pub created_at: Timestamp,
    pub completed_at: Option<Timestamp>,
}
```

```rust
pub enum ItemKind {
    UserMessage,
    AssistantMessage,
    ProviderEvent,
    Usage,
    Error,
    ArtifactRef,

    // Reserved in v0.1.
    ToolCall,
    ToolResult,
    Approval,
    MemoryRecall,
    MemoryProposal,
    SkillEvent,
    AgentEvent,
}
```

```rust
pub enum ItemStatus {
    Created,
    Streaming,
    Completed,
    Failed,
    Cancelled,
}
```

### 4.4 Task

Task 是可运行工作的抽象。v0.1 只实现 chat task 的最小语义。

```rust
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
```

```rust
pub enum TaskKind {
    Chat,
    Replay,

    // Reserved in v0.1.
    ToolRun,
    AgentRun,
    MultiAgentRun,
    SwarmRun,
    LearningJob,
}
```

```rust
pub enum TaskStatus {
    Pending,
    Running,
    WaitingForApproval,
    Paused,
    Completed,
    Failed,
    Cancelled,
}
```

### 4.5 Artifact

Artifact 是大输出或外部化资源引用。v0.1 主要用于 trace、export、large provider metadata 或后续 tool output 的预留。

```rust
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
```

```rust
pub enum ArtifactKind {
    Trace,
    Export,
    ProviderRawMetadata,

    // Reserved in v0.1.
    ToolOutput,
    Patch,
    TestReport,
    AgentTranscript,
}
```

## 5. EventFrame

EventFrame 是写入 trace 和分发给 CLI/TUI 的统一事件包。

```rust
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
```

要求：

- `seq` 在同一个 trace 内单调递增。
- `schema_version` v0.1 固定为 `1`。
- `extension` 只能保存安全、可序列化、可脱敏的数据。
- EventFrame 是持久化和 replay 的主语义，不依赖 UI 状态。

## 6. RunEvent v0

v0.1 必须实现的事件：

```rust
pub enum RunEvent {
    ThreadCreated { thread_id: ThreadId },
    TurnStarted { turn_id: TurnId },
    UserMessageRecorded { item_id: ItemId, text: String },

    ProviderRequestStarted {
        provider_id: ProviderId,
        profile_id: ModelProfileId,
        model: String,
    },

    AssistantMessageStarted { item_id: ItemId },
    AssistantDelta { item_id: ItemId, text: String },
    AssistantReasoningDelta { item_id: ItemId, text: String },
    AssistantMessageCompleted { item_id: ItemId },

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

    TurnCompleted { turn_id: TurnId },

    TaskCreated { task_id: TaskId, kind: TaskKind },
    TaskStarted { task_id: TaskId },
    TaskCompleted { task_id: TaskId },
    TaskFailed { task_id: TaskId, error: NormalizedError },
    TaskCancelled { task_id: TaskId, reason: Option<String> },

    ArtifactCreated { artifact_id: ArtifactId, kind: ArtifactKind },

    Error { error: NormalizedError },
    Done,
}
```

v0.1 只预留、不执行的事件：

```rust
pub enum ReservedRunEvent {
    ToolCallRequested,
    ToolCallApproved,
    ToolCallDenied,
    ToolResult,
    ToolDispatchStarted,
    ToolDispatchCompleted,
    ToolRepairReported,
    NoProgressLoopDetected,
    RouteEscalationRecorded,
    SkillActivated,
    SkillStepStarted,
    MemoryRecall,
    MemoryWriteProposed,
    MemoryWriteApplied,
    AgentStarted,
    AgentHandoff,
    AgentCompleted,
    SwarmTaskStarted,
    SwarmAgentEvent,
    SwarmTaskCompleted,
    LearningObservation,
    LearningProposalCreated,
    LearningProposalApplied,
    WindowOpened,
    WindowFocused,
    WindowClosed,
    WindowLayoutChanged,
    SnapshotCreated,
    SandboxDecisionRecorded,
    DiagnosticsReported,
}
```

保留事件不得在 v0.1 里成为实际功能入口。它们只用于稳定未来扩展字段和避免命名冲突。

## 7. Provider Extension Metadata

Provider 专属能力必须进入 extension metadata。

允许示例：

```json
{
  "provider": "deepseek",
  "reasoning_blocks": [],
  "prefix_cache_hit": true,
  "cache_read_tokens": 1200
}
```

`RouteDecision` 用于记录未来 Auto router 或手动 profile resolution 的结果。v0.1 可以只记录手动选择，不实现自动路由。
未来自动升档或降级必须通过 `RouteDecision`、`RouteEscalationRecorded` 或安全 extension 记录触发原因；无进展循环应优先记录为 no-progress signal，不应被静默解释成“需要更贵模型”。

```rust
pub struct RouteDecision {
    pub requested_profile: Option<ModelProfileId>,
    pub selected_profile: ModelProfileId,
    pub selected_model: String,
    pub reasoning_level: Option<String>,
    pub strategy: RouteStrategy,
    pub decision_reason: Option<String>,
    pub fallback_reason: Option<String>,
}
```

```rust
pub enum RouteStrategy {
    Manual,
    DefaultProfile,

    // Reserved after v0.1.
    AutoRouter,
    LocalHeuristicFallback,
}
```

```rust
pub struct CostEstimate {
    pub amount: f64,
    pub currency: String,
    pub input_cost: Option<f64>,
    pub output_cost: Option<f64>,
    pub cache_read_cost: Option<f64>,
    pub cache_write_cost: Option<f64>,
}
```

禁止：

- 把 provider 私有结构作为 core event payload。
- 把 API key、authorization header、cookie、完整 request header 写入 trace。
- 让 TUI 直接依赖 provider extension。

## 8. Error Model

```rust
pub struct NormalizedError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub source: ErrorSource,
    pub details: Option<ExtensionMap>,
}
```

```rust
pub enum ErrorSource {
    Provider,
    Config,
    Storage,
    Core,
    Cli,
    Tui,

    // Reserved in v0.1.
    Tool,
    Policy,
    Agent,
    Memory,
    Skill,
    RuntimeApi,
}
```

错误必须可展示、可写入 trace、可用于 replay。包含敏感信息的 provider 原始错误必须先脱敏再进入 `details`。

## 9. Compatibility Rules

- v0.1 不承诺稳定外部 API，但承诺 trace schema 有明确版本。
- 任何 breaking schema change 必须提升 schema version。
- 新字段优先 optional。
- 新事件优先 additive。
- 删除事件必须提供 migration 或 replay fallback。
- `extension` 中的数据不能成为 core 行为的唯一依据。

## 10. v0.1 验收

Protocol v0 可进入实现前，必须能回答：

- 一次 CLI chat 如何映射成 Thread、Turn、Item、Task。
- 一次 TUI chat 是否复用同一套模型。
- provider stream 如何变成 EventFrame。
- JSONL trace 是否足以 replay mock provider 输出。
- SQLite 是否只是索引，不是另一套事件真相。
- 后续 tool/agent/memory 是否能通过保留事件接入，而不是绕过协议。
