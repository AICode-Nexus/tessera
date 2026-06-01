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
pub struct RuntimeInstanceId(String);
pub struct ClientInstanceId(String);
pub struct TaskOwnershipId(String);
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
pub struct AgentHandoffId(String);
pub struct ReviewerGateId(String);
pub struct SubagentSessionId(String);
pub struct CodingWorkflowId(String);
pub struct PatchProposalId(String);
pub struct MutationRequestId(String);
pub struct WorkspaceWorktreeId(String);
pub struct TestPlanId(String);
pub struct TestRunId(String);
pub struct TestEvidenceSummaryId(String);
pub struct ReviewBundleId(String);
pub struct RestorePlanId(String);
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

`Paused` 表示 provider-neutral lifecycle metadata，可用于 UI 和 trace replay 展示任务暂挂状态。当前 foundation 不承诺真实 provider HTTP stream 被挂起、后台任务被持久化，或后续可从 checkpoint 恢复执行。

### 4.4.1 Task Pause Checkpoint

Task pause checkpoint 是 provider-neutral resume envelope metadata。它只描述未来 chat resume 可以从 trace projection 继续，不包含 provider 私有 socket、HTTP headers、API key、cookie、authorization 或 execution handle。

```rust
pub enum ResumeMode {
    BeforeProviderRequest,
    AfterCompletedProviderTurn,
    FromTraceProjection,
}

pub struct EventRange {
    pub start_seq: u64,
    pub end_seq: u64,
}

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
```

当前 core pause path 只写 `ResumeMode::FromTraceProjection`，并且不执行 `/resume-task`、不重连后台 runtime、不恢复 workspace checkpoint。

v0.5 background task ownership foundation 增加 trace-safe owner metadata：

```rust
pub enum TaskOwnerKind {
    Execution,
    Observer,
}

pub enum TaskOwnerStatus {
    Attached,
    Heartbeat,
    Detached,
    Lost,
}

pub enum TaskReattachMode {
    ObserveExistingOwner,
    ResumeFromCheckpoint,
    TerminalProjection,
    OwnerLost,
}

pub struct RuntimeInstance {
    pub runtime_id: RuntimeInstanceId,
    pub process_id: Option<u32>,
    pub started_at: Timestamp,
    pub hostname: Option<String>,
    pub working_directory: Option<String>,
    pub capabilities: Vec<String>,
}

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

pub struct TaskOwnerHeartbeat {
    pub lease_id: TaskOwnershipId,
    pub task_id: TaskId,
    pub runtime_id: RuntimeInstanceId,
    pub heartbeat_at: Timestamp,
    pub expires_at: Timestamp,
    pub last_seq: u64,
}

pub struct TaskReattachRecord {
    pub task_id: TaskId,
    pub mode: TaskReattachMode,
    pub previous_lease_id: Option<TaskOwnershipId>,
    pub new_lease_id: Option<TaskOwnershipId>,
    pub checkpoint_id: Option<TaskPauseCheckpointId>,
    pub since_seq: Option<u64>,
    pub reason: Option<String>,
}
```

这些结构只能记录 runtime/client/lease/status/seq/reason 等 metadata，不得保存 provider socket、headers、API key、cookie、env、命令行 secret、tool output 或文件内容。

v0.5 runtime API / app-server alignment 增加 wire DTO shape，但仍不启动 listener：

```rust
pub const RUNTIME_API_PROTOCOL_VERSION: &str = "v0";

pub enum RuntimeApiBindKind {
    LocalhostTcp,
    UnixSocket,
}

pub enum RuntimeApiAuthMode {
    LoopbackDevToken,
    OsUserSession,
}

pub enum RuntimeApiQueueOverflow {
    RejectNew,
}

pub struct RuntimeApiServerConfig {
    pub version: String,
    pub bind: RuntimeApiBindConfig,
    pub auth: RuntimeApiAuthPolicy,
    pub queue: RuntimeApiQueuePolicy,
}

pub enum RuntimeApiCommand {
    ListEvents(RuntimeApiEventStreamRequest),
    SubscribeEvents(RuntimeApiEventStreamRequest),
}

pub struct RuntimeApiCommandEnvelope {
    pub command_id: String,
    pub client_id: Option<ClientInstanceId>,
    pub trace_id: Option<String>,
    pub since_seq: Option<u64>,
    pub command: RuntimeApiCommand,
}
```

`RuntimeApiServerConfig::localhost_default()` uses `127.0.0.1`, no fixed port, `LoopbackDevToken`, bounded event/client queue capacities, and `RejectNew` overflow semantics. These DTOs may be exported to generated TypeScript as schema evidence for GUI/app-server clients, but they do not create an app-server crate, bind a socket, store auth token values, call providers, execute tools, or own runtime scheduling. `RuntimeApiCommand` currently covers read-only event listing/subscription shape only; future mutation commands must map to core runtime commands or `ClientIntent` and enter policy/trace before execution.

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

当前实现的事件（v0.1 基线 + v0.2-v0.7 foundation/runtime signals）：

```rust
pub enum RunEvent {
    ThreadCreated { thread_id: ThreadId },
    TurnStarted { turn_id: TurnId },
    UserMessageRecorded { item_id: ItemId, text: String },
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
    TaskPauseCheckpointCreated { checkpoint: TaskPauseCheckpoint },
    TaskPaused { task_id: TaskId, reason: Option<String> },
    TaskResumed { task_id: TaskId, reason: Option<String> },
    RuntimeInstanceStarted { instance: RuntimeInstance },
    TaskOwnerAttached { lease: Box<TaskOwnerLease> },
    TaskOwnerHeartbeat { heartbeat: TaskOwnerHeartbeat },
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
    TaskReattachRecorded { record: TaskReattachRecord },

    AgentRunStarted {
        task_id: TaskId,
        profile_id: AgentProfileId,
        objective: String,
    },
    AgentStepStarted { task_id: TaskId, step_index: u32 },
    AgentStepCompleted { summary: AgentStepSummary },
    AgentRunCompleted { summary: AgentRunSummary },
    AgentHandoffRecorded { summary: AgentHandoffSummary },
    ReviewerGateRequested { request: ReviewerGateRequest },
    ReviewerGateResolved { decision: ReviewerGateDecision },
    CodingWorkflowStarted {
        workflow_id: CodingWorkflowId,
        task_id: TaskId,
        objective: String,
    },
    WorkspaceMutationScopeRecorded { scope: WorkspaceMutationScope },
    MutationRequestProposalRecorded { proposal: MutationRequestProposal },
    ApplyPatchPreflightRecorded { record: ApplyPatchPreflightRecord },
    ApplyPatchExecutionRecorded { record: ApplyPatchExecutionRecord },
    WorkspaceWorktreeLifecycleRecorded { record: WorkspaceWorktreeLifecycleRecord },
    PatchProposalRecorded { proposal: PatchProposal },
    PatchApplicationRecorded { record: PatchApplicationRecord },
    TestPlanRecorded { plan: TestPlanRecord },
    TestRunRecorded { record: TestRunRecord },
    TestEvidenceSummaryRecorded { record: TestEvidenceSummaryRecord },
    ReviewBundleRecorded { bundle: ReviewBundle },
    RestorePlanRecorded { plan: RestorePlanRecord },

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

`TaskPauseCheckpointCreated` 记录 trace-safe resume envelope metadata，当前只支持 chat trace projection resume mode。`TaskPaused` 和 `TaskResumed` 只记录 lifecycle metadata。它们可以被 client/TUI/GUI 投影为 `Paused` / `Running` 状态，但 provider stream suspension、background persistence、checkpoint restore 和 agent resume runtime 仍是后续能力。

`RuntimeInstanceStarted` / `TaskOwnerAttached` / `TaskOwnerHeartbeat` / `TaskOwnerDetached` / `TaskOwnerLost` / `TaskReattachRecorded` 是 v0.5 background task ownership foundation。它们只建立 trace-backed execution owner、observer/lost-owner 和 explicit reattach outcome metadata，供 `RuntimeReader`、client、CLI、TUI、GUI 和 future app-server 投影。当前 no-tool chat / agent runs 会自动写入 owner attach/detach metadata；当前不启动 daemon，不冻结 provider socket，不保证进程退出后仍有后台执行 owner 存活，也不执行工具或 workspace restore。

`InstructionsDiscovered` 是 v0.5 opt-in project instruction discovery 的 source report。payload 必须包含 `task_id` 和 `sources`；`sources` 只能记录 `AGENTS.md` / `CLAUDE.md` 的 source id、kind、absolute/relative path、precedence、placement、load status、byte counts、sha256、redaction status 和 warnings，不得包含 instruction text/content、secret、provider-private prompt 或 filesystem handle。

`SkillActivated` 是 v0.5 explicit Skill Runtime v1 的 trace-safe activation metadata。payload 必须包含 `task_id` 和 `activation`；`activation` 只能记录 skill manifest、entrypoint/reference source metadata、step metadata、byte counts、hash、redaction status 和 warnings，不得包含 `SKILL.md` 正文、reference 正文、provider prompt fragment、script body、tool output、secret 或 filesystem handle。

`AgentRunStarted` / `AgentStepStarted` / `AgentStepCompleted` / `AgentRunCompleted` 是 v0.5 no-tool single-agent loop 的标准生命周期事件。它们记录 `TaskKind::AgentRun` 的 provider-neutral run envelope、step index、step status、final summary 和 event evidence range；不得包含 provider-private raw response、hidden reasoning、tool output、shell command、file diff、secret 或 runtime handle。

`AgentHandoffRecorded` / `ReviewerGateRequested` / `ReviewerGateResolved` 是 v0.6 structured handoff and reviewer gate foundation。它们只记录 compact summary、bounded evidence refs 和 reviewer decision metadata，供 replay、client projection、CLI/TUI/GUI 和 future runtime API 检查。它们不启动 persistent child-agent runtime，不执行工具，不修改 workspace，不批准 file diff，也不创建 swarm scheduler。

`CodingWorkflowStarted` / `WorkspaceMutationScopeRecorded` / `MutationRequestProposalRecorded` / `ApplyPatchPreflightRecorded` / `ApplyPatchExecutionRecorded` / `WorkspaceWorktreeLifecycleRecorded` / `PatchProposalRecorded` / `PatchApplicationRecorded` / `TestPlanRecorded` / `TestRunRecorded` / `TestEvidenceSummaryRecorded` / `ReviewBundleRecorded` / `RestorePlanRecorded` 是 v0.7 coding-agent workflow foundation。它们只记录 workflow id、task id、worktree-first mutation scope、policy-controlled mutation request proposals、apply-patch preflight readiness summaries、apply-patch execution intent/result metadata、detached worktree lifecycle metadata、patch proposal summary、diff artifact refs、checkpoint/reviewer/policy references、sandbox profile labels、test plan/test result artifact refs、test evidence aggregate summaries、review bundle metadata and restore plan metadata。Mutation request operation kind 使用 `patch_application`、`test_run`、`checkpoint_restore`、`version_control_*` 等中性标签，不暴露 GUI/runtime command 字符串。它们不得 inline patch body、file contents、stdout/stderr bodies、provider-private responses、hidden reasoning、API keys、cookies、authorization headers 或 filesystem handles。`MutationRequestProposalRecorded`、`ApplyPatchPreflightRecorded`、`ApplyPatchExecutionRecorded`、`WorkspaceWorktreeLifecycleRecorded`、`PatchApplicationRecorded`、`TestEvidenceSummaryRecorded` 和 `RestorePlanRecorded` 当前仍不提供完整 coding-agent runtime：除 isolated apply-patch executor slice 明确实现的单文件 patch 写入和 opt-in detached worktree lifecycle 外，不运行测试，不读取测试输出正文，不 restore/revert checkpoint，不 stage/commit/push Git，也不让 UI 拥有执行状态。

`ApplyPatchGate` 是 core-owned preflight helper，不是 executor。它可从已经加载到内存的 patch artifact body 文本中做 bounded dry-run parsing，输出 affected path、operation summary 和 blocker metadata；`ApplyPatchPreflightRecorded` 只允许记录这些 summary 字段、request/patch refs、evidence refs 和 `executor_blocked` reason，不得把 patch body、文件内容、外部 patch command、workspace file handle、executor handle 或 applied-path claim 放进 protocol。

`ApplyPatchExecutionRecorded` 是 executor 的 first-class metadata event，不复用 `PatchApplicationRecorded` 作为唯一执行事实来源。`ApplyPatchExecutionRecord` 必须绑定 preflight id、workflow/task、mutation request、patch proposal、checkpoint/reviewer/policy/sandbox refs、isolated root label、executor label、status、affected paths、conflict paths、blockers 和 artifact refs；它不得包含 patch body、file contents、stdout/stderr bodies、provider-private responses、hidden reasoning、API keys、cookies、authorization headers、primary-root absolute path 或 raw filesystem handle。`PatchApplicationRecord` 可继续作为 workflow summary/projection，但不能单独证明 runtime 写入已发生。

`WorkspaceWorktreeLifecycleRecorded` 是 automatic isolated worktree lifecycle 的 provider-neutral metadata event。`WorkspaceWorktreeLifecycleRecord` 必须包含 worktree id、workflow/task id、trace id、source commit、可选 source branch label、worktree root label、worktree base key、lifecycle status、reason、可选 mutation request/patch refs、bounded evidence refs 和 safe metadata。status 使用 `planned`、`created`、`creation_failed`、`retained`、`cleanup_started`、`cleanup_completed`、`cleanup_failed`。`tessera worktree cleanup` 只能在 trace 已证明 worktree 由 Tessera 创建且 latest lifecycle 为 `retained` 时追加 cleanup lifecycle records；dry-run 只验证不追加。该 record 不得包含 source checkout 绝对路径、worktree 绝对路径、Git command、stdout/stderr body、file contents、secrets、authorization headers 或 provider-private handles。

Read-only worktree lifecycle consumers are part of the v0.7 projection surface: `RuntimeReader::list_worktree_lifecycles`, `tessera-client` `worktree_lifecycles` / `worktree_summary`, `tessera worktree list --trace <trace_id> [--json]`, TUI status summaries and GUI metadata panels may inspect lifecycle status from trace. `trace_cleanup_candidate` is only trace evidence that a generated retained worktree may be eligible for the separate cleanup command; it is not permission to clean up without explicit `--worktree-path` validation. GUI worktree panels must stay safe aggregate views and omit local paths, reason text, touched paths, summaries, diagnostics, artifact bodies and mutation controls.

`tessera apply-patch` 是 CLI envelope over isolated single-file executor。显式参数模式直接从调用者参数构造 envelope；trace-driven `--from-trace` 模式先从既有 trace metadata 解析 workflow/scope/mutation request/patch/checkpoint/reviewer/policy/sandbox/artifact evidence，再复用同一 envelope。它可以 append `ApplyPatchPreflightRecorded` 和 `ApplyPatchExecutionRecorded`，但不得 inline patch body 或 file contents。默认该 CLI 不创建 worktree；opt-in `--from-trace --auto-worktree` 会创建一个 generated detached worktree，append path-redacted `WorkspaceWorktreeLifecycleRecorded` records，并在 CLI output 返回本地 worktree path。该路径仍不得运行测试、restore checkpoint、stage/commit/push Git、让 UI state own execution，或使用 shell/`git apply`/provider tool execution 来应用 patch。

Trace-driven apply-patch automation uses the same protocol events. `tessera apply-patch --from-trace` resolves workflow, scope, mutation request, patch proposal, checkpoint lifecycle, reviewer decision, policy decision, sandbox label and clean patch artifact metadata from trace records, but still appends ordinary `ApplyPatchPreflightRecorded` / `ApplyPatchExecutionRecorded` records and must not introduce patch body, file content, absolute isolated-root path, shell command, checkpoint restore or Git mutation payloads. In auto-worktree mode, dry-run resolves the trace and records apply-patch preflight without creating a worktree or appending lifecycle records; non-dry apply appends planned/created lifecycle records before apply-patch preflight/execution and a retained/cleanup lifecycle record afterward. `tessera worktree list` is a read-only trace projection command. `tessera worktree cleanup` is a separate operator command that requires `--from-trace`, `--worktree-id` and an explicit `--worktree-path`, validates the retained lifecycle metadata, and uses only non-force `git worktree remove`; it must not run tests, restore checkpoints, stage, commit, push, open PRs or generalize into shell execution.

仍只预留、不执行的事件：

```rust
pub enum ReservedRunEvent {
    RouteEscalationRecorded,
    SkillStepStarted,
    MemoryRecall,
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

保留事件不得在当前版本里成为实际功能入口。它们只用于稳定未来扩展字段和避免命名冲突。

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

## 8. Skill Manifest And Activation Schema

Skill registry v0.2 只描述和查询 skill metadata。v0.5 explicit Skill Runtime v1 在此基础上增加 project-local `SKILL.md` discovery、显式 activation metadata、只读 reference source metadata 和 no-tool context rendering。第一版入口优先兼容 `SKILL.md` flat frontmatter，高级 `skill.toml` 仅作为格式预留。

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

`SkillManifest` 不包含 command、executable 或 script 字段。当前 activation 只能由 core planner 显式加载 project-local `SKILL.md` 和用户指定的相对 reference 文件，并通过 `skill_activated` 写 trace-safe metadata。真实脚本、工具调用和步骤执行仍必须等待 tool/policy/sandbox/trace 边界，不得由 registry 直接执行。

```rust
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

pub enum SkillActivationStatus {
    Activated,
    Failed,
}

pub enum SkillRedactionStatus {
    Clean,
    Redacted,
}

pub enum SkillStepKind {
    DiscoverEntrypoint,
    LoadEntrypoint,
    LoadReference,
    RenderContext,
}

pub enum SkillStepStatus {
    Completed,
    Skipped,
    Failed,
}

pub struct SkillReferenceSource {
    pub source_id: ContextId,
    pub path: String,
    pub relative_path: String,
    pub status: SkillLoadStatus,
    pub original_bytes: u64,
    pub loaded_bytes: u64,
    pub sha256: Option<String>,
    pub redaction_status: SkillRedactionStatus,
    pub warnings: Vec<String>,
}

pub struct SkillActivationStep {
    pub step_index: u32,
    pub kind: SkillStepKind,
    pub status: SkillStepStatus,
    pub source_id: Option<ContextId>,
    pub warnings: Vec<String>,
}

pub struct SkillActivation {
    pub task_id: TaskId,
    pub skill_id: SkillId,
    pub manifest: SkillManifest,
    pub status: SkillActivationStatus,
    pub entrypoint: SkillReferenceSource,
    pub references: Vec<SkillReferenceSource>,
    pub steps: Vec<SkillActivationStep>,
    pub warnings: Vec<String>,
}
```

`SkillReferenceSource` 只描述来源和安全处理结果。`SKILL.md` body、reference body、provider-visible rendered context 和 secret-like lines 不得进入 trace payload。

### Agent Profile And Run Summary Schema

Agent profile v0.5 foundation 描述可执行 agent 的静态 metadata。当前 no-tool `AgentLoop` 会使用它记录 run/step summary，并可接收显式 opt-in 的 project instruction context 和 explicit read-only skill context；它仍不执行工具、不运行 skill script、不持有 provider-private runtime state。

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

```rust
pub enum AgentStepStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
    Paused,
    StoppedNoProgress,
}

pub struct AgentStepSummary {
    pub task_id: TaskId,
    pub step_index: u32,
    pub status: AgentStepStatus,
    pub assistant_text: String,
    pub stop_reason: Option<String>,
}

pub struct AgentRunSummary {
    pub task_id: TaskId,
    pub profile_id: AgentProfileId,
    pub status: TaskStatus,
    pub steps_completed: u32,
    pub final_text: String,
    pub stop_reason: Option<String>,
    pub evidence_event_range: Option<EventRange>,
}
```

`AgentProfile` 不包含 command、executable、shell、provider-private handle 或 runtime state。`AgentStepSummary` 和 `AgentRunSummary` 只记录 provider-neutral lifecycle/result metadata。Skill activation 已通过 `SkillActivated` 标准事件表达；后续 executable skill steps、tool request、handoff 和 completion 必须继续通过 core/protocol/trace 的标准事件表达。

### Structured Handoff And Reviewer Gate Schema

v0.6 first foundation defines structured handoff and reviewer gate records before any persistent sub-agent runtime. A handoff is a compact, replayable summary from a child or delegated task back to a parent task. A reviewer gate is a trace-backed decision point over that handoff and its evidence.

```rust
pub enum AgentHandoffStatus {
    Completed,
    Failed,
    Paused,
    Cancelled,
}

pub enum HandoffEvidenceKind {
    TraceRange,
    TranscriptArtifact,
    SummaryArtifact,
    DiffArtifact,
    DiagnosticArtifact,
    TestOutputArtifact,
}

pub struct HandoffEvidenceRef {
    pub kind: HandoffEvidenceKind,
    pub artifact_id: Option<ArtifactId>,
    pub trace_id: Option<String>,
    pub event_range: Option<EventRange>,
    pub label: Option<String>,
    pub summary: Option<String>,
}

pub struct AgentHandoffMetrics {
    pub steps_completed: u32,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub estimated_cost: Option<CostEstimate>,
}

pub struct AgentHandoffSummary {
    pub handoff_id: AgentHandoffId,
    pub parent_task_id: TaskId,
    pub child_task_id: Option<TaskId>,
    pub status: AgentHandoffStatus,
    pub objective: String,
    pub summary: String,
    pub evidence: Vec<HandoffEvidenceRef>,
    pub metrics: AgentHandoffMetrics,
    pub evidence_event_range: Option<EventRange>,
}

pub enum ReviewerDecisionKind {
    Accept,
    Reject,
    RequestRevision,
}

pub struct ReviewerGateRequest {
    pub gate_id: ReviewerGateId,
    pub handoff_id: AgentHandoffId,
    pub parent_task_id: TaskId,
    pub requested_decisions: Vec<ReviewerDecisionKind>,
    pub evidence: Vec<HandoffEvidenceRef>,
}

pub struct ReviewerGateDecision {
    pub gate_id: ReviewerGateId,
    pub handoff_id: AgentHandoffId,
    pub decision: ReviewerDecisionKind,
    pub reviewer: String,
    pub reason_code: String,
    pub comment: Option<String>,
}
```

`HandoffEvidenceRef` is metadata only. It may point at trace ranges or artifact IDs, but must not inline full transcripts, file contents, shell output, provider-private raw responses, hidden reasoning, API keys, cookies, authorization headers, or workspace diffs. `ReviewerGateDecision` records review state only; file mutation, Git mutation, checkpoint restore, and tool execution remain future v0.7+ gates.

### Persistent Subagent Session Foundation Schema

The next v0.6 foundation slice describes persistent sub-agent sessions as replayable metadata before any scheduler or child runtime exists. A sub-agent session descriptor links a parent task to an optional child task, declares caps and scope, and points to transcript artifacts instead of dumping child context into the parent.

```rust
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

pub enum SubagentInactivePolicy {
    PauseParent,
    QueueDecision,
    RequireReviewer,
}

pub struct SubagentSessionCaps {
    pub max_steps: u32,
    pub max_depth: u32,
    pub timeout_ms: Option<u64>,
    pub max_child_sessions: u32,
    pub max_estimated_cost: Option<CostEstimate>,
    pub concurrency_slot: Option<String>,
}

pub struct SubagentApprovalForwarding {
    pub inactive_policy: SubagentInactivePolicy,
    pub reviewer_gate_id: Option<ReviewerGateId>,
    pub approval_id: Option<ApprovalId>,
    pub forwarded_from_parent: bool,
}

pub struct SubagentSessionDescriptor {
    pub session_id: SubagentSessionId,
    pub parent_task_id: TaskId,
    pub child_task_id: Option<TaskId>,
    pub profile_id: AgentProfileId,
    pub objective: String,
    pub status: SubagentSessionStatus,
    pub scope_labels: Vec<String>,
    pub tool_permission_labels: Vec<String>,
    pub memory_scope_labels: Vec<String>,
    pub transcript_artifact_id: Option<ArtifactId>,
    pub caps: SubagentSessionCaps,
    pub approval_forwarding: Option<SubagentApprovalForwarding>,
}
```

Implemented foundation event names:

```rust
pub enum SubagentSessionRunEvent {
    SubagentSessionPlanned { session: SubagentSessionDescriptor },
    SubagentSessionStarted { session: SubagentSessionDescriptor },
    SubagentSessionWaitingForApproval { session: SubagentSessionDescriptor },
    SubagentSessionInactive { session: SubagentSessionDescriptor },
    SubagentSessionCompleted { session: SubagentSessionDescriptor },
}
```

These events only describe session state. They do not start child runs, call providers, execute tools, forward approvals automatically, mutate workspaces, restore checkpoints, or create swarm scheduling.

### Subagent Runtime Ownership Metadata

v0.6 runtime ownership events describe future scheduler decisions and lifecycle ownership records before any child-agent execution path exists. They are provider-neutral metadata and must remain replayable without a scheduler loop.

```rust
pub enum SubagentRuntimeDecisionKind {
    StartAllowed,
    StartDenied,
    QueueOnly,
    RequireReviewer,
}

pub struct SubagentRuntimeDecision {
    pub session_id: SubagentSessionId,
    pub parent_task_id: TaskId,
    pub child_task_id: Option<TaskId>,
    pub kind: SubagentRuntimeDecisionKind,
    pub reason: String,
    pub caps_snapshot: SubagentSessionCaps,
}

pub struct SubagentTranscriptArtifactRecord {
    pub session_id: SubagentSessionId,
    pub parent_task_id: TaskId,
    pub child_task_id: Option<TaskId>,
    pub artifact_id: ArtifactId,
    pub event_range: EventRange,
    pub summary_label: Option<String>,
}

pub enum SubagentTranscriptArtifactStatus {
    Reserved,
    Published,
    Sealed,
    Abandoned,
}

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

pub enum SubagentApprovalForwardingStatus {
    QueuedForReviewer,
    ForwardedToParent,
    DeniedByPolicy,
}

pub struct SubagentApprovalForwardingRecord {
    pub session_id: SubagentSessionId,
    pub parent_task_id: TaskId,
    pub approval_id: ApprovalId,
    pub reviewer_gate_id: Option<ReviewerGateId>,
    pub status: SubagentApprovalForwardingStatus,
    pub reason: String,
}

pub enum SubagentInactiveParentAction {
    PauseParent,
    QueueDecision,
    RequireReviewer,
}

pub struct SubagentInactivePolicyRecord {
    pub session_id: SubagentSessionId,
    pub parent_task_id: TaskId,
    pub policy: SubagentInactivePolicy,
    pub parent_action: SubagentInactiveParentAction,
    pub reason: String,
}

pub enum SubagentCancellationCascade {
    CancelChild,
    ObserveOnly,
    QueueCancellation,
}

pub struct SubagentCancellationRecord {
    pub session_id: SubagentSessionId,
    pub parent_task_id: TaskId,
    pub source_task_id: TaskId,
    pub reason: String,
    pub cascade: SubagentCancellationCascade,
}
```

Implemented ownership event names:

```rust
pub enum SubagentRuntimeOwnershipRunEvent {
    SubagentRuntimeDecisionRecorded { decision: SubagentRuntimeDecision },
    SubagentTranscriptArtifactRecorded { transcript: SubagentTranscriptArtifactRecord },
    SubagentTranscriptArtifactLifecycleRecorded { lifecycle: SubagentTranscriptArtifactLifecycleRecord },
    SubagentApprovalForwardingRecorded { forwarding: SubagentApprovalForwardingRecord },
    SubagentInactivePolicyRecorded { inactive: SubagentInactivePolicyRecord },
    SubagentCancellationRecorded { cancellation: SubagentCancellationRecord },
}
```

`SubagentTranscriptArtifactLifecycleRecorded` records artifact-handle lifecycle state only: reserved, published, sealed, or abandoned. It may reference bounded event ranges and summary labels, but must not inline transcript bodies, provider-private responses, hidden reasoning, tool output, file contents, shell output, secrets, authorization headers, cookies, workspace diffs or filesystem handles.

These events do not start child runs, call providers, dispatch tools, create a scheduler loop, forward approvals automatically, mutate workspaces, restore checkpoints, store transcript bodies, or imply app-server control.

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

### Instruction Source Metadata

Project instruction discovery 使用独立的 source report，而不是把正文塞进 context reference 或 trace payload。

```rust
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
    pub warnings: Vec<String>,
}
```

`InstructionLoadStatus` 目前覆盖 `loaded`、`skipped_lower_precedence`、`skipped_symlink`、`skipped_outside_workspace`、`skipped_non_utf8`、`skipped_too_large` 和 `read_failed`。`InstructionRedactionStatus` 目前覆盖 `clean` 与 `redacted`。

`LoadedInstruction` 是 core 内部 provider context 输入，不是 trace schema；trace 只记录 `InstructionSource` metadata。

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
