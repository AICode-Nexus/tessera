# Tessera 需求与架构设计

日期：2026-05-14

## 1. 背景

目标是开发 Tessera：一个新的通用大模型 TUI，形态上接近 deepseek-tui 一类终端交互工具，但定位不应局限于某一个模型或某一种聊天界面。

本项目的核心要求是质量优先，同时要适合 AI 持续参与开发。第一版可以不直接做完整 agent，但架构必须从第一天为 agent、工具调用、MCP、代码修改、测试执行和运行回放预留清晰边界，避免未来从普通聊天工具重构成 agent 平台时大拆。

## 2. 产品定位

一句话定位：

> 一个模型无关、面向 agent 演进、所有工具调用可审计、所有运行可回放的终端大模型工作台。

英文定位：

> A model-agnostic, agent-ready terminal workbench built on typed events, auditable tools, and replayable runs.

它首先是一个稳定、可控、可扩展的 LLM Terminal Workbench，而不是一个只支持单模型的聊天客户端。

## 3. 质量优先的技术选型

### 3.1 主选型

- 语言：Rust
- TUI：Ratatui
- 异步运行时：Tokio
- HTTP：Reqwest
- 序列化：Serde
- 配置：TOML 或 JSON
- 本地存储：SQLite + JSONL trace
- 测试：cargo test、fixture replay、golden tests
- MCP：优先评估官方 Rust SDK；必要时通过进程或协议桥接其他 SDK

### 3.2 为什么选择 Rust

Rust 符合“质量优先”的目标：

- 单文件二进制分发体验好。
- 适合构建长期维护的本地开发工具。
- 类型系统适合固化核心协议，例如 Message、RunEvent、ToolCall、PolicyDecision。
- 适合做本地 shell、文件系统、git、权限和审计相关能力。
- 性能和终端交互控制能力强。
- 可以通过清晰 crate 边界降低 AI 后续维护 Rust 代码的难度。

### 3.3 为什么不是 TypeScript 作为主实现

TypeScript + Ink 更适合快速原型和 MCP/Provider 生态接入速度，但本项目明确以质量、长期维护、终端体验和本地工具能力为优先级。

TypeScript 可以作为参考实现、协议验证或外部 bridge 的候选，但主实现建议 Rust-first。

## 4. 设计原则

### 4.1 TUI 只是 View

Ratatui 层只负责：

- 渲染布局
- 键盘事件
- 输入框状态
- 选择和焦点
- 展示流式事件

它不应该直接：

- 调用模型 API
- 执行 shell 或文件操作
- 操作 MCP tool
- 实现 agent loop
- 读写 provider 私有结构

所有核心能力都应进入 headless core。

### 4.2 Headless Core 优先

核心逻辑必须可以脱离 TUI 运行。项目至少应提供两个入口：

- `cli`：headless 调试、脚本、自动化、replay
- `tui`：Ratatui 交互界面

这能保证 AI 修改核心逻辑时可以通过命令行和测试验证，而不是只能手工启动 TUI 检查。

### 4.3 Provider 不污染 Core

OpenAI、Anthropic、Gemini、Ollama、DeepSeek、Qwen 等 provider 的私有响应结构不能泄漏到 core。

Provider adapter 的职责是：

- 接收标准 RunRequest
- 调用对应模型 API
- 将 provider 私有流式响应转换成标准 RunEvent
- 将 provider 错误转换成 NormalizedError

### 4.4 工具调用必须经过 Policy

任何 shell、filesystem、git、http、MCP tool 执行都必须经过 policy gate。

TUI、provider 和 agent loop 都不能绕过 policy 直接执行工具。

policy 层负责：

- 权限判断
- 风险识别
- 用户审批
- secret masking
- deny/allow 规则
- trace 记录

### 4.5 所有运行可回放

每次模型交互、工具调用、审批结果和错误都要写入 append-only JSONL trace。

目标是：

- bug 可以离线复现
- provider mock 可以用真实 trace 生成
- agent 行为可以审计
- AI 修复问题时不依赖真实 API key

### 4.6 记忆不是 Prompt 拼接

记忆系统不能简单等同于把历史对话塞进 prompt。记忆要有类型、来源、作用域、有效期、置信度和审计记录。

长期记忆默认不应是全局共享。至少要区分：

- 用户级记忆
- 工作区级记忆
- 项目级记忆
- skill 级记忆
- agent 级临时记忆
- session 级短期状态

记忆召回必须可解释，TUI 要能展示“为什么这条记忆被召回、来自哪里、作用域是什么”。

### 4.7 自学习不能静默改行为

自学习系统的第一原则是 proposal-first。系统可以从 trace、用户反馈、失败案例、eval 结果中提出改进建议，但不能默认静默修改 skill、prompt、policy 或 agent 配置。

自学习的闭环应是：

```text
observe -> extract -> propose -> verify -> approve -> apply
```

其中 `approve` 之前不能改变用户可见行为。

### 4.8 多 Agent 架构建立在单 Agent 之上

多 agent 模式不是另起一套执行系统。每个 agent 都应复用同一个 AgentLoop、Provider、Tool、Policy、Memory 和 Trace 协议。

蜂群模式是多 agent 的高级自动调度形态。普通多 agent 模式应先支持显式角色、显式 handoff、显式审查，再进入自动化 swarm scheduler。

### 4.9 任务和窗口分离

多任务、多窗口管理不能只做成 TUI 里的 tabs。任务是运行时对象，窗口是观察和控制任务的视图。

一个任务可以被多个窗口观察，例如聊天窗口、trace 窗口、diff 窗口、工具审批窗口、日志窗口。一个窗口也可以切换绑定到不同任务。这样后续支持后台任务、并行 agent、replay、长时间工具执行和任务恢复时，不需要重写 TUI。

## 5. Workspace 结构

建议使用 Rust workspace：

```text
crates/
  core/
    消息协议、RunRequest、RunEvent、ConversationEngine、AgentLoop 预留

  protocol/
    public request/response schemas
    event frames
    thread/turn/item lifecycle contracts
    backward-compatible API types

  providers/
    Provider trait
    openai-compatible adapter
    ollama adapter
    anthropic adapter
    gemini adapter
    deepseek adapter
    qwen adapter

  tools/
    Tool trait
    shell tool
    filesystem tool
    git tool
    http tool
    mcp tool adapter

  policy/
    approval rules
    dangerous command detection
    permission model
    secret masking

  secrets/
    env var resolution
    OS keychain integration
    encrypted or permission-checked file fallback
    secret redaction

  context/
    file context
    project context
    directory summary
    token budget
    source references

  tasks/
    Task model
    task lifecycle
    background task registry
    cancellation and pause/resume
    task dependencies
    task event routing

  agents/
    AgentProfile
    AgentLoop
    agent state
    handoff records
    role prompts
    agent run controls

  memory/
    memory item schema
    scoped memory store
    recall planner
    embedding index adapter
    memory write proposals
    memory review workflow

  skills/
    skill manifest
    prompt and instruction packs
    workflow templates
    context loaders
    tool requirements
    skill fixtures and evaluations

  swarm/
    agent role definitions
    task graph
    scheduler
    handoff protocol
    consensus and review rules
    swarm trace adapter

  learning/
    learning ledger
    trace mining
    failure pattern extraction
    improvement proposals
    eval generation
    approved artifact updates

  storage/
    config
    sessions
    traces
    sqlite repository
    jsonl writer

  replay/
    fixture replay
    golden trace tests
    provider mocks

  runtime_api/
    local HTTP/SSE server
    runtime thread endpoints
    task endpoints
    usage and health endpoints
    editor/client integration

  windows/
    window model
    pane and tab state
    layout tree
    focus history
    task-window bindings
    command palette state

  cli/
    headless runner
    chat command
    replay command
    task command
    doctor command
    config inspection

  tui/
    Ratatui app
    layout
    keymap
    widgets
    event rendering
    task switcher
    window manager

  diagnostics/
    LSP hooks
    post-edit diagnostics
    toolchain checks
    workspace health

  snapshots/
    workspace checkpoint
    side-git snapshots
    restore/revert support
```

原则：

- 每个 crate 一个清晰职责。
- 每个 crate 都应有 README，说明职责、边界和禁止事项。
- 不允许出现巨大 `app.rs` 或跨层循环依赖。

## 6. 核心协议草案

### 6.1 RunEvent

所有模型输出、工具请求、审批、错误和完成状态都通过统一事件流传递。

```rust
pub enum RunEvent {
    AssistantDelta {
        text: String,
    },
    ToolCallRequested {
        call: ToolCall,
    },
    SkillActivated {
        skill_id: String,
        version: Option<String>,
    },
    SkillStepStarted {
        skill_id: String,
        step_id: String,
    },
    TaskCreated {
        task_id: String,
        kind: TaskKind,
    },
    TaskStarted {
        task_id: String,
    },
    TaskPaused {
        task_id: String,
    },
    TaskResumed {
        task_id: String,
    },
    TaskCompleted {
        task_id: String,
        outcome: TaskOutcome,
    },
    TaskCancelled {
        task_id: String,
        reason: Option<String>,
    },
    WindowOpened {
        window_id: String,
        kind: WindowKind,
        task_id: Option<String>,
    },
    WindowFocused {
        window_id: String,
    },
    WindowClosed {
        window_id: String,
    },
    WindowLayoutChanged {
        layout_id: String,
    },
    AgentStarted {
        agent_id: String,
        role: String,
    },
    AgentHandoff {
        from_agent_id: String,
        to_agent_id: String,
        handoff_id: String,
    },
    AgentCompleted {
        agent_id: String,
        outcome: AgentOutcome,
    },
    MemoryRecall {
        scope: MemoryScope,
        item_ids: Vec<String>,
    },
    MemoryWriteProposed {
        proposal_id: String,
        scope: MemoryScope,
    },
    MemoryWriteApplied {
        item_id: String,
        scope: MemoryScope,
    },
    ToolCallApproved {
        call_id: String,
    },
    ToolCallDenied {
        call_id: String,
        reason: String,
    },
    ToolResult {
        call_id: String,
        result: ToolResult,
    },
    Usage {
        input_tokens: Option<u64>,
        output_tokens: Option<u64>,
    },
    SwarmTaskStarted {
        task_id: String,
        strategy: SwarmStrategy,
    },
    SwarmAgentEvent {
        task_id: String,
        agent_id: String,
        event: SwarmChildEvent,
    },
    SwarmTaskCompleted {
        task_id: String,
        outcome: SwarmOutcome,
    },
    LearningObservation {
        observation_id: String,
        source_trace_id: String,
    },
    LearningProposalCreated {
        proposal_id: String,
        target: LearningTarget,
    },
    LearningProposalApplied {
        proposal_id: String,
        target: LearningTarget,
    },
    Error {
        error: NormalizedError,
    },
    Done,
}
```

`SwarmChildEvent` 应是对单个 agent 运行事件的扁平包装，而不是无限递归的 swarm 嵌套事件。这样 trace 更容易序列化、过滤和回放。

### 6.2 Provider trait

```rust
#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    async fn stream(&self, request: RunRequest) -> Result<RunEventStream, ProviderError>;
}
```

Provider 只能输出标准事件，不负责工具执行，也不实现 agent loop。

### 6.3 Tool trait

```rust
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn descriptor(&self) -> ToolDescriptor;

    async fn execute(&self, input: ToolInput) -> Result<ToolResult, ToolError>;
}
```

Tool 的执行入口应只被 tool runtime 调用，不能从 UI 或 provider 直接调用。

### 6.4 PolicyDecision

```rust
pub enum PolicyDecision {
    Allow,
    Deny { reason: String },
    AskUser { risk: RiskSummary },
}
```

后续可以扩展为 workspace-scoped、tool-scoped、command-scoped 的权限模型。

### 6.5 Skill

Skill 是一组可复用能力包，用来把特定任务的提示词、上下文加载方式、工具需求、工作流步骤、验收标准和回放测试封装起来。

Skill 不是无约束插件。默认情况下，skill 不能任意执行代码，也不能绕过 policy 调用工具。它只能声明自己需要哪些工具、上下文和步骤，实际执行仍由 core、tool runtime 和 policy 接管。

一个 skill 应至少包含：

```text
skill.toml
README.md
instructions.md
workflows/
fixtures/
evals/
```

`skill.toml` 草案：

```toml
id = "code-review"
name = "Code Review"
version = "0.1.0"
description = "Review code changes and produce prioritized findings."

[requirements]
tools = ["git.diff", "filesystem.read"]
context = ["workspace"]

[policy]
default_permission = "ask"
network = "deny"
write_files = "deny"
```

Skill 的核心边界：

- skill 可以声明提示词、步骤、工具需求和上下文策略。
- skill 可以定义 slash command，例如 `/skill use code-review`。
- skill 可以提供 fixtures 和 evals，帮助 AI 修改 skill 后验证行为。
- skill 不能直接执行 shell、写文件、访问网络或调用 MCP。
- skill 的所有工具调用必须转成标准 ToolCall，并经过 policy。
- skill 激活和每个步骤都必须写入 trace。

### 6.6 Agent

Agent 是带有角色、目标、模型 profile、工具权限、上下文范围、记忆范围和停止条件的执行单元。

Agent 不等于 provider，也不等于一个聊天 session。Agent 是 core 里的可执行工作单元，负责把目标拆成模型请求、工具调用、记忆召回和结果输出。

`AgentProfile` 草案：

```toml
id = "reviewer"
role = "reviewer"
model_profile = "gpt-fast"
skills = ["code-review"]
memory_scopes = ["workspace", "project"]
tool_scopes = ["filesystem.read", "git.diff"]
max_steps = 8
```

Agent 的边界：

- agent 可以调用 provider，但不能绕过 provider adapter。
- agent 可以请求工具，但不能绕过 policy。
- agent 可以请求记忆召回，但不能直接读取所有 memory store。
- agent 可以提出记忆写入，但默认应先进入 review/proposal。
- agent 的每一步都必须进入 trace。

### 6.7 多 Agent 模式

多 agent 模式是多个 AgentProfile 协作完成任务。它比单 agent loop 更强，但比蜂群模式更显式、更可控。

第一阶段多 agent 模式应支持：

- `sequential_handoff`：一个 agent 完成后结构化交接给下一个 agent。
- `parallel_compare`：多个 agent 独立产出方案，再由用户或 reviewer 选择。
- `reviewer_gate`：worker 产出结果，reviewer 审查，必要时返回修改。
- `debate`：两个或多个 agent 对关键设计分歧给出论证，最后由主 agent 汇总。

多 agent 模式必须满足：

- 每个 agent 的角色、模型、工具权限、记忆范围显式配置。
- handoff 必须结构化，不能只靠自然语言粘贴上下文。
- 并发数和总 token 成本必须有限制。
- 每个 agent 的 trace 可单独回放，也能合并成任务级 trace。

### 6.8 Memory

记忆系统负责跨 session 保存和召回有价值的信息，但不能污染当前对话或跨越不该跨越的作用域。

记忆类型建议分为：

- `working`：当前任务短期工作记忆，随任务结束可丢弃。
- `episodic`：一次运行或一次任务的事件摘要，来自 trace。
- `semantic`：稳定事实、用户偏好、项目约定。
- `procedural`：可复用步骤、调试经验、工作流知识。
- `skill_memory`：某个 skill 的经验、失败案例和 eval 结果。

`MemoryItem` 草案：

```rust
pub struct MemoryItem {
    pub id: String,
    pub scope: MemoryScope,
    pub memory_type: MemoryType,
    pub content: String,
    pub source_trace_id: Option<String>,
    pub confidence: f32,
    pub created_at: Timestamp,
    pub expires_at: Option<Timestamp>,
    pub tags: Vec<String>,
}
```

`MemoryScope` 至少应包含：

```rust
pub enum MemoryScope {
    User { user_id_hash: String },
    Workspace { path_hash: String },
    Project { project_id: String },
    Skill { skill_id: String },
    Agent { agent_id: String },
    Session { session_id: String },
}
```

`User` 级记忆也不应被理解成无边界全局记忆。实现时应使用类似 `Subject` 的显式主体概念，把 user、workspace、project、app/channel 等边界组合进查询条件；如果未来支持多应用或多入口，同一用户的记忆也应按入口作用域隔离。

记忆写入策略：

- 默认不自动写长期记忆。
- 模型可提出 `MemoryWriteProposed`。
- 用户可批准、编辑或拒绝。
- 高频低风险的 task trace 摘要可以先进入 working/episodic memory。
- semantic/procedural memory 必须有来源 trace 或用户确认。

记忆召回策略：

- 召回前先判断 scope。
- 召回结果要带来源、置信度和原因。
- TUI 要能展示和关闭某条召回记忆。
- 记忆召回不能改变工具权限。

### 6.9 Learning

自学习系统负责从运行历史中提取可复用改进，但它不是自动自我改代码。

学习目标包括：

- 发现常见失败模式。
- 生成新的 replay fixture。
- 改进 skill instructions。
- 建议新的 policy rule。
- 建议 provider adapter 兼容性修复。
- 生成 eval case。
- 提出 memory 写入或清理建议。

学习产物建议分为：

- `MemoryProposal`
- `SkillPatchProposal`
- `PolicyRuleProposal`
- `EvalCaseProposal`
- `ProviderCompatibilityNote`
- `WorkflowImprovementProposal`

自学习必须有 ledger：

```text
learning-ledger/
  observations.jsonl
  proposals.jsonl
  approvals.jsonl
  applied.jsonl
```

自学习默认模式：

- 只观察，不自动修改。
- 只提案，不自动应用。
- 应用前必须跑 replay/eval。
- 所有学习提案必须可追溯到 trace、用户反馈或 eval 失败。
- 已应用的学习产物必须版本化。

### 6.10 Swarm

蜂群模式是多 agent 协作执行模式，适合复杂任务拆分、并行探索、交叉评审和结果汇总。

蜂群模式不应在 v0.1 实现，但 core 协议要预留。它必须建立在多 agent 模式、tool runtime、policy、memory 和 trace 之上，而不是单独实现一套并行执行逻辑。

蜂群模式的基础概念：

- `SwarmTask`：用户目标和约束。
- `SwarmPlan`：任务拆解后的有向任务图。
- `SwarmAgent`：带角色、模型 profile、上下文范围和工具权限的执行单元。
- `SwarmScheduler`：负责派发、并发限制、超时和取消。
- `Handoff`：agent 之间的结构化交接记录。
- `Consensus`：多 agent 结果合并、投票或主审裁决。
- `SwarmTrace`：完整记录每个 agent 的输入、输出、工具调用、审批和结论。

蜂群模式必须遵守：

- 每个 agent 的上下文范围要显式声明。
- 每个 agent 的工具权限要显式声明。
- 每个 agent 的记忆范围要显式声明。
- 所有 agent 的工具调用仍然经过统一 policy。
- swarm 只能组合已有 Provider、Tool、Skill、Memory 和 AgentLoop。
- swarm 结果必须可回放，不能依赖临时 UI 状态。
- 默认并发数必须有限制，避免 API 成本和工具执行失控。

蜂群模式早期可以先支持三种策略：

- `parallel_explore`：多个 agent 分别探索不同问题，最后汇总。
- `review_then_fix`：一个 agent 实现，另一个 agent 审查，再由主 agent 修正。
- `planner_workers_reviewer`：planner 拆任务，workers 执行，reviewer 汇总和质检。

### 6.11 Task

Task 是所有可运行工作的统一抽象。聊天、agent run、tool execution、replay、swarm、learning job 都应该是 Task，而不是散落在不同模块里的异步任务。

Task 类型建议包括：

- `chat`：普通模型对话。
- `agent_run`：单 agent 执行。
- `multi_agent_run`：显式多 agent 协作。
- `swarm_run`：蜂群调度任务。
- `tool_run`：长时间工具执行。
- `replay_run`：trace/fixture 回放。
- `learning_job`：自学习分析和提案生成。

`TaskStatus` 至少应包含：

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

Task 的核心要求：

- 每个 task 有稳定 `task_id`。
- 每个 task 可以绑定一个或多个窗口。
- 每个 task 可以被暂停、恢复、取消。
- task 可以声明父子关系和依赖关系。
- task 的所有 RunEvent 都写入 trace。
- 后台 task 不能绕过 policy。
- TUI 关闭窗口不等于取消 task，除非用户明确选择取消。

### 6.12 Window

Window 是 TUI 里的可恢复视图，不是运行时任务本身。

Window 类型建议包括：

- `chat`：对话主窗口。
- `task_list`：任务列表。
- `trace`：当前任务事件流。
- `approval`：工具审批。
- `diff`：代码修改预览。
- `logs`：后台任务日志。
- `memory`：记忆召回和写入提案。
- `skills`：skill 列表和详情。
- `agents`：agent 状态和 handoff。
- `swarm`：蜂群任务图。
- `settings`：配置和 profile。
- `help`：快捷键和命令。

Window 管理要求：

- 支持 tabs、panes、overlay 三种基础形态。
- 支持 focus stack，用户能快速回到上一个窗口。
- 支持 task-window binding，窗口可以绑定或解绑任务。
- 支持 layout tree 序列化，重启后恢复布局。
- 支持命令面板创建、切换、关闭窗口。
- 支持只读窗口，例如 trace/replay；也支持可交互窗口，例如 approval/chat。
- 支持小屏降级，窄终端下自动转成单列 tab 模式。

第一阶段不需要做复杂 tiling window manager，但数据模型必须避免把窗口状态硬编码到一个 `AppState` 大结构里。

## 7. 配置和密钥

配置文件建议使用：

```text
~/.config/tessera/config.toml
```

配置只保存：

- provider 名称
- base_url
- model profiles
- 默认模型
- UI 偏好
- policy 规则
- skill registry 路径
- task 并发、保留和恢复策略
- window 默认布局和快捷键
- memory store 路径和默认作用域
- learning 默认模式
- agent profiles
- swarm 默认并发和成本限制
- secret 的环境变量名或 keychain 引用

不应明文保存 API key。API key 优先来自：

- 环境变量
- 系统 keychain
- 外部 secret provider

示例：

```toml
[providers.openai_compatible]
type = "openai-compatible"
base_url = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"

[[profiles]]
name = "gpt-fast"
provider = "openai_compatible"
model = "gpt-4.1-mini"

[[profiles]]
name = "local-qwen"
provider = "ollama"
model = "qwen2.5-coder"

[skills]
paths = ["~/.config/tessera/skills", "./.tessera/skills"]

[tasks]
max_background_tasks = 4
restore_running_tasks = false
keep_completed = 100

[windows]
default_layout = "single"
restore_layout = true

[memory]
store = "~/.local/share/tessera/memory"
default_scopes = ["workspace", "project"]
write_mode = "propose"

[learning]
mode = "propose_only"
auto_apply = false

[agents.reviewer]
role = "reviewer"
model_profile = "gpt-fast"
skills = ["code-review"]
memory_scopes = ["workspace", "project"]
tool_scopes = ["filesystem.read", "git.diff"]

[swarm]
max_parallel_agents = 3
default_strategy = "review_then_fix"
```

## 8. Session 和 Trace

### 8.1 Session

Session 保存用户可见的会话状态：

- session id
- title
- messages
- selected profile
- active skill
- active task
- open windows
- focused window
- layout tree
- active agents
- memory references
- swarm task references
- context references
- created_at / updated_at

### 8.2 Trace

Trace 保存完整运行过程：

- request
- provider
- model
- normalized events
- skill activation
- skill step
- task lifecycle
- window lifecycle
- window-task binding
- agent run
- agent handoff
- memory recall
- memory write proposal
- swarm task
- swarm agent events
- learning observation
- learning proposal
- raw provider metadata 的安全子集
- tool call
- policy decision
- approval result
- tool result
- usage
- latency
- errors

Trace 使用 append-only JSONL，便于回放、审计和 AI 调试。

### 8.3 Durable Runtime Model

参考 DeepSeek-TUI 的运行时经验，session 和 trace 还不够。新设计应增加稳定的 runtime 数据模型：

- `Thread`：一次可恢复的会话/工作流容器。
- `Turn`：一次用户输入触发的模型运行。
- `Item`：turn 内的可观察单元，例如用户消息、assistant delta、tool call、file change、command execution、approval、error。
- `Task`：可排队、后台执行、取消和恢复的工作对象，可关联 thread/turn。
- `Artifact`：大输出、日志、patch、测试报告、子 agent transcript 的外部化引用。

这样 TUI、CLI、HTTP/SSE API、replay runner 和未来桌面/编辑器客户端可以共享同一套生命周期语义，避免每个入口各自发明状态。

第一版不必实现完整 runtime API，但 v0.1 的类型设计要预留：

- 单调递增 event sequence。
- `since_seq` 增量重放。
- turn/item lifecycle status。
- schema_version。
- 大输出 artifact reference。
- interrupt/cancel 语义。

### 8.4 Runtime API

新设计不应只面向交互式 TUI。应预留本地 runtime API，供桌面工作台、编辑器插件、自动化任务和测试 harness 使用。

建议支持：

- `doctor --json`：机器可读健康检查。
- `serve --http`：本地 HTTP/SSE runtime API。
- `serve --mcp`：把本工具作为 MCP server 暴露给其他客户端。
- `serve --acp` 或兼容协议：后续接编辑器 agent client。

API 默认只绑定 localhost。任何非本机访问都必须显式配置认证 token。

### 8.5 Tool Surface

工具设计要避免“工具越多越好”。参考 DeepSeek-TUI 的教训，工具面应有明确原则：

- 有结构化输出价值的能力，优先做专用 tool。
- 长尾命令继续走 shell，不盲目包装。
- 避免两个工具做同一件事，减少模型选择噪音。
- 大输出必须走 artifact 或 handle，不直接塞回 transcript。
- 长任务必须转成 background task，再通过 wait/read 获取进度。
- 验证命令要能作为 gate 记录到 task。

v0.1 可以只定义 ToolDescriptor 和 ToolResult；但 tool surface 的治理原则要先写进 prompt 和贡献规范。

## 9. 第一版功能范围

### 9.1 v0.1

目标：建立稳定核心骨架和可用聊天 TUI。

- Rust workspace
- `protocol` 基础事件和 runtime 类型
- `core` 事件协议
- `providers` 支持 OpenAI-compatible
- `providers` 支持 Ollama
- `secrets` 环境变量读取和本地安全存储占位
- headless `cli chat`
- `cli doctor --json` 最小健康检查
- Ratatui TUI
- 流式输出
- 本地 session 保存
- JSONL trace
- 基础 task id 和 window id 数据模型
- `/model`
- `/new`
- `/save`
- `/export`
- 基础测试和 provider mock

### 9.2 暂不做

v0.1 不做：

- 多 agent
- 自动改代码
- 复杂 RAG
- 插件市场
- 云同步
- 长期记忆
- 多任务后台执行 runtime
- 复杂多窗口布局
- 多 agent 执行 runtime
- 自学习执行 runtime
- skill 执行 runtime
- 蜂群模式执行 runtime
- 自动执行 shell
- 完整 HTTP/SSE runtime API
- MCP 完整生态

这些能力要等核心协议、trace、policy 稳定后再进入。

## 10. 后续路线

### v0.2：上下文工作台

- `/context add <file>`
- `/context remove`
- 上下文预览
- token budget
- 文件引用来源
- runtime Thread/Turn/Item schema
- runtime event `since_seq` replay
- memory schema 草案
- skill manifest schema 草案
- 内置只读 skill registry
- task registry v1
- task list window
- basic tab/window model
- provider mock tests
- replay runner
- `serve --http` 只读健康和 thread/event 查询

### v0.3：工具审批

- shell tool
- filesystem read/write tool
- git tool
- http tool
- artifact/handle for large outputs
- verification gate record
- approval UI
- approval window
- background task cancellation
- dangerous command detection
- tool trace

### v0.4：MCP

- MCP client
- MCP server registry
- MCP tool 到内部 Tool 协议的 adapter
- MCP tool approval
- MCP trace
- memory runtime v1
- memory recall preview
- memory write proposal
- trace window
- logs window

### v0.5：单 agent loop

- planner / executor 基础循环
- tool result feeding
- max step limit
- stop condition
- task trace
- skill runtime v1
- `/skill list`
- `/skill use <id>`
- skill fixtures
- task pause/resume
- task-window binding

### v0.6：多 agent 模式

- `agents` crate 完整 runtime
- `sequential_handoff`
- `parallel_compare`
- `reviewer_gate`
- `debate`
- agent trace replay
- agent memory scope control
- agents window
- handoff window

### v0.7：项目级 coding agent

- workspace scan
- patch preview
- diff apply
- test runner
- checkpoint
- rollback
- run summary
- diff window
- test output window

### v0.8：蜂群模式

- `swarm` crate
- `/swarm start`
- `parallel_explore`
- `review_then_fix`
- `planner_workers_reviewer`
- agent role profiles
- handoff records
- swarm trace replay
- swarm graph window
- concurrency and cost guardrails

### v0.9：自学习系统

- `learning` crate
- trace mining
- failure pattern extraction
- skill improvement proposals
- policy rule proposals
- eval case generation
- learning ledger
- replay/eval before apply

## 11. AI 友好开发规范

为了让 AI 长期参与开发，项目需要强约束：

- 每个 crate 有 README。
- 每个核心类型有清楚文档注释。
- 跨 crate 接口保持简单，少用复杂泛型和宏。
- 优先使用 `trait + enum + struct`，避免过度抽象。
- provider adapter 不得执行工具。
- TUI 不得调用模型 API。
- tool 执行不得绕过 policy。
- 所有核心行为必须有 fixture 或 unit test。
- 所有模型交互都必须能生成 trace。
- 所有 trace 都应能被 replay runner 消费。
- task 是运行时对象，window 是视图对象，不能混在一个状态结构里。
- 关闭窗口不能隐式取消后台 task。
- window layout 必须可序列化，不能只保存在临时 UI 变量里。
- agent profile 必须显式声明模型、工具、记忆和 step 限制。
- memory 写入长期存储默认走 proposal，不静默沉淀。
- skill 必须带 README、fixtures 和 evals，避免只靠提示词描述。
- swarm 策略必须能用离线 fixture 验证，不能只依赖真实 API。
- learning 只能基于 trace、反馈或 eval 生成提案，不能凭空改行为。
- 大文件要拆分，避免 AI 一次无法理解。

## 12. 风险和规避

### 12.1 Ratatui 应用膨胀

风险：UI、状态、业务逻辑写进一个大文件。

规避：

- UI 只订阅 RunEvent。
- 按 layout、widgets、keymap、app_state 拆分。
- core 不依赖 tui。

### 12.2 Provider 差异污染核心

风险：不同 provider 的工具调用、token、错误结构进入 core。

规避：

- provider 只输出 RunEvent。
- provider 私有结构只允许存在 adapter 内。
- 错误统一成 NormalizedError。

### 12.3 Agent 后补导致重构

风险：v0.1 只按聊天工具设计，后续 agent loop 无法接入。

规避：

- 第一版就保留 ToolCallRequested、ToolResult、PolicyDecision。
- ConversationEngine 不假设只有文本回复。
- Session 和 Trace 区分存储。

### 12.4 工具执行安全失控

风险：shell/file/git 能力上线后绕过审批。

规避：

- tool runtime 是唯一执行入口。
- policy 是 tool runtime 的前置依赖。
- TUI 只展示审批和结果。

### 12.5 无法稳定复现问题

风险：provider 行为依赖实时 API，请求失败难以调试。

规避：

- JSONL trace 从第一天实现。
- replay runner 从 v0.2 前完成。
- provider mock 由 trace 驱动。

### 12.6 Skill 变成不受控插件

风险：skill 如果可以任意执行代码，会绕开 policy 和 trace，最终变成安全和可维护性问题。

规避：

- skill 默认只声明能力、步骤、提示词、上下文和工具需求。
- skill 的工具调用必须转换成标准 ToolCall。
- skill 不能直接执行脚本，除非脚本被注册为 Tool 并经过 policy。
- skill registry 要支持版本、来源和启用状态。

### 12.7 蜂群模式成本和状态失控

风险：多 agent 并发会放大 token 成本、工具风险和状态复杂度。

规避：

- 默认并发数保守，例如 3。
- 每个 SwarmTask 有最大 step、最大 token、最大耗时限制。
- 每个 agent 的上下文和工具权限显式声明。
- 所有 agent event 都进入 SwarmTrace。
- 先支持少量固定策略，再考虑自定义编排。

### 12.8 记忆污染和跨作用域泄漏

风险：记忆如果没有明确作用域，会把某个项目、某个用户、某个 skill 的事实错误地带入另一个任务。

规避：

- MemoryScope 是必填字段。
- 长期记忆默认按 workspace/project/skill 作用域隔离。
- 召回结果必须显示来源、作用域和置信度。
- identity、secret、credential、临时错误日志等敏感内容默认不自动进入长期记忆。
- 用户可以查看、禁用、删除和导出记忆。

### 12.9 自学习导致行为漂移

风险：系统如果自动修改 prompt、skill、policy 或记忆，可能越学越偏，且难以回滚。

规避：

- 默认 `propose_only`。
- 所有学习建议进入 learning ledger。
- 应用前必须经过用户批准。
- 应用前必须跑 replay/eval。
- 已应用变更必须有版本和回滚路径。

### 12.10 多 Agent 结果冲突

风险：多个 agent 可能给出冲突结论，或者互相放大错误假设。

规避：

- 多 agent 输出必须保留 agent_id、role 和 evidence。
- reviewer 或主 agent 只能汇总有证据的结论。
- 关键结论要支持“保留分歧”，不能强行合并。
- TUI 要展示 agent 之间的 handoff 和分歧点。

### 12.11 多任务状态混乱

风险：后台任务、agent、tool、replay 如果各自管理生命周期，会导致取消、恢复、trace 和错误处理不一致。

规避：

- 所有可运行工作统一建模为 Task。
- task lifecycle 必须进入 trace。
- task cancellation 必须是显式事件。
- task registry 是唯一任务索引。
- 后台 task 数量必须有限制。

### 12.12 多窗口和运行时耦合

风险：窗口关闭、tab 切换或布局变化如果直接驱动运行时，会造成误取消、重复请求或状态丢失。

规避：

- Window 只绑定 task，不拥有 task。
- 关闭 window 不取消 task。
- layout tree 独立序列化。
- 窄屏模式和多 pane 模式共享同一 window model。
- TUI 通过 RunEvent 和 task registry 渲染状态，不直接持有 provider/tool runtime。

## 13. 推荐的第一阶段验收标准

v0.1 完成时应满足：

- 可以通过 CLI 发起一次流式对话。
- 可以通过 TUI 完成一次流式对话。
- 可以在 OpenAI-compatible 和 Ollama profile 之间切换。
- 每次运行都有 JSONL trace。
- 会话可以保存和恢复。
- 基础 TaskId、WindowId、task-window binding schema 已定义。
- TUI 支持至少一个主聊天窗口和一个可扩展窗口模型。
- `cargo test` 覆盖 core、provider mock、storage 基础行为。
- TUI 层没有 provider SDK 依赖。
- provider adapter 没有 tool 执行逻辑。
- RunEvent 已预留 skill 和 swarm 事件，不要求 v0.1 执行。
- RunEvent 已预留 agent、memory 和 learning 事件，不要求 v0.1 执行。
- RunEvent 已预留 task 和 window 事件，不要求 v0.1 完整多任务执行。
- config schema 已预留 skill registry、task、window、memory、agent、learning 和 swarm guardrails。
- 没有 API key 明文进入仓库或 session 文件。

## 14. DeepSeek-TUI 对照审查

本节基于对 `Hmbown/deepseek-tui` 当前源码和文档的阅读，用来反向校验本设计。

### 14.1 DeepSeek-TUI 做得好的地方

DeepSeek-TUI 的优势不是“聊天 TUI”，而是完整本地 agent 产品：

- Rust workspace 已经拆出 `agent`、`app-server`、`config`、`core`、`execpolicy`、`mcp`、`protocol`、`secrets`、`state`、`tools`、`tui` 等 crate。
- 有真实的安装分发链路：Cargo、npm wrapper、Homebrew、Docker、GitHub Releases。
- 有本地 runtime API：HTTP/SSE、MCP server、ACP stdio、`doctor --json`。
- 有 durable Thread/Turn/Item 模型，并支持事件 replay。
- 有 durable background task manager，任务能关联 thread/turn、timeline、artifacts、verification gates。
- tool surface 有明确治理原则：结构化工具优先，shell 兜底，避免重复别名，大输出用 handle/artifact。
- sub-agent 不是一次性函数调用，而是 persistent session，支持 role、fork_context、concurrency cap、structured output。
- 有 MCP manager、skills discovery、memory、sandbox、LSP diagnostics、workspace snapshot/restore、cost tracking、localization、theme 和 onboarding。

这些证明本项目如果要做“质量优先”的通用 TUI，不能只设计聊天流和模型 adapter。runtime、task、trace、approval、tool surface、分发和诊断都是一等产品能力。

### 14.2 DeepSeek-TUI 暴露的问题

DeepSeek-TUI 也暴露了一个成熟项目常见问题：能力很多，但历史演进导致边界还在迁移。

当前架构文档明确说明：

- `crates/tui` 仍然是 TUI、runtime API、task manager、tool execution loop 的真实运行时来源。
- 其他 workspace crates 正在增量拆分，但还不是唯一 truth source。
- `crates/tui/src/tui/app.rs`、`ui.rs`、`core/engine.rs`、`turn_loop.rs`、`task_manager.rs`、`runtime_threads.rs`、`tools/subagent/mod.rs` 都已经很大。
- 过去的 swarm agent surface 已经移除，当前保留的是 persistent sub-agent sessions 和 RLM sessions。
- memory 是 opt-in 的单个用户 Markdown 文件，注入 system prompt；这简单有效，但不是 scoped memory / searchable memory / proposal-based memory。

这些问题给本项目的启示是：

- 不能让 `tui` crate 成为真实运行时来源。
- 不能先把功能堆进 TUI，再“以后拆出来”。
- swarm 不应早于稳定的 sub-agent/session/task runtime。
- memory 第一版可以简单，但数据模型不能锁死成一个全局 Markdown 文件。

### 14.3 当前新设计的优点

相对 DeepSeek-TUI，本设计的优点是边界更早、更硬：

- `TUI 只是 View` 是核心原则，避免运行时沉淀在 UI。
- Task 和 Window 分离，避免多任务、多窗口和后台执行互相绑死。
- 从一开始区分 `Session`、`Trace`、`Thread`、`Turn`、`Item`、`Task`、`Artifact`。
- Memory 有 scope、type、confidence、source trace，不默认全局共享。
- Learning 默认 proposal-only，避免系统静默改 prompt、policy、skill 或 memory。
- Skill 默认不是可执行插件，必须通过 tool runtime 和 policy。
- 多 agent 和 swarm 分层：先 persistent sub-agent / handoff，再自动 swarm scheduler。
- Provider adapter 不污染 core，适合做通用模型 TUI，而不是 DeepSeek 专用 TUI。

### 14.4 当前新设计的缺点

当前设计也有明显风险：

- crate 规划太完整，容易在 v0.1 过度工程化。
- `RunEvent` 预留了太多未来事件，可能让第一版协议复杂度过高。
- 对 runtime API、doctor、ACP/editor integration、install/update、localization、cost tracking、LSP diagnostics、sandbox、snapshot/rollback 的重视还不够。
- Skill 设计过早引入 `skill.toml`，可能不如先兼容社区通用的 `SKILL.md` frontmatter。
- Memory / Learning / Swarm 的理想设计很强，但如果没有 replay/eval 和 UI 审批体验，会变成空架构。
- 多 provider 通用性会牺牲 DeepSeek 专属能力，例如 reasoning blocks、prefix cache telemetry、large context 成本优化。
- 如果 v0.1 只做聊天和 schema，用户可能感知不到“质量优先”的价值。

### 14.5 优化后的设计取舍

结合 DeepSeek-TUI 的经验，新设计应调整为：

1. **v0.1 只实现最小真实 runtime，不实现所有 crate 的完整能力。**
   但 `protocol`、`core`、`storage`、`providers`、`cli`、`tui` 的边界必须真实存在。

2. **提前做 Thread/Turn/Item，而不是只有 Session/Trace。**
   这是 runtime API、任务回放、多窗口、后台任务、agent 和 replay 的共同底座。

3. **先做 persistent sub-agent，不急着做 swarm。**
   swarm 是调度策略；没有稳定 agent session、handoff、task manager、artifact 和 trace，swarm 只会制造并发混乱。

4. **Skill 先兼容 `SKILL.md`，再扩展 `skill.toml`。**
   `SKILL.md` frontmatter 更接近现有生态；`skill.toml` 可作为高级 manifest，用于声明工具权限、eval、fixtures 和 policy。

5. **Memory 分两层。**
   第一层支持简单 opt-in user memory；第二层再做 scoped searchable memory。即使第一层简单，也要在 schema 中保留 scope/source/approval 字段，避免锁死。

6. **Tool surface 要写进 prompt 和贡献规范。**
   不做“工具越多越强”。每个工具必须说明为什么比 shell 好，输出是否结构化，是否支持 artifact/handle。

7. **runtime API 要早于复杂多窗口。**
   多窗口如果没有 stable runtime API 和 event replay，只会变成 TUI 内部状态。先把 `doctor --json`、thread/event 只读查询和 replay 做稳。

8. **OS sandbox、secret、snapshot、diagnostics 要作为质量能力进入早期路线。**
   这些不是高级功能，而是 coding agent 的安全和恢复底座。

9. **保留 DeepSeek 专属能力的 adapter extension。**
   通用 provider trait 之外，要允许 provider-specific telemetry，例如 reasoning blocks、prefix cache、thinking effort、cost breakdown，但只能通过标准 extension metadata 暴露。

10. **只有一个真实运行时来源。**
    `tui`、`cli`、`runtime_api` 都只能调用同一个 core/runtime；不能各自维护状态机。

### 14.6 修正后的近期优先级

新的近期优先级应是：

1. 建立 Rust workspace 和真实 crate 边界。
2. 定义 `protocol`：Thread/Turn/Item/Task/Artifact/EventFrame。
3. 定义 `core`：ProviderAdapter、RunEvent、ConversationEngine。
4. 定义 `storage`：SQLite + JSONL + schema_version。
5. 实现 OpenAI-compatible + Ollama 的流式聊天。
6. 实现 `cli chat` 和 `cli doctor --json`。
7. 实现最小 TUI：chat window + task/window id schema。
8. 实现 replay fixture 和 provider mock。

这比“先写漂亮 TUI”更慢，但能避免 DeepSeek-TUI 当前正在经历的运行时拆分压力。

### 14.7 DeepSeek-TUI 解析稿补充吸收

后续对 DeepSeek-TUI 的介绍文章设计稿进一步暴露出若干可吸收的架构能力。Tessera 应吸收这些设计，但不能把 v0.1 做成 DeepSeek-TUI 的功能复刻。

应立即固化到 v0.1 协议和 trace 的能力：

- Provider capability schema：reasoning stream、cache telemetry、cost estimate、tool calling、context window 都应是 capability，而不是 UI 临时判断。
- Reasoning delta：思考块应作为可选标准事件进入 `RunEvent`，由 provider adapter 转换，TUI 只负责展示。
- Extended usage：usage 不只记录 input/output tokens，还应预留 cache read/write/miss tokens、latency、estimated cost 和 currency。
- RouteDecision：即使 v0.1 不做 Auto router，也应记录手动 profile resolution，未来 Auto routing 只扩展 strategy。
- Artifact handle：大输出、provider metadata、安全原始信息、未来 sub-agent transcript 都应外部化引用。

应进入 v0.2-v0.4 的能力：

- Model router 草案：借鉴 Auto 模式，但先不默认开启。路由输入、输出和 fallback 必须写 trace。
- Cost/cache telemetry UI：成本和 prefix cache 稳定性是用户可感知质量能力，应基于 trace 展示。
- Tool descriptor + policy gate + approval UI：Plan / Agent / trusted workspace 的权限边界应设计成统一 mode model。
- OS sandbox + workspace checkpoint：shell/file/git/apply-patch 上线前必须先有沙箱和可恢复快照。
- Runtime API：HTTP/SSE 和未来 ACP/editor integration 只能暴露 core runtime，不能另建状态机。
- Diagnostics/LSP：LSP 诊断应作为结构化 diagnostics event 进入 trace，而不是 TUI 内部提示。
- MCP adapter：MCP tool 必须转成 Tessera ToolCall，再经过 policy 和 trace。

应进入 v0.5+ 的能力：

- Persistent sub-agent sessions：sub-agent 是可追踪 task/session，不是一次性函数调用。
- `context_handle` / `handle_read`：父 agent 只接收 summary/evidence/metrics，大 transcript 通过 artifact handle 切片读取。
- Structured handoff：agent 交接必须结构化并可回放。
- Reviewer gate：worker/reviewer 模式早于 swarm scheduler。
- Swarm：只能建立在稳定 task、agent、handoff、artifact 和 trace 之上。

明确暂缓：

- v0.1 不做 Auto router。
- v0.1 不做 YOLO/trusted workspace 自动批准。
- v0.1 不做工具执行。
- v0.1 不做 sub-agent 并发。
- 不把 provider 价格硬编码进 core。
- 不把 Tessera 变成 DeepSeek 专用客户端。

这部分的详细采纳矩阵已单独沉淀到 `docs/deepseek-tui-lessons.md`，作为后续规划和实现审查的依据。

## 15. 下一步

下一步建议先完成更细的实施计划：

1. 确定项目名称和命令名。
2. 固化 workspace crate 列表。
3. 设计 protocol v0：Thread/Turn/Item/Task/Artifact/EventFrame。
4. 写出 v0.1 的核心类型定义。
5. 设计 trace JSONL schema。
6. 设计 config.toml schema。
7. 设计 skill manifest v0 草案。
8. 设计 agent profile v0 草案。
9. 设计 memory schema v0 草案。
10. 设计 learning ledger v0 草案。
11. 设计 task lifecycle v0 草案。
12. 设计 window model v0 草案。
13. 设计 swarm event 和 trace v0 草案。
14. 先实现 headless CLI。
15. 再实现 Ratatui TUI。

在进入实现前，应先审查本设计文档，确认技术选型和 v0.1 范围没有偏离“质量优先、agent-ready、AI 友好”的目标。
