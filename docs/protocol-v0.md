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
pub struct SkillId(String);
pub struct ToolId(String);
pub struct ToolCallId(String);
pub struct ToolDispatchId(String);
pub struct ToolResultId(String);
pub struct ToolRepairId(String);
pub struct ApprovalId(String);
pub struct PolicyDecisionId(String);
pub struct SandboxDecisionId(String);
pub struct OsSandboxProfileId(String);
pub struct SnapshotId(String);
pub struct ContextId(String);
pub struct DiagnosticReportId(String);
pub struct MemoryProposalId(String);
pub struct AgentProfileId(String);
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

当前实现的事件（v0.1 基线 + v0.2/v0.3 草案信号）：

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

    NoProgressLoopDetected {
        task_id: TaskId,
        signal: NoProgressLoop,
    },

    DiagnosticsReported { report: DiagnosticReport },
    MemoryWriteProposed { proposal: MemoryProposal },
    MemoryWriteApplied { proposal: MemoryProposal },
    MemoryWriteRejected { proposal: MemoryProposal },
    ArtifactCreated { artifact_id: ArtifactId, kind: ArtifactKind },
    SnapshotCreated { checkpoint: WorkspaceCheckpoint },
    ToolCallRequested { request: ToolCallRequest },
    ToolPolicyDecisionRecorded { decision: ToolPolicyDecision },
    SandboxDecisionRecorded { decision: SandboxDecision },
    OsSandboxProfileSelected { profile: OsSandboxProfile },
    ToolDispatchStarted { dispatch: ToolDispatch },
    ToolDispatchCompleted { result: ToolResult },
    ToolResultRecorded { result: ToolResult },
    ToolRepairReported { report: ToolRepairReport },
    ToolCallApproved { approval: ToolApproval },
    ToolCallDenied { approval: ToolApproval },

    Error { error: NormalizedError },
    Done,
}
```

仍只预留、不执行的事件：

```rust
pub enum ReservedRunEvent {
    RouteEscalationRecorded,
    SkillActivated,
    SkillStepStarted,
    MemoryRecall,
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

`NoProgressLoop` 是 v0.2 草案信号，用于在连续只读、重复 repair 或无输出循环出现时先停止、询问或摘要，而不是静默升档到更贵模型。`route_escalation_allowed` 默认必须为 `false`，直到 policy-backed routing 明确实现。

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
pub struct NoProgressLoop {
    pub kind: NoProgressSignalKind,
    pub consecutive_count: u32,
    pub threshold: u32,
    pub action: NoProgressAction,
    pub reason: String,
    pub route_escalation_allowed: bool,
}

pub enum NoProgressSignalKind {
    RepeatedReadOnly,
    RepeatedRepair,
    NoOutput,
}

pub enum NoProgressAction {
    Stop,
    AskUser,
    Summarize,
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

## 8. Skill Manifest Schema

Skill registry v0.2 只描述和查询 skill metadata，不执行 skill runtime。第一版入口优先兼容 `SKILL.md` frontmatter，高级 `skill.toml` 仅作为格式预留。

```rust
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

pub struct SkillEntrypoint {
    pub format: SkillEntrypointFormat,
    pub path: String,
}

pub enum SkillEntrypointFormat {
    SkillMd,
    SkillToml,
}

pub struct SkillRequirements {
    pub tools: Vec<String>,
    pub context: Vec<String>,
}

pub struct SkillPolicy {
    pub default_permission: String,
    pub network: String,
    pub write_files: String,
}
```

`SkillManifest` 不包含 command、executable 或 script 字段。后续 skill 激活、工具调用和步骤执行必须通过 core/tool/policy/trace 边界，不得由 registry 直接执行。

### Agent Profile Schema

Agent profile v0.5 foundation 只描述可执行 agent 的静态 metadata，不启动 agent loop、不激活 skill、不执行工具。它用于让未来 single-agent loop 在进入 runtime 前先拥有 provider-neutral 的角色、模型、scope 和 step limit 表达。

```rust
pub struct AgentProfile {
    pub id: AgentProfileId,
    pub name: String,
    pub role: String,
    pub model_profile: ModelProfileId,
    pub skills: Vec<SkillId>,
    pub memory_scopes: Vec<String>,
    pub context_scopes: Vec<String>,
    pub tool_permissions: Vec<ToolPermission>,
    pub max_steps: u32,
    pub metadata: Option<ExtensionMap>,
}
```

`AgentProfile` 不包含 command、executable、shell、provider-private handle 或 runtime state。后续 agent step、skill activation、tool request、handoff 和 completion 必须通过 core/protocol/trace 的标准事件表达。

## 9. Tool Descriptor / Policy / Dispatch / Repair Schema

Tool descriptor v0.3 草案只描述工具能力，不执行工具。第一版用于让 policy gate、approval UI、sandbox、MCP adapter 和 ordered dispatcher 有共同 schema。

```rust
pub struct ToolDescriptor {
    pub id: ToolId,
    pub display_name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub required_permissions: Vec<ToolPermission>,
    pub side_effects: Vec<ToolSideEffect>,
    pub parallel_safe: bool,
    pub metadata: Option<ExtensionMap>,
}

pub enum ToolPermission {
    FilesystemRead,
    FilesystemWrite,
    Network,
    Shell,
    Git,
    EnvRead,
}

pub enum ToolSideEffect {
    ReadOnly,
    WritesWorkspace,
    WritesOutsideWorkspace,
    Network,
    Shell,
    PersistentState,
}
```

`parallel_safe` 默认必须为 `false`，第三方/MCP tool 必须显式 opt in 才能被未来并发 dispatcher 视为可并行。`ToolDescriptor` 不包含 command、executable 或 shell 字段；真实执行必须等待 policy/sandbox/checkpoint/trace 边界完成。

MCP adapter foundation 使用同一 `ToolDescriptor` / `ToolCallRequest` schema 表达 MCP tool metadata 和 call arguments。MCP annotations 只能作为不可信 hint：closed read-only tool 可以映射为 `ReadOnly` side effect，open-world tool 必须映射为 `Network` permission/side effect，未知或非 read-only tool 必须保持保守。adapter metadata 只能保存 `mcp_server_id`、`mcp_tool_name` 和 annotation hints，不得保存 server URL、command、executable 或 transport handle。

Tool call request、policy decision 和 approval 现在可以写入 trace，但仍不执行工具：

```rust
pub struct ToolCallRequest {
    pub call_id: ToolCallId,
    pub tool_id: ToolId,
    pub input: serde_json::Value,
    pub metadata: Option<ExtensionMap>,
}

pub struct ToolPolicyDecision {
    pub decision_id: PolicyDecisionId,
    pub call_id: ToolCallId,
    pub tool_id: ToolId,
    pub outcome: PolicyOutcome,
    pub reason: String,
    pub required_permissions: Vec<ToolPermission>,
    pub side_effects: Vec<ToolSideEffect>,
    pub approval_id: Option<ApprovalId>,
}

pub enum PolicyOutcome {
    Allow,
    Deny,
    AskUser,
}

pub struct ToolApproval {
    pub approval_id: ApprovalId,
    pub call_id: ToolCallId,
    pub tool_id: ToolId,
    pub status: ApprovalStatus,
    pub reason: Option<String>,
}

pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
}
```

Tool dispatch 和 result metadata 也可以写入 trace，用来保证未来安全并发工具的结果按声明顺序 append：

```rust
pub struct ToolDispatch {
    pub dispatch_id: ToolDispatchId,
    pub call_id: ToolCallId,
    pub tool_id: ToolId,
    pub declared_index: u32,
    pub parallel_safe: bool,
    pub metadata: Option<ExtensionMap>,
}

pub enum ToolResultStatus {
    Succeeded,
    Failed,
    Skipped,
}

pub struct ToolResult {
    pub result_id: ToolResultId,
    pub call_id: ToolCallId,
    pub tool_id: ToolId,
    pub declared_index: u32,
    pub status: ToolResultStatus,
    pub output: serde_json::Value,
    pub error: Option<NormalizedError>,
    pub artifact_refs: Vec<ArtifactId>,
    pub metadata: Option<ExtensionMap>,
}

pub enum ToolRepairKind {
    FlattenedNestedCalls,
    ScavengedJson,
    TruncatedArguments,
    CallStormDetected,
}

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
```

`ToolCallRequest` 不包含 command、executable、shell 或 provider-private execution handle。`ToolPolicyDecision` 只记录 policy outcome，不能被解释为已经执行。`ToolDispatch` 和 `ToolResult` 记录调度/结果 metadata；真实执行仍必须由未来 tools + sandbox + checkpoint 边界承担。

`ToolRepairReport` 只记录 provider-neutral 修复摘要，例如 nested calls flatten、JSON scavenge、argument truncation 和 call storm detection。它不得保存 provider 原始 reasoning、raw text、hidden content 或 secret。

## 10. Workspace Guardrail / Sandbox Decision Schema

Workspace guardrail v0.3 草案只记录路径和沙箱判定 metadata，不执行 shell、不读取文件、不写文件，也不代表 OS sandbox 已经启用。它为后续 file write、shell、git、MCP tool 和 checkpoint 串联提供统一 trace 边界。

```rust
pub struct WorkspaceScope {
    pub workspace_root: String,
    pub allowed_roots: Vec<String>,
    pub denied_roots: Vec<String>,
}

pub enum WorkspaceAccess {
    Read,
    Write,
    Execute,
}

pub struct WorkspaceGuardrail {
    pub scope: WorkspaceScope,
    pub requested_path: Option<String>,
    pub resolved_path: Option<String>,
    pub access: WorkspaceAccess,
    pub within_workspace: bool,
    pub required_permissions: Vec<ToolPermission>,
    pub side_effects: Vec<ToolSideEffect>,
}

pub enum SandboxDecisionKind {
    Allow,
    Deny,
    AskUser,
}

pub struct SandboxDecision {
    pub decision_id: SandboxDecisionId,
    pub call_id: Option<ToolCallId>,
    pub tool_id: Option<ToolId>,
    pub kind: SandboxDecisionKind,
    pub reason: String,
    pub guardrail: WorkspaceGuardrail,
    pub metadata: Option<ExtensionMap>,
}
```

`sandbox_decision_recorded` payload 必须包含 `decision.kind`、`decision.reason` 和 `decision.guardrail`。`resolved_path` 是 guardrail 的词法解析结果，不得暗示已经通过 `canonicalize` 触碰真实文件系统。该事件不得包含 command、executable、shell、secret 或 provider-private execution handle。

### 10.1 OS Sandbox Profile Schema

OS sandbox profile v0.3 草案只描述未来 tool runtime 应使用的隔离 profile，不启动 OS sandbox、不 fork 进程、不执行 shell，也不授予文件或网络访问。它把 tool descriptor 的权限和 side effects 映射成可审计 metadata，为后续真实 sandbox executor、checkpoint 和 approval 串联提供合同。

```rust
pub enum OsSandboxMode {
    ReadOnly,
    WorkspaceWrite,
    NetworkRequired,
    Denied,
}

pub enum OsSandboxFilesystem {
    ReadOnly,
    WorkspaceWrite,
    Denied,
}

pub enum OsSandboxNetwork {
    Disabled,
    Requested,
}

pub enum OsSandboxShell {
    Denied,
}

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
```

`os_sandbox_profile_selected` payload 必须包含 `profile.mode`、`profile.filesystem`、`profile.network`、`profile.shell`、`profile.requires_checkpoint` 和 `profile.reason`。该事件不得包含 command、executable、shell command、env secret 或 provider-private execution handle；`network: requested` 只表示 future policy/runtime 需要显式处理网络，不表示网络已经打开。

## 11. Snapshot / Checkpoint Schema

Checkpoint v0.2 只记录可追踪 metadata，不创建、不恢复、不回滚文件。`snapshot_created` 可以用于记录未来 side-git 或等价 checkpoint 的句柄，并关联 task/turn。v0.3 foundation 的 core checkpoint planner 可以基于 `OsSandboxProfile.requires_checkpoint` 生成同一 schema 的 checkpoint metadata，但仍不创建真实 checkpoint。

```rust
pub struct WorkspaceCheckpoint {
    pub id: SnapshotId,
    pub kind: SnapshotKind,
    pub storage_uri: String,
    pub workspace_root: Option<String>,
    pub parent_snapshot_id: Option<SnapshotId>,
    pub summary: Option<String>,
    pub metadata: Option<ExtensionMap>,
}

pub enum SnapshotKind {
    SideGit,
    FileArchive,
    External,
}
```

Checkpoint schema 不包含 restore command、revert command 或 shell command。后续真实 create/restore/revert 必须经过 policy/sandbox，并写入独立 trace event。

## 12. Diagnostics / LSP Event Schema

Diagnostics v0.4 foundation 只记录 LSP-style diagnostic metadata，不启动 LSP server、不运行 compiler、不读取文件。它让 future diagnostics crate、editor integration、runtime API 和 replay 使用同一 `diagnostics_reported` event。

```rust
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

pub struct DiagnosticRange {
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: u32,
    pub end_character: u32,
}

pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub code: Option<String>,
    pub message: String,
    pub uri: Option<String>,
    pub range: Option<DiagnosticRange>,
    pub metadata: Option<ExtensionMap>,
}

pub struct DiagnosticReport {
    pub report_id: DiagnosticReportId,
    pub source: String,
    pub diagnostics: Vec<Diagnostic>,
    pub metadata: Option<ExtensionMap>,
}
```

`diagnostics_reported` payload 必须包含 `report.report_id`、`report.source` 和 `report.diagnostics`。range 使用 LSP-style line/character 字段；该事件不得包含 command、executable、process id、secret 或 provider-private handle。

## 13. Memory Proposal Schema

Memory proposal v0.4 foundation 只把“建议写入长期记忆”的候选项展示给 UI，不写入长期 memory store，不读取外部记忆，也不自动应用。它让 future memory runtime、GUI review surface 和 replay 使用同一套 proposal contract。

```rust
pub enum MemoryProposalStatus {
    Pending,
    Applied,
    Rejected,
}

pub struct MemoryProposal {
    pub proposal_id: MemoryProposalId,
    pub status: MemoryProposalStatus,
    pub title: String,
    pub summary: String,
    pub source_item_id: Option<ItemId>,
    pub reason: Option<String>,
    pub metadata: Option<ExtensionMap>,
}
```

`memory_write_proposed` payload 必须包含 pending `proposal`。`memory_write_applied` 和 `memory_write_rejected` 只记录 UI review state；它们不得表示真实 long-term memory write，除非 future memory runtime 在 policy/scope/trace 边界完成后另行扩展。payload 不得包含 memory store path、database URI、embedding payload、secret 或 command。

## 14. Context Workbench Schema

Context workbench v0.2 只记录上下文引用和预算，不读取文件、不保存大块内容、不构建 provider prompt。它为 cache-stable context 打底，明确区分稳定前缀、追加 transcript 和临时 scratch。

```rust
pub struct ContextReference {
    pub id: ContextId,
    pub source: ContextSource,
    pub placement: ContextPlacement,
    pub estimated_tokens: u64,
    pub pinned: bool,
    pub summary: Option<String>,
    pub metadata: Option<ExtensionMap>,
}

pub struct ContextSource {
    pub kind: ContextSourceKind,
    pub uri: Option<String>,
    pub label: Option<String>,
}

pub enum ContextPlacement {
    StablePrefix,
    AppendOnlyTranscript,
    VolatileScratch,
}

pub struct ContextBudget {
    pub max_tokens: u64,
    pub reserved_output_tokens: u64,
}
```

`ContextReference` 不包含 `content`、`bytes` 或 provider-specific prompt fragment。后续 context loader/compaction/handle read 必须通过 core/policy/trace 边界。

禁止：

- 把 provider 私有结构作为 core event payload。
- 把 API key、authorization header、cookie、完整 request header 写入 trace。
- 让 TUI 直接依赖 provider extension。

## 15. Error Model

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

## 16. Compatibility Rules

- v0.1 不承诺稳定外部 API，但承诺 trace schema 有明确版本。
- 任何 breaking schema change 必须提升 schema version。
- 新字段优先 optional。
- 新事件优先 additive。
- 删除事件必须提供 migration 或 replay fallback。
- `extension` 中的数据不能成为 core 行为的唯一依据。

## 17. v0.1 验收

Protocol v0 可进入实现前，必须能回答：

- 一次 CLI chat 如何映射成 Thread、Turn、Item、Task。
- 一次 TUI chat 是否复用同一套模型。
- provider stream 如何变成 EventFrame。
- JSONL trace 是否足以 replay mock provider 输出。
- SQLite 是否只是索引，不是另一套事件真相。
- 后续 tool/agent/memory 是否能通过保留事件接入，而不是绕过协议。
