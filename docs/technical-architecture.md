# Tessera Technical Architecture

日期：2026-05-14

## 1. 定位

Tessera 是一个 Rust-first、AI-friendly、agent-ready 的本地终端大模型工作台。

它不是单纯的聊天 TUI，也不是某个 provider 的客户端。它的核心是一个可审计、可回放、可扩展的本地 runtime。CLI、TUI、未来 GUI、replay runner、未来 runtime API、工具系统、agent 系统、memory 和 skill 都必须共享同一套协议和状态模型。

核心目标：

- Model-agnostic：支持多个 provider，但 core 不被 provider 私有结构污染。
- Headless-first：先有可测试的 headless runtime，再有 TUI 和 GUI。
- Replayable：所有运行都能通过 JSONL trace 回放和审计。
- Auditable：未来所有工具调用必须经过 policy gate。
- AI-friendly：代码边界小、协议清晰、fixture/replay 完整，方便 AI 稳定参与开发。
- Agent-ready：v0.1 不实现 agent runtime，但协议、task、artifact、policy 和 trace 从第一天给 agent 留出接入点。

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

DeepSeek-TUI 解析稿对 Tessera 的核心启发是：runtime 能力比 UI 外观更重要。Tessera 应优先吸收它的 durable task、runtime API、tool policy、sandbox、snapshot、sub-agent handle、MCP/ACP integration 和 distribution 设计，但按阶段纳入，避免 v0.1 失控。详细采纳矩阵见 [DeepSeek-TUI Lessons](deepseek-tui-lessons.md)。

## 4. v0.1 Crate 结构

v0.1 首批只开必要 crate：

```text
crates/
  protocol/
  core/
  providers/
  storage/
  config/
  cli/
  tui/
  # future, not v0.1:
  # client/
  # gui/
```

职责摘要：

- `protocol`：公共类型、ID、runtime schema、RunEvent、EventFrame、NormalizedError。
- `core`：运行生命周期、ConversationEngine、事件路由、provider/storage 协调。
- `providers`：Provider trait、OpenAI-compatible、Ollama、Mock provider。
- `storage`：JSONL trace writer、SQLite index、repository。
- `config`：配置读取、profile、data dir、secret env var 引用。
- `cli`：headless 命令入口和本地二进制命令编排。
- `tui`：Ratatui view。
- future `client`：UI-neutral intent、projection、view model；从 TUI 抽出后供 GUI 复用。
- future `gui`：desktop/web shell，只消费 `client` + core/runtime API。

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
- 变更必须遵守 `AGENTS.md` 和 `docs/crate-boundaries.md`。

AI 修改代码时应优先处理小边界任务：

- 一个 crate。
- 一个 public type。
- 一个 adapter。
- 一个 replay fixture。
- 一个可验证行为。

避免让 AI 一次修改 UI、core、provider、storage 四层。

## 7. Agent-ready 设计

v0.1 不实现 agent runtime，但必须从第一天保证 agent 能平滑接入。

需要预留：

- `Task`：agent run、tool run、replay、learning job 都能成为 task。
- `Artifact`：patch、test report、tool output、agent transcript 外部化。
- `RunEvent` reserved events：tool、skill、memory、agent、swarm、learning 的事件名先稳定。
- `PolicyDecision` 类型占位：未来工具调用必须可审批。
- `ToolCallRequested` / `ToolResult` 事件占位：provider 和 agent 都不能直接执行工具。
- `AgentProfile` schema 占位：模型、角色、工具权限、记忆范围、step limit 显式配置。
- `MemoryScope` schema 占位：避免长期记忆默认全局污染。
- `Skill` manifest 兼容 `SKILL.md` frontmatter，后续再扩展 `skill.toml`。

Agent-ready 不等于 v0.1 做 agent。它意味着 v0.1 的 runtime 不会把未来 agent 逼进旁路系统。

DeepSeek-TUI 的 sub-agent 设计还暴露出一个关键点：父 agent 不应把所有子任务 transcript 塞回上下文。Tessera 后续 agent 系统必须使用 artifact/context handle 模型：

- 子 agent transcript 默认进入 artifact。
- 父 agent 只接收 structured summary、evidence 和 metrics。
- 需要细节时通过 handle slice 或 projection 读取。
- 并发数、递归深度、token 成本必须显式限制。
- handoff 必须结构化并写入 trace。

## 8. Future Architecture Path

建议演进路线：

1. v0.1：headless runtime、CLI/TUI chat、trace、mock/replay。
2. v0.2：context workbench、read-only runtime API、task registry v1、GUI shell spike、cost/cache telemetry、model router 草案。
3. v0.3：tool descriptor、policy gate、approval UI、artifact handles、OS sandbox、workspace checkpoint。
4. v0.4：MCP adapter、HTTP/SSE runtime API、diagnostics/LSP、memory proposal UI。
5. v0.5：single agent loop、skill runtime v1、pause/resume、context handle projection。
6. v0.6：persistent sub-agent sessions、structured handoff、reviewer gate。
7. v0.7：coding agent workflow、diff/test/checkpoint/rollback、apply-patch tool。
8. v0.8：swarm scheduler，建立在稳定 agent/task/trace 之上。
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
- v0.1 先稳定，不追求功能数量。

## 10. 技术选型表

| Area | Choice | Reason |
| --- | --- | --- |
| Language | Rust | 本地质量、安全、分发、类型边界 |
| TUI | Ratatui + crossterm | Rust 生态成熟终端 UI + 可控键盘事件循环 |
| GUI | Tauri 2 + TypeScript/React/Vite 作为产品 GUI 默认方向；egui 只作为内部 inspector 候选；GPUI 继续观察 | 保持 Rust runtime owner，同时获得复杂工作台 UI、可访问性、截图自动化和跨平台桌面分发能力 |
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
