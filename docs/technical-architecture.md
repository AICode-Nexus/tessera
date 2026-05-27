# Tessera Technical Architecture

日期：2026-05-22

## 1. 定位

Tessera 是一个 Rust-first、AI-friendly、agent-ready 的本地终端大模型工作台。

它不是单纯的聊天 TUI，也不是某个 provider 的客户端。它的核心是一个可审计、可回放、可扩展的本地 runtime。CLI、TUI、未来 GUI、replay runner、未来 runtime API、工具系统、agent 系统、memory 和 skill 都必须共享同一套协议和状态模型。

核心目标：

- Model-agnostic：支持多个 provider，但 core 不被 provider 私有结构污染。
- Headless-first：先有可测试的 headless runtime，再有 TUI 和 GUI。
- Replayable：所有运行都能通过 JSONL trace 回放和审计。
- Auditable：未来所有工具调用必须经过 policy gate。
- AI-friendly：代码边界小、协议清晰、fixture/replay 完整，方便 AI 稳定参与开发。
- Agent-ready：v0.1 先预留 agent 接入点；当前 v0.5 已收口为 no-tool single-agent loop、opt-in project instruction discovery/source reporting、explicit read-only Skill Runtime v1、background task ownership trace-backed foundation、no-tool chat/agent run owner attach/detach 和 runtime API/app-server DTO alignment；v0.6 已完成 structured handoff / reviewer gate、persistent sub-agent session metadata、runtime ownership metadata 和 transcript artifact lifecycle metadata 的 protocol、core helper、client projection 和 GUI bindings foundation；工具、executable skills、默认/全局指令加载、background reattach、persistent child-agent runtime 和 swarm 仍必须按门禁推进。

## 2. 技术选型

### 2.1 主语言：Rust

选择 Rust 是质量优先的结果。

理由：

- 适合构建长期维护的本地开发工具。
- 单文件二进制分发体验好。
- 类型系统适合固化协议，例如 `Thread`、`Turn`、`Item`、`Task`、`EventFrame`、`RunEvent`。
- 对本地文件、进程、终端、权限、安全和审计场景更可控。
- 能通过 crate 边界把系统拆成 AI 更容易理解和修改的小单元。

TypeScript 可以用于外部 bridge、协议验证、文档工具或未来 Web/desktop companion，但不作为主 runtime。

### 2.2 TUI：Ratatui

Ratatui 用于终端 UI，crossterm 用于终端输入和 alternate screen 控制。

约束：

- TUI 只做渲染、输入、焦点和展示。
- TUI 不调用 provider。
- TUI 不执行工具。
- TUI 不拥有真实 runtime 状态机。
- TUI 只订阅 core 事件并向 core 提交用户意图。

### 2.3 GUI：Tauri-first，但后置实现

GUI 不进入 v0.1 实现范围，但架构从现在起必须支持未来 GUI。

默认产品 GUI 方向：

- Tauri 2 + TypeScript/React/Vite shell。
- Rust core / future runtime API 仍然是唯一 runtime owner。
- Tauri Rust side 只做 typed command/event bridge。
- WebView frontend 只渲染 `ClientSnapshot` / `ClientProjection`，不调用 provider、不读 SQLite、不执行工具。

配套策略：

- v0.1 不引入 GUI toolkit 或 Web build system。
- 先把用户意图、消息投影、状态栏投影和任务投影做成 UI-neutral client model。
- TUI、未来 GUI、未来 runtime API 都消费同一套 EventFrame / TraceRecord 投影。
- GUI 只能作为 client shell，不拥有 provider、storage 或 task runtime。
- GUI 早期 Tauri spike 只允许接 mock/replay 或 read-only runtime，验证 typed IPC、布局、状态投影、分发体积和可访问性。

候选方向：

- Tauri：默认产品 GUI 方向，适合复杂工作台 UI、跨平台桌面壳、HTML 可访问性、截图自动化和 Web 组件生态。
- egui：内部诊断面板或轻量 inspector 候选，适合 Rust-first 调试工具，不作为产品 GUI 默认方向。
- GPUI：继续观察，适合未来原生高性能方向，但不进入 v0.2 默认路径。

在没有完成 client model 和 live event bridge 前，不应开始真实 GUI 功能开发。详细决策见 [ADR-001: GUI Architecture and Toolkit Direction](adr/ADR-001-gui-architecture-and-toolkit.md) 和 [GUI-Ready Architecture](gui-ready-architecture.md)。

### 2.4 Async Runtime：Tokio

Tokio 用于 provider streaming、storage coordination、TUI event loop 和未来 background task。

约束：

- Tokio 绑定不进入 `protocol`。
- 长任务必须通过 Task 生命周期表达。
- cancellation、timeout 和 backpressure 必须进入 core 层，而不是散落在 UI。

### 2.5 HTTP：Reqwest

Reqwest 用于 OpenAI-compatible、Ollama 和未来 provider adapter。

约束：

- HTTP client 只出现在 `providers` 或未来网络相关 adapter。
- provider response 必须转换成标准 `RunEvent`。
- request headers、API key、cookie 不进入 trace。

### 2.6 Serialization：Serde

Serde 用于协议、配置、trace、fixture 和 provider metadata 的安全子集。

约束：

- 所有持久化结构必须有 schema version。
- provider-specific metadata 只能进入 extension map。
- breaking schema change 必须有 migration 或 replay fallback。

### 2.7 Storage：SQLite + JSONL

JSONL trace 是事件真相。SQLite 是可重建索引。

Rust 侧通过 `rusqlite` 访问 SQLite。`rusqlite` 是访问层，SQLite 是实际数据库引擎和文件格式。发布构建使用 `rusqlite` 的 `bundled` feature，将 SQLite 编入二进制，降低用户机器缺少 `libsqlite3` 的安装风险。

理由：

- JSONL 适合 append-only trace、审计、diff 和 replay。
- SQLite 适合 thread/turn/item/task/artifact 查询。
- SQLite 损坏时可以从 JSONL 重建。
- `rusqlite` 比 async SQL 栈更轻，适合 v0.1 的本地索引用途。

约束：

- 任何 durable runtime event 先写 JSONL。
- SQLite 不应成为另一套事件事实。
- 大输出使用 artifact 引用，不直接写入 transcript。
- diff/test 等 artifact body 必须通过显式 artifact storage API 写入 `artifacts/`，trace 只记录 `ArtifactBodyRecord` handle、byte length、media type 和 redaction status。

### 2.8 CLI：Clap

CLI 是 headless runtime 的第一验证入口。

v0.1 必须包含：

- `tessera chat`
- `tessera doctor --json`

CLI 不能绕过 core 直接调 provider 或 storage internals。

### 2.9 Config：TOML

配置建议使用 TOML：

```text
~/.config/tessera/config.toml
```

配置保存 provider/profile、data dir、UI 偏好和 future guardrails。API key 只保存环境变量名或 keychain 引用，不保存明文。

### 2.10 Provider Capabilities

DeepSeek-TUI 的经验说明，现代 provider 差异不只体现在 endpoint 和 model name。reasoning stream、prefix cache、cost telemetry、context window、route strategy 都会影响真实体验。

Tessera 应把这些差异建模为 provider capability：

- `supports_streaming`
- `supports_reasoning_delta`
- `supports_cache_telemetry`
- `supports_cost_estimate`
- `supports_tool_calling`
- `max_context_tokens`
- `extension_metadata`

约束：

- capability 只描述 provider 能力，不改变 core 协议边界。
- provider 专属字段只能进入 extension metadata。
- TUI 可以展示 capability 派生状态，但不能依赖 provider 私有结构。
- v0.1 只定义 capability 和 trace 字段，不实现 Auto router。

### 2.11 Cache-stable context

Reasonix 官方架构进一步说明，prefix cache 的价值不来自 provider 自动开启缓存，而来自 client 能否长期保持可缓存字节稳定。Tessera 不能变成 DeepSeek-only，但 future context builder 必须 provider-neutral 地支持 cache-stable 运行方式。

约束：

- Stable prefix：系统提示词、工具描述、skill 摘要等稳定材料应在一次 session 内固定序列化，变化必须可追踪。
- Append-only transcript：历史事件优先追加，不原地重写、不重排；压缩应优先追加 summary 或 artifact handle。
- Volatile scratch：reasoning delta、临时计划、UI-only 状态默认不回灌到下一次 provider input。
- Visible telemetry：cache hit/miss、context usage、route/escalation reason 和 estimated cost 必须进入标准 event 或安全 extension，不能只存在于 TUI footer。
- No silent escalation：未来任何模型升档都必须对用户可见并写入 trace；无进展只读循环应先 stop / ask / summarize，而不是直接切更贵模型。

v0.1 不实现长期上下文构建器，但 protocol、trace 和 client projection 必须不阻断这条路径。详细采纳矩阵见 [Reasonix Lessons](reasonix-lessons.md)。

### 2.12 Coding Agent Surface Direction

Tessera 的长期目标不是只做一个聊天终端，而是形成 CLI、TUI、GUI、runtime API、skills、hooks、subagents 和 automations 都能共享的本地 coding-agent workbench。Codex CLI / App / App Server、Claude Code CLI / Desktop / Web、DeepSeek-TUI 和 Reasonix 的共同经验是：产品表面可以很多，但运行时对象必须少而硬。

需要提前固化的对象：

- `Thread` / `Turn` / `Item`：对话、推理、工具、审批、诊断和学习都挂在统一时间线。
- `Task`：chat run、agent run、subagent、tool run、automation job、learning job 都必须有可取消、可暂停、可恢复或可解释失败的生命周期。
- `TaskOwnership`：future background tasks must have execution owner, observer, heartbeat, lost-owner and reattach metadata in trace before GUI/app-server/automation can control them. Current v0.5 foundation provides the protocol events, core recorder/projection, client projection, CLI read-only owner listing, and automatic owner attach/detach around no-tool chat/agent runs, but not daemon ownership transfer or provider socket freezing.
- `Artifact`：diff、patch、test report、terminal output、browser evidence、subagent transcript、large provider metadata 都不应直接塞进上下文；v0.7 foundation adds explicit artifact body storage metadata and redaction status before any patch/test executor may consume those bodies.
- `Approval` / `ReviewerGate`：用户审批、reviewer gate、policy decision、GUI diff review 都必须 trace-backed；v0.6 foundation records handoff summaries, bounded evidence refs and reviewer decisions before any persistent sub-agent runtime.
- `ApplyPatchGate`：v0.7 preflight gate 验证 mutation request、patch proposal、scope、checkpoint、reviewer、policy、sandbox、worktree metadata 和可选 executor context 是否满足未来 executor 条件。默认请求仍只返回 preflight/dry-run readiness 并保持 `executor_blocked`；只有显式 isolated executor context、非 primary/local root label、单文件 create/modify dry-run、checkpoint/reviewer/policy/sandbox gates 同时满足时，才可返回 executor-ready metadata。它不读取或写入 workspace、不创建 worktree、不运行 patch/test/Git 命令、不恢复 checkpoint、不驱动 UI。
- `TraceApplyPatchResolver`：v0.7 trace-driven apply-patch automation 的只读 resolver。它只消费 JSONL trace metadata，解析已审过的 workflow/scope/mutation request/patch proposal/checkpoint/reviewer/policy/sandbox 和 clean patch artifact metadata，并返回 typed envelope 给现有 apply-patch gate/executor 路径；它不读取 patch body bytes、不写 storage、不执行工具、不创建 worktree、不恢复 checkpoint、不 mutate Git、不驱动 UI。
- `ApplyPatchExecutor`：v0.7 executor helper 只允许在调用方显式提供的 isolated root 内执行单文件 UTF-8 text create/modify patch。它先复用纯内存 patch model，再验证 root kind、root label、allowed paths、canonical parent、symlinked parent/target，最后通过 target directory 内 temp file + rename 写入。它不创建 worktree、不运行 shell/test/Git、不 restore checkpoint、不写 storage、不驱动 UI。
- `SubagentSession`：current v0.6 foundation records and projects parent/child task linkage, caps, scope labels, transcript artifact handles, approval forwarding metadata, inactive-child policy and cancellation cascade metadata before any scheduler, fan-out or child execution runtime exists. It also defines core-owned, non-executing helpers for transcript artifact lifecycle metadata, approval-forwarding metadata, inactive-policy metadata, cancellation cascade metadata and child task owner attach/detach/heartbeat/lost/reattach metadata. Future persistent runtime must add executable scheduler coordination, durable task ownership persistence, heartbeat/lost-owner handling, automatic approval forwarding, inactive-child execution and executable cancellation/reattach handling before child provider calls.
- `InstructionSource` / `ContextReference`：`AGENTS.md`、未来 `CLAUDE.md`、skills、hook output、MCP metadata 都只能作为有来源、有上限、可审计的 context 输入。
- `RuntimeApi`：GUI、IDE、automation 和 remote client 只通过 typed messages / bounded queues / generated schemas 访问 core。Current v0.5 alignment provides `RuntimeApi*` DTOs, localhost-safe default bind/auth/queue metadata, generated TypeScript schema evidence, read-only event stream request mapping, and a bounded SSE frame buffer, but not a listening app-server.

约束：

- CLI 可以是脚本入口，也可以调用 core 的 `AgentLoop`；但不能自己实现 agent loop、provider routing 或 trace 写入旁路。
- GUI 可以是多线程、多项目、多 worktree 控制面，但不能持有真实 task/session 状态机。
- Hook 和 automation 只能订阅标准事件或提出 traced proposal，不能绕过 tool/policy/sandbox。
- Skills 优先兼容 `SKILL.md` progressive disclosure；脚本和引用文件仍受 policy、scope 和 redaction 约束。
- Subagent 是上下文隔离和结构化 handoff 机制，不是绕过 reviewer gate 的并发执行捷径。

详细方向见 [Coding-Agent Direction](coding-agent-direction.md)。

## 3. 整体架构

v0.1 的架构分为七层：

```text
User
  |
  v
CLI / TUI / future GUI
  |
  v
Core Runtime
  |
  +--> Providers
  |
  +--> Storage
  |
  +--> Config
  |
  v
Protocol
```

未来能力接入方式：

```text
Tools / Policy / Skills / Memory / Agents / Swarm / Learning
  |
  v
Core Runtime + Protocol + Trace
```

这些未来能力不得绕过 core、protocol、policy 和 trace。

DeepSeek-TUI 解析稿对 Tessera 的核心启发是：runtime 能力比 UI 外观更重要。Tessera 应优先吸收它的 durable task、runtime API、tool policy、sandbox、snapshot、sub-agent handle、MCP/ACP integration 和 distribution 设计，但按阶段纳入，避免 v0.1 失控。Reasonix 官方仓库进一步强化了 cache-stable context、ordered parallel dispatch、tool-call repair telemetry 和 visible cost control 的长期约束。Codex 和 Claude Code 生态进一步说明，CLI、GUI、skills、hooks、app-server、background tasks 和 subagents 都必须建立在同一套 task/policy/trace/runtime API 上。详细采纳矩阵见 [DeepSeek-TUI Lessons](deepseek-tui-lessons.md)、[Reasonix Lessons](reasonix-lessons.md) 和 [Coding-Agent Direction](coding-agent-direction.md)。

## 4. v0.1 Crate 结构

v0.1 首批只开必要 crate：

```text
crates/
  protocol/
  client/
  core/
  providers/
  storage/
  config/
  cli/
  tui/
  gui-bindings/
  gui-bridge/
apps/
  gui-tauri/
  # future, not v0.1:
  # full gui runtime integration
```

职责摘要：

- `protocol`：公共类型、ID、runtime schema、RunEvent、EventFrame、NormalizedError、context reference schema、diagnostics schema、memory proposal schema、skill manifest schema、sandbox profile schema、checkpoint schema、coding workflow metadata schema。
- `client`：UI-neutral intent、status/message/approval/memory proposal projection、ClientSnapshot；从 EventFrame / TraceRecord 生成 TUI 和未来 GUI 共享的 view model。
- `core`：运行生命周期、ConversationEngine、事件路由、provider/storage 协调、context workbench、draft model routing、no-progress loop signal、diagnostics reporter、只读 skill registry、explicit read-only SkillRuntimePlanner、metadata-only MCP adapter、sandbox profile planner、mutation enforcement planner、trace apply-patch resolver、checkpoint metadata planner、runtime HTTP/SSE shape helper、checkpoint metadata projection 和 metadata-only coding workflow coordinator。
- `providers`：Provider trait、OpenAI-compatible、Ollama、Mock provider。
- `storage`：JSONL trace writer、SQLite index、repository。
- `config`：配置读取、profile、data dir、secret env var 引用。
- `cli`：headless 命令入口和本地二进制命令编排。
- `tui`：Ratatui view，只保留终端输入、live event wrapper 和 terminal renderer。
- `gui-bindings`：从 Rust DTO 生成 GUI TypeScript bindings，输出到 `apps/gui-tauri/src/generated/bindings.ts`。
- `gui-bridge`：GUI typed command DTO、mock/replay projection、bounded event buffer；不接 provider/storage。
- `apps/gui-tauri`：Tauri 2 + React/Vite desktop/web shell，只消费 `gui-bridge` / `client` projection。

详细依赖方向见 [Crate Boundaries](crate-boundaries.md)。

## 5. Runtime Data Flow

一次 v0.1 chat run 的数据流：

```text
User input
  -> CLI/TUI/future GUI
  -> Core creates Task + Turn + UserMessage Item
  -> Core calls Provider
  -> Provider streams provider-specific chunks
  -> Provider adapter emits provider-neutral RunEvent
  -> Core wraps RunEvent into EventFrame
  -> Storage appends JSONL trace
  -> Storage updates SQLite index
  -> CLI/TUI/future GUI renders streamed events
  -> Replay can rebuild the run from JSONL
```

关键点：

- Provider 不写 storage。
- TUI 不调 provider。
- CLI 不绕过 core。
- SQLite 不是事件真相。
- Replay 不需要真实 API key。

## 6. AI-friendly 设计规范

Tessera 要适合 AI 长期参与开发，不只是“代码能跑”。

强制规范：

- 每个 crate 有 README，写清职责、边界、禁止事项。
- Public type 有文档注释。
- 文件保持小而专注，避免巨大 `app.rs`、`engine.rs`、`state.rs`。
- 跨 crate API 用简单 `struct`、`enum`、`trait` 表达，少用复杂宏和过度泛型。
- 先写 protocol/fixture/replay，再扩展 provider/tool/agent。
- 每个 runtime 行为都必须能通过 unit test、fixture 或 replay 验证。
- 所有模型交互必须有 trace。
- 所有 schema 都必须版本化。
- 大输出必须 artifact 化。
- checkpoint lifecycle 必须 trace-backed；当前 restore lifecycle 只能记录 `RestoreBlocked` metadata，不能 restore/revert 文件。
- 变更必须遵守 `AGENTS.md` 和 `docs/crate-boundaries.md`。

AI 修改代码时应优先处理小边界任务：

- 一个 crate。
- 一个 public type。
- 一个 adapter。
- 一个 replay fixture。
- 一个可验证行为。

避免让 AI 一次修改 UI、core、provider、storage 四层。

## 7. Agent-ready 设计

v0.1 先保证 agent 能平滑接入；v0.5 的当前实现已经提供 no-tool `AgentLoop`，用于记录 `TaskKind::AgentRun`、agent run/step lifecycle、finish summary 和 replay evidence。它不是完整 coding-agent runtime。

需要预留：

- `Task`：agent run、tool run、replay、learning job 都能成为 task。
- `Artifact`：patch、test report、tool output、agent transcript 外部化。
- `RunEvent` lifecycle：no-tool agent loop 已使用 `agent_run_started`、`agent_step_started`、`agent_step_completed` 和 `agent_run_completed`；explicit Skill Runtime v1 已使用 `skill_activated` 记录 trace-safe activation metadata；executable skill steps、handoff、swarm、learning 仍按后续版本门禁推进。
- `PolicyDecision` 类型占位：未来工具调用必须可审批。
- `ToolCallRequested` / `ToolDispatch` / `ToolResult` metadata：provider 和 agent 都不能直接执行工具；即便未来底层并发，trace 和模型可见结果也必须按声明顺序输出。
- `DiagnosticsReported` metadata：diagnostics/LSP 结果可以写 trace，但 core helper 不启动 LSP server、compiler 或 test runner。
- `McpToolAdapter` metadata：MCP tool metadata 只能转换成 Tessera descriptor/request，不连接 MCP server、不执行 tool，annotations 只作为不可信 hint。
- `ToolRepairReport` metadata：tool-call repair 只能记录 provider-neutral 摘要，不能把 provider 原始 reasoning 或 raw text 写进 trace。
- `SandboxDecision` metadata：workspace path guardrail 必须先写入 provider-neutral trace，再进入后续真实 sandbox/tool runtime。
- `OsSandboxProfile` metadata：core 可以规划 read-only / workspace-write / network-required / denied profile，但不启动 OS sandbox、不执行工具、不打开网络。
- `MutationEnforcementPlan` metadata：file-changing coding workflows 默认 worktree-first；explicit-local 必须携带独立 policy decision id；sandbox profile 必须先被选择，executor request 在 checkpoint/reviewer/policy gates 关闭前保持 blocked。
- `AgentProfile` schema foundation：模型、角色、工具权限、记忆范围、context scope 和 step limit 显式配置；core 提供只读 registry 和 no-tool `AgentLoop`，可接收显式 opt-in 且由 core planner 发现的 project instruction context 和 explicit read-only skill context，但不执行 tool、不运行 skill script、不默认读取全局/用户 instructions、不维持后台任务。
- `MemoryProposal` metadata：proposal 可以进入 UI review，但长期 memory runtime 和真实写入必须等待 scope schema、policy 和 trace 边界，避免默认全局污染。
- `Skill` manifest 兼容 `SKILL.md` frontmatter，后续再扩展 `skill.toml`；v0.5 `SkillRuntimePlanner` 支持 project-local discovery、显式 activation、只读 reference、redaction 和 trace-safe `skill_activated` metadata，但不执行脚本、工具、MCP、hooks 或 global/default skill loading。

Agent-ready 不等于立即拥有完整 coding-agent。当前 no-tool loop 只是稳定 run envelope、trace 和 replay 语义，避免未来 agent 被迫进入旁路系统。

DeepSeek-TUI 的 sub-agent 设计还暴露出一个关键点：父 agent 不应把所有子任务 transcript 塞回上下文。Tessera 后续 agent 系统必须使用 artifact/context handle 模型：

- 子 agent transcript 默认进入 artifact。
- 父 agent 只接收 structured summary、evidence 和 metrics。
- 需要细节时通过 handle slice 或 projection 读取。
- 并发数、递归深度、token 成本必须显式限制。
- handoff 必须结构化并写入 trace。
- persistent sub-agent runtime 必须由 core 的 coordinator 拥有；CLI/TUI/GUI/client/provider/storage 都不能直接调度 child session。
- transcript artifact lifecycle 只能记录 handle metadata、event ranges、labels 和 reasons；transcript body、summary generation、provider calls 和 parent-context ingestion 必须留给后续门禁。
- planned -> active -> waiting/inactive/completed 的状态变化必须可从 trace 重放；task owner attach/heartbeat/detach/lost 同样适用于 child session。
- current child task owner bridges 只能构造 attach/detach/heartbeat/lost/reattach metadata；heartbeat loop、lost-owner detection、automatic reattach、provider resume 和 provider execution 必须留给 runtime gate。
- cancellation cascade 必须显式记录并通过 core helper 校验；当前只能产生 metadata，真实 child cancellation、reattach 和 provider socket handling 必须留给后续 runtime gate。

## 8. Future Architecture Path

建议演进路线：

1. v0.1：headless runtime、CLI/TUI chat、trace、mock/replay。
2. v0.2：context workbench、read-only runtime API、task registry v1、GUI shell spike、cost/cache telemetry、model router 草案。
3. v0.3：tool descriptor、policy gate、approval UI、artifact handles、OS sandbox、workspace checkpoint。
4. v0.4：MCP adapter、HTTP/SSE runtime API shape、diagnostics/LSP metadata、memory proposal UI。
5. v0.5：no-tool single agent loop、non-interactive agent run envelope、opt-in project instruction discovery、skill runtime v1、trace-backed background ownership foundation、no-tool owner attach/detach、runtime API/app-server DTO alignment、pause/resume、context handle projection。
6. v0.6：persistent sub-agent sessions、structured handoff、reviewer gate、artifact-backed transcript isolation、core-owned sub-agent runtime ownership contract、transcript artifact lifecycle metadata。
7. v0.7：coding agent workflow、worktree-first mutation、diff/test/checkpoint/rollback、apply-patch tool、TUI/GUI diff and review surfaces。
8. v0.8：swarm scheduler 和 automation trigger integration，建立在稳定 agent/task/trace/review/cost gates 之上。
9. v0.9：learning proposal system，默认只提案、不自动应用。

## 9. Non-negotiable Invariants

- 只有一个真实 runtime 来源。
- TUI 永远不是 runtime owner。
- GUI 永远不是 runtime owner。
- Provider 永远不执行工具。
- Tool 未来永远不绕过 policy。
- Trace 从第一天就是 durable event truth。
- SQLite 可以重建。
- Secrets 不进入持久化数据。
- Agent、memory、skill、swarm 都通过 protocol/core/trace 接入。
- Project instructions、hooks、automations 和 app-server 都通过 protocol/core/trace/policy 接入。
- v0.1 先稳定，不追求功能数量。

## 10. 技术选型表

| Area | Choice | Reason |
| --- | --- | --- |
| Language | Rust | 本地质量、安全、分发、类型边界 |
| TUI | Ratatui + crossterm | Rust 生态成熟终端 UI + 可控键盘事件循环 |
| GUI | Tauri 2 + TypeScript/React/Vite 作为产品 GUI 默认方向；v0.2 已有 mock/replay shell spike；egui 只作为内部 inspector 候选；GPUI 继续观察 | 保持 Rust runtime owner，同时获得复杂工作台 UI、可访问性、截图自动化和跨平台桌面分发能力 |
| Async | Tokio | Streaming、background task、IO |
| HTTP | Reqwest | Provider adapter HTTP client |
| Serialization | Serde | Protocol、config、trace、fixture |
| Config | TOML | 本地工具配置可读性好 |
| Storage | JSONL + SQLite via `rusqlite/bundled` | 可回放事件真相 + 可查询索引 + 发布可移植性 |
| CLI | Clap | 稳定 headless 入口 |
| Error | thiserror / anyhow boundary | typed library errors + entrypoint context |
| IDs | uuid or ulid | 本地唯一、持久化简单 |
| Testing | cargo test + fixtures + golden trace | AI-friendly regression base |
| Quality Gates | fmt + clippy + test + doctor | 实现前后都有明确门禁 |

具体 crate 选择可以在 scaffold 前再次确认，但这些技术方向应作为 v0.1 默认基线。
