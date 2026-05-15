# Tessera Global Plan

日期：2026-05-14

本文是 Tessera 的全局推进清单。它不替代 `docs/v0.1-plan.md` 的阶段细节，也不替代架构文档；它用于回答三个问题：

- 当前已经完成了什么。
- 下一步应该做什么。
- 哪些能力必须等前置门禁满足后才能做。

状态标记：

- `[x]` 已完成并已验证。
- `[ ]` 未开始或未达到验收标准。
- `[~]` 已有骨架，但还不是完整用户可用能力。

## 1. 当前基线

- [x] 架构文档冻结：`technical-architecture`、`v0.1-plan`、`protocol-v0`、`trace-schema-v0`、`crate-boundaries`。
- [x] DeepSeek-TUI 解析吸收：provider capability、reasoning delta、cache/cost telemetry、route decision、artifact handle 已纳入规划。
- [x] AI 友好边界写入 `AGENTS.md`。
- [x] Rust workspace 建立：`protocol`、`client`、`core`、`providers`、`storage`、`config`、`cli`、`tui`。
- [x] 每个 crate 都有 README，说明职责和禁止事项。
- [x] CI 建立：fmt、clippy、test。
- [x] `CHANGELOG.md` 建立，阶段性变化已记录。
- [x] 已提交并 push 到 `origin/main`：`43918fe feat: scaffold v0.1 runtime`。
- [x] SQLite 通过 `rusqlite/bundled` 集成，降低本地发布时对系统 `libsqlite3` 的依赖。
- [x] GUI-ready 方向写入架构：未来 GUI 必须复用 headless runtime、client intent 和 UI-neutral view model。
- [x] GUI 技术架构和选型写入 ADR：产品 GUI 默认 Tauri 2 + TypeScript/React/Vite，egui 仅作为内部 inspector 候选，GPUI 继续观察。
- [x] GUI-ready client model 边界已落地：`tessera-client` 承载 `ClientIntent`、`ClientStatus`、`ClientProjection` 和 `ClientSnapshot`。

## 2. v0.1 Runtime Checklist

### Protocol

- [x] 强类型 ID：Thread、Turn、Item、Task、Artifact、Event、Provider、ModelProfile、Window、RouteDecision。
- [x] Runtime object schema：Thread、Turn、Item、Task、Artifact。
- [x] EventFrame 和 TraceRecord。
- [x] RunEvent：assistant delta、reasoning delta、usage、provider capability、route decision、task lifecycle。
- [x] CostEstimate、RouteDecision、ProviderCapability。
- [x] Reserved event 命名规划进入文档。
- [x] 为 trace replay 补充 fixture 兼容性测试。

### Storage

- [x] JSONL trace writer。
- [x] SQLite event index。
- [x] trace_id + seq 可关联 JSONL 和 SQLite。
- [x] storage 单元测试。
- [x] Thread / Turn / Item / Task / Artifact repository 查询 API。
- [x] index rebuild 初版。
- [x] `rusqlite` bundled SQLite 构建配置。

### Providers

- [x] Provider trait。
- [x] Mock provider。
- [x] OpenAI-compatible adapter 骨架。
- [x] Ollama adapter 骨架。
- [x] OpenAI-compatible SSE 解析测试。
- [x] Ollama JSONL 解析测试。
- [x] Provider Debug 不泄露 API key。
- [x] HTTP stream 按字节缓冲到换行再解码，避免多字节字符跨 chunk 失败。
- [x] OpenAI-compatible live smoke test，默认跳过，只在环境变量存在时运行。
- [x] Ollama live smoke test，默认跳过，只在本地服务可用时运行。
- [ ] provider error normalization 进一步细化错误码。

### Core

- [x] ConversationEngine。
- [x] mock provider 可驱动完整 run loop。
- [x] core 将 provider stream 转成 EventFrame 并写 trace。
- [x] core 不依赖 provider 私有响应结构。
- [x] cancellation / timeout / backpressure 基础语义：live sink 可请求取消，provider event timeout 会写 `task_cancelled`，TUI live channel 已 bounded。
- [x] replay runner 初版。

### CLI

- [x] `tessera doctor --json`。
- [x] `tessera chat --provider mock --prompt ...`。
- [x] doctor 输出 data dir、trace writable、SQLite index health、provider profile。
- [x] config-driven provider profile routing。
- [~] CLI 使用 OpenAI-compatible provider 完成真实流式对话：代码路径已接入，缺 live smoke 验证。
- [~] CLI 使用 Ollama provider 完成真实流式对话：代码路径已接入，缺 live smoke 验证。
- [~] secret 只从 env 读取，禁止进入 trace 的集成测试：已覆盖缺失 env 时请求前失败且不写 trace，仍缺真实 provider 成功路径脱敏验证。

### TUI

- [x] Ratatui crate 建立。
- [x] profile / reasoning / cache / cost status-line 占位。
- [x] 最小主聊天窗口：shared client projection、line renderer、terminal frame、`tessera tui` 入口已完成。
- [x] 输入框和流式输出：键盘事件、提交、core live event sink、CLI bridge、TUI channel apply 已完成。
- [x] live event backpressure：TUI live channel 使用 bounded channel，channel full / closed 会回传取消信号。
- [x] 模型/profile 切换入口：Tab / Shift-Tab 产生 `ClientIntent::SwitchProfile`，提交 prompt 时携带当前 profile。
- [ ] `/new`、`/save`、`/export`。
- [x] TUI 只订阅 core 事件，不直接依赖 provider SDK 或 SQLite internals。

### GUI Preparation

- [x] GUI 不另起 runtime：架构约束已写入 `docs/gui-ready-architecture.md`、`docs/technical-architecture.md` 和 `docs/crate-boundaries.md`。
- [x] GUI 技术选型 ADR：默认产品 GUI 方向为 Tauri 2 + TypeScript/React/Vite；GUI 实现仍等待 `client` 边界。
- [x] UI-neutral view model：`tessera-client` 已提供 `ClientIntent`、`ClientStatus`、`ClientProjection`、`ClientSnapshot`，TUI 已切换为复用该模型。
- [~] GUI 技术选型 spike：架构决策已完成；Tauri mock/replay 小样验证仍待 v0.2。
- [~] Live event bridge：core/CLI/TUI 已共享同一套 `EventFrame` 流；future GUI 复用契约已明确，待 GUI shell spike 验证。

### Quality Gates

- [x] `PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check`。
- [x] `PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings`。
- [x] `PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace`。
- [x] `tessera doctor --json` mock smoke。
- [x] `tessera chat --provider mock --prompt "hello"` smoke。
- [x] replay golden trace gate。
- [x] live provider smoke gate，默认跳过。

## 3. 下一步执行顺序

按稳定发展优先级，下一阶段不要先做 UI 大改，也不要直接做 tool/agent。

1. [x] Config-driven provider routing。
2. [x] OpenAI-compatible live smoke test，默认跳过。
3. [x] Ollama live smoke test，默认跳过。
4. [x] Thread / Turn / Item repository 查询 API。
5. [x] Replay fixture 和 golden trace test。
6. [x] `rusqlite/bundled` 发布可移植性配置。
7. [ ] 真实 provider smoke 验证：当前环境 OpenAI-compatible env 未设置，Ollama `localhost:11434` 不可达。
8. [x] 最小 TUI chat loop：view/input/event reducer、terminal event loop、CLI `tui` 入口已完成。
9. [x] TUI profile switch 入口。
10. [x] Live event bridge：core event sink、CLI bridge、TUI live channel 已完成。
11. [x] cancellation / timeout / backpressure 基础语义。
12. [x] GUI 技术架构和选型 ADR：先锁定 Tauri-first 产品 GUI 路线和 AI-ready IPC/权限边界，不引入 v0.1 GUI 依赖。
13. [x] GUI-ready client model 边界：已抽出 `tessera-client`，TUI 的 intent、message projection 和 status projection 复用 UI-neutral API。
14. [ ] `/new`、`/save`、`/export` 基础入口。
15. [ ] v0.1 release checklist 和 tag 计划。

## 4. v0.2 Checklist

- [ ] Context workbench 初版。
- [ ] Read-only runtime API。
- [ ] Task registry v1。
- [ ] Tauri GUI shell spike：只接 mock/replay 或 read-only runtime，不引入第二套 provider 或 storage 访问路径。
- [ ] Rust-to-TypeScript DTO 生成策略。
- [ ] Usage/cache/cost telemetry summary。
- [ ] Model router 草案，只记录 route decision，不默认自动路由。
- [ ] Artifact handle projection。
- [ ] Skill registry schema，只读，不执行 skill runtime。
- [ ] Snapshot/checkpoint schema 设计。
- [ ] 分发计划：Cargo、GitHub Releases、Homebrew、npm wrapper、Docker。

## 5. v0.3-v0.4 Checklist

- [ ] Tool descriptor。
- [ ] Policy gate。
- [ ] Approval UI。
- [ ] Workspace guardrail。
- [ ] OS sandbox。
- [ ] Workspace checkpoint。
- [ ] MCP adapter，将 MCP tool 转成 Tessera ToolCall。
- [ ] HTTP/SSE runtime API。
- [ ] Diagnostics / LSP event。
- [ ] Memory proposal UI。

## 6. v0.5+ Checklist

- [ ] Single agent loop。
- [ ] Skill runtime v1。
- [ ] Pause / resume。
- [ ] Context handle projection。
- [ ] Persistent sub-agent sessions。
- [ ] Structured handoff。
- [ ] Reviewer gate。
- [ ] Coding agent diff / test / checkpoint / rollback。
- [ ] Apply-patch tool。
- [ ] Swarm scheduler。
- [ ] Learning proposal system，默认只提案、不自动应用。

## 7. 强制顺序约束

- [ ] 没有 replay fixture 前，不扩大 provider 行为面。
- [ ] 没有 sandbox 和 checkpoint 前，不上线文件修改工具。
- [ ] 没有 policy gate 前，不上线 shell / file / git tool。
- [ ] 没有 usage/cache/cost telemetry 前，不上线 Auto router。
- [ ] 没有 structured handoff 和 reviewer gate 前，不上线 swarm。
- [ ] 没有 scope schema 前，不上线长期 memory runtime。

## 8. 每次阶段推进必须更新

- [ ] 更新本文件对应 checklist。
- [ ] 更新 `CHANGELOG.md` 的 `Unreleased`。
- [ ] 如果改变 v0.1 范围，更新 `docs/v0.1-plan.md`。
- [ ] 如果改变 crate 边界，更新 `docs/crate-boundaries.md`。
- [ ] 如果改变 trace/protocol，更新 `docs/protocol-v0.md` 和 `docs/trace-schema-v0.md`。
- [ ] 跑完质量门禁后再提交。
