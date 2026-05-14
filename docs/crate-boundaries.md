# Tessera Crate Boundaries

日期：2026-05-14

## 1. 目的

本文定义 Tessera v0.1 的 crate 边界和依赖方向。它的作用是防止项目从第一版开始就把 runtime、UI、provider、storage 和未来 agent 能力混在一起。

这是实现阶段的约束文档，不是长期愿景清单。

## 2. 依赖方向

v0.1 允许的核心依赖方向：

```text
cli  -> core -> providers
cli  -> core -> storage
cli  -> config
cli  -> tui      # only for local binary command dispatch

tui  -> core
tui  -> config

future gui -> client/core/runtime_api
future gui -> config
future client -> protocol

core -> protocol
core -> storage
core -> providers

providers -> protocol
storage   -> protocol
config    -> protocol
```

禁止的依赖方向：

```text
protocol -> anything in this workspace
providers -> core
providers -> storage
providers -> tui
storage -> core
storage -> providers
storage -> tui
tui -> providers
tui -> storage internals
cli -> providers internals
gui -> providers
gui -> storage internals
gui -> tui
gui -> cli internals
```

CLI、TUI 和未来 GUI 都只能通过 core 使用 provider 和 storage。`cli -> tui` 只允许作为本地二进制的命令入口编排，不允许把 TUI 状态变成 CLI 或 core 的运行时状态。这样才能保证只有一个真实 runtime 来源。

## 3. Crate 职责

### protocol

职责：

- Public runtime schema。
- ID 类型。
- Thread / Turn / Item / Task / Artifact。
- EventFrame。
- RunEvent。
- NormalizedError。
- provider-neutral extension metadata 类型。

允许依赖：

- serde。
- time 或 chrono。
- uuid 或 ulid。
- thiserror 可选。

禁止：

- HTTP client。
- Tokio runtime 绑定。
- Ratatui。
- SQLite。
- provider SDK。
- 文件系统读写。
- 环境变量读取。

### core

职责：

- ConversationEngine。
- Run lifecycle。
- Event routing。
- Provider trait 使用。
- Trace 写入协调。
- Task/Window 的最小运行时语义。
- v0.1 reserved type 的行为边界。

允许依赖：

- protocol。
- providers trait surface。
- storage public repository API。
- config public API。
- tokio。
- futures/streams。

禁止：

- Ratatui widget。
- provider 私有响应结构。
- API key 明文处理。
- shell/file/git/http tool 执行。
- MCP runtime。
- agent loop 实现。

### providers

职责：

- Provider trait。
- OpenAI-compatible adapter。
- Ollama adapter。
- Mock provider。
- Provider capability discovery。
- Reasoning delta conversion。
- Cache/cost/latency telemetry normalization。
- Provider error normalization。
- Provider stream 到 RunEvent 的转换。

允许依赖：

- protocol。
- reqwest。
- serde。
- async-trait 或等价方案。
- tokio stream utilities。

禁止：

- storage 写入。
- TUI/CLI 输出。
- tool 执行。
- policy decision。
- memory recall。
- agent planning。

Provider 只能输出标准事件和安全 extension metadata。

### storage

职责：

- JSONL trace writer。
- SQLite index。
- Thread/Turn/Item/Task/Artifact repository。
- schema migrations。
- index rebuild。

允许依赖：

- protocol。
- sqlx 或 rusqlite。
- serde_json。
- filesystem primitives。

禁止：

- provider SDK。
- TUI/CLI 渲染。
- 模型请求。
- policy 判断。
- secret 原文持久化。

如果 JSONL 和 SQLite 状态冲突，JSONL 是事件真相，SQLite 是可重建索引。

### config

职责：

- `config.toml` 读取。
- provider profile。
- model profile。
- data dir。
- secret env var 名称。
- UI 偏好占位。
- future guardrails schema 占位。

允许依赖：

- protocol。
- toml。
- serde。
- directories。

禁止：

- 保存 API key 明文。
- 调 provider。
- 写 trace。
- 读取 TUI 状态。

v0.1 secret 只解析环境变量引用，不做完整 keychain。

### cli

职责：

- `tessera chat`。
- `tessera doctor --json`。
- 后续 replay command。
- 面向脚本和自动化的稳定入口。

允许依赖：

- core。
- config。
- protocol。
- clap。
- serde_json。

禁止：

- 直接调用 provider adapter internals。
- 直接写 storage internals。
- 绕过 core 构造运行状态。

### tui

职责：

- Ratatui 渲染。
- 键盘事件。
- 输入框状态。
- 简单窗口/焦点状态。
- 展示 core 事件。

允许依赖：

- core public API。
- config public API。
- protocol public types。
- ratatui。
- crossterm。

禁止：

- 调 provider SDK。
- 执行 shell/file/git/http tool。
- 写 provider request。
- 直接读写 SQLite internals。
- 持有真实 runtime 状态机。

TUI 是 view，不是 runtime。

### future client

职责：

- UI-neutral intent。
- status / message / task projection。
- trace record 到 view model 的纯函数转换。
- keymap、command palette 和 GUI action 的共享 command schema。

允许依赖：

- protocol。
- serde。

禁止：

- Ratatui widget。
- GUI toolkit widget。
- provider SDK。
- storage internals。
- 真实 runtime 状态机。

`client` 只有在 TUI 和 GUI 都需要复用同一套状态投影时才独立成 crate。

### future gui

职责：

- 桌面或 Web shell。
- 布局、菜单、快捷键、可访问性和渲染。
- 展示 core/runtime API 事件和 trace projection。

允许依赖：

- future `client`。
- protocol。
- config。
- core public API 或 future runtime_api client。
- 选定 GUI toolkit。

禁止：

- 调 provider SDK。
- 直接读写 SQLite internals。
- 直接执行 shell/file/git/http tool。
- 依赖 TUI crate。
- 持有真实 runtime 状态机。

GUI 是 client shell，不是第二套 runtime。

## 4. 暂不独立成 Crate 的能力

以下能力 v0.1 只保留类型或配置占位：

- tools。
- policy。
- tasks。
- windows。
- agents。
- memory。
- skills。
- swarm。
- learning。
- runtime_api。
- gui。
- client。
- diagnostics。
- snapshots。
- sandbox。
- distribution。

它们不得在 v0.1 中形成独立执行系统。如果确实需要类型，放入 `protocol` 或 `core` 的 reserved area，并写清楚不执行。

## 5. DeepSeek-TUI Lessons 对边界的补充

DeepSeek-TUI 的解析暴露出几个必须提前固化的边界。

### model_router

未来 Auto model routing 不应放在 TUI，也不应放进 provider adapter。

推荐边界：

- `model_router` 可以先作为 `core` 内部模块。
- 输入是用户请求摘要、最近上下文摘要、provider capability、cost policy。
- 输出是 `RouteDecision`。
- `RouteDecision` 必须写 trace。
- provider 只执行已选定的真实 model/profile。

### tools / policy / sandbox

工具、审批和沙箱必须同阶段设计。

推荐边界：

- `tools` 提供 ToolDescriptor 和 ToolResult。
- `policy` 产生 Allow / Deny / AskUser。
- `sandbox` 执行 OS/workspace path guardrail。
- `core` 编排 tool request，但不直接实现具体工具细节。
- `tui` 只展示审批和结果。

禁止：

- 先上线 shell tool，后补 policy。
- 先上线 file write，再补 checkpoint。
- YOLO/trusted workspace 绕过 trace。

### diagnostics

LSP diagnostics 是强质量信号，但不属于 TUI。

推荐边界：

- `diagnostics` 后续独立成 crate。
- 输入是 workspace root、changed files、tool/editor result。
- 输出是结构化 diagnostics event。
- diagnostics 必须能写入 trace，并可在 replay 中作为 fixture。

### snapshots

快照和回滚必须独立于用户项目 `.git`。

推荐边界：

- `snapshots` 后续独立成 crate。
- 支持 side-git 或等价 checkpoint。
- 每次文件修改前后都能关联 task/turn。
- restore/revert 必须写 trace。

### runtime_api

HTTP/SSE 和未来 ACP/editor integration 都不能拥有第二套 runtime。

推荐边界：

- `runtime_api` 只暴露 core 的 thread/task/event 查询和控制。
- event stream 使用 EventFrame。
- `since_seq` 是增量读取基础。
- 默认只绑定 localhost。

### GUI client

GUI 和 TUI 必须共享同一套 client projection。

推荐边界：

- `client` 先从当前 TUI view-state reducer 中抽出 UI-neutral 部分。
- `tui` 只保留 terminal 输入和 Ratatui widgets。
- `gui` 只保留 desktop/web shell 和 toolkit widgets。
- live event bridge 由 core/runtime API 提供，TUI/GUI shell 负责订阅，`client` 只做事件到 view model 的纯函数投影。
- 完成 client model 前，GUI spike 只能读 mock/replay 数据。

### distribution

分发不是 v0.1 实现项，但架构上要避免只适合源码运行。

推荐边界：

- 后续支持 Cargo、GitHub Releases、Homebrew、npm wrapper、Docker。
- npm wrapper 只下载二进制，runtime 不依赖 Node。
- `doctor --json` 检查安装、配置、data dir、provider、trace、SQLite、sandbox 能力。

## 6. 未来拆分门槛

一个模块独立成 crate 前，必须满足：

- 有清晰 public API。
- 有 README。
- 有单元测试。
- 有至少一个 replay 或 fixture 场景。
- 不依赖 TUI。
- 不绕过 trace。
- 不绕过 policy 边界。
- 不引入循环依赖。

## 7. AI 开发约束

AI 或人类开发者修改代码时必须遵守：

- 不把核心逻辑写进 `tui`。
- 不把核心逻辑写进未来 `gui`。
- 不把 provider 私有结构传出 `providers`。
- 不在 `providers` 中执行工具。
- 不在 `storage` 中调用模型。
- 不把 API key 写进 session、trace、SQLite 或日志。
- 不新增大而全的 `app.rs`、`engine.rs`、`state.rs`。
- 每个新增 crate 必须有 README。
- 每个跨 crate public type 必须有文档注释。
- 每个运行时行为必须能被测试或 replay。

## 8. 典型错误

### 错误：TUI 直接调用 OpenAI

后果：CLI、TUI、runtime API 行为分裂。

正确做法：TUI 发送用户输入给 core，core 调 provider，provider 输出事件。

### 错误：GUI 重新实现一套聊天 runtime

后果：CLI、TUI、GUI、runtime API 的行为和 trace 不一致。

正确做法：GUI 只消费 shared client projection 和 core/runtime API 事件。

### 错误：Provider 直接写 Trace

后果：provider adapter 变成 runtime，mock/replay 难以统一。

正确做法：provider 输出 RunEvent，core 把事件包成 EventFrame 并交给 storage。

### 错误：Storage 保存 Provider 原始响应

后果：secret 和 provider 私有结构污染 trace。

正确做法：provider 先转换成标准事件，只把安全 metadata 放进 extension 或 artifact。

### 错误：提前做 Swarm Crate

后果：并发、成本、trace、handoff 都会在基础 task/agent 未稳定前失控。

正确做法：先实现 single run、trace、replay、task lifecycle，再做 agent，再做 swarm。

## 9. v0.1 通过标准

实现阶段通过 `cargo metadata` 或代码检查时，应能确认：

- `protocol` 没有 workspace 内部依赖。
- `providers` 不依赖 `core`、`storage`、`tui`。
- `tui` 不依赖 provider SDK。
- future `gui` 不依赖 provider SDK、storage internals 或 TUI。
- `cli` 和 `tui` 共享 core runtime。
- CLI、TUI、future GUI 共享同一套 client/runtime 语义。
- storage 可以从 JSONL 重建 SQLite 索引。
