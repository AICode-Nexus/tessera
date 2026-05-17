# Tessera Global Plan

日期：2026-05-17

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
- [x] Reasonix 官方复核吸收：cache-stable context、ordered parallel dispatch、tool-call repair telemetry、visible cost control 和 no-progress loop policy 已纳入规划。
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
- [x] v0.1 release checklist 和 tag plan 已写入 `docs/v0.1-release-checklist.md`。

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
- [x] provider error normalization：HTTP/provider body 会归一化为 provider-neutral code、retryable、safe details，并在写 trace 前脱敏。

### Core

- [x] ConversationEngine。
- [x] mock provider 可驱动完整 run loop。
- [x] core 将 provider stream 转成 EventFrame 并写 trace。
- [x] core 不依赖 provider 私有响应结构。
- [x] cancellation / timeout / backpressure 基础语义：live sink 可请求取消，provider event timeout 会写 `task_cancelled`，TUI live channel 已 bounded。
- [x] replay runner 初版。

### CLI

- [x] `tessera doctor` / `tessera doctor --json`：文本输出包含 status、data_dir、trace writable、SQLite index health、provider profile IDs，JSON 保持脚本友好。
- [x] `tessera chat --provider mock --prompt ...`。
- [x] `tessera chat --stdin`：支持从 stdin 管道读取单轮 prompt，便于脚本组合。
- [x] `tessera chat --file <path>`：支持从 UTF-8 文件读取单轮 prompt，便于脚本、文档和批处理请求组合。
- [x] `tessera chat --json`：支持单轮 chat 输出稳定 JSON，包含 `trace_id` 和 `assistant_text`，便于脚本接后续 `sessions` / `transcript`。
- [x] `tessera chat --list-commands`：无需解析 config / data_dir 即可打印交互式 slash command 清单，便于用户在启动 REPL 前发现能力。
- [x] CLI REPL startup context + `/doctor`：交互式 `tessera chat` 启动时显示 active profile、data dir、configured profiles，并可在 REPL 内用 `/doctor` 查看同一 data dir 的 runtime health。
- [x] CLI REPL local ergonomics：`/commands` 作为 `/help` 别名，`/history` 只读列出当前 visible client projection，`/clear` 清理当前可见 thread 而不删除磁盘 trace。
- [x] CLI numbered session resume：`sessions` / `/sessions` 文本输出带 1-based 编号，`/resume <number>` 和 `chat --resume <number>` 可按当前 session 排序恢复 trace。
- [x] `tessera config validate`：顶层配置自检命令，可输出文本或 `--json`，检查 provider shape、重复 profile id、data_dir resolution 和 secret env 是否存在，不打开 storage、不输出真实 secret。
- [x] `tessera profiles`：顶层 provider profile inspection 命令，可输出文本或 `--json`，只展示 secret env var 名称，不读取真实 secret。
- [x] `tessera sessions`：顶层 session discovery 命令，可输出带编号的人类可读列表或 `--json`，复用 read-only `RuntimeReader`。
- [x] `tessera transcript <trace_id>`：顶层 transcript inspect 命令，可输出 markdown 或 `--json`，复用 trace projection，不重新请求 provider。
- [x] `tessera replay <trace_id>`：顶层 replay summary 命令，可输出文本或 `--json`，复用 core `ReplayRunner`，不重新请求 provider。
- [x] `tessera events <trace_id>`：顶层 trace event inspect 命令，可输出文本或 `--json`，支持 `--since` / `--limit` 分页，复用 read-only `RuntimeReader`。
- [x] `tessera chat --provider mock` 交互式 CLI REPL：无 `--prompt` 时进入 Claude/Codex 风格命令行聊天壳，启动时显示 active profile / data dir / configured profiles，支持 `/help`、`/commands`、`/new`、`/clear`、`/profiles`、`/profile <id>`、`/sessions`、`/resume <trace_id|#>`、`/doctor`、`/history`、`/status`、`/export`、`/quit`，并复用 `tessera-client` projection 与 core live event stream。
- [x] `tessera chat --resume <trace_id>`：启动交互式 CLI 时直接投影旧 trace，不需要先进入 REPL 再手动输入 `/resume`。
- [x] `tessera chat --continue`：启动交互式 CLI 时自动投影最近更新的 trace session，下一轮 prompt 继续带 restored history。
- [x] `tessera init` 安全配置模板：生成 mock / Ollama / OpenAI-compatible 示例，只写 secret env var 名称，不写 provider secret。
- [x] CLI session list / resume：REPL 支持 `/sessions` 和 `/resume <trace_id>`，通过 `RuntimeReader` 只读 trace summary 和 trace record projection，不重新请求 provider。
- [x] CLI resumed-session continuation：`/resume <trace_id>` 后的下一轮 prompt 会把恢复出的 user/assistant transcript 作为 provider-visible chat history，同时新 trace 只记录当前用户 turn。
- [x] doctor 输出 data dir、trace writable、SQLite index health、provider profile。
- [x] config-driven provider profile routing。
- [x] CLI 使用 OpenAI-compatible provider 完成真实流式对话：OneAPI-compatible endpoint + `deepseek-v4-pro` 已完成 live smoke，trace 已检查无 secret-like 内容。
- [~] CLI 使用 Ollama provider 完成真实流式对话：代码路径已接入，缺 live smoke 验证。
- [x] secret 只从 env 读取，禁止进入 trace：已覆盖缺失 env 时请求前失败且不写 trace，并已检查真实 OpenAI-compatible 成功路径 trace 无 key / Authorization / Bearer / Cookie。

### TUI

- [x] Ratatui crate 建立。
- [x] profile / reasoning / cache / cost status-line 占位。
- [x] 最小主聊天窗口：shared client projection、line renderer、terminal frame、`tessera tui` 入口已完成。
- [x] 输入框和流式输出：键盘事件、提交、core live event sink、CLI bridge、TUI channel apply 已完成。
- [x] live event backpressure：TUI live channel 使用 bounded channel，channel full / closed 会回传取消信号。
- [x] 模型/profile 切换入口：Tab / Shift-Tab 产生 `ClientIntent::SwitchProfile`，提交 prompt 时携带当前 profile。
- [x] `/new`、`/save`、`/export` 基础入口：shared client slash-command intent、TUI local handling、markdown projection export。
- [x] usage/cache/cost live status projection：`tessera-client` 从 `UsageReported` live event 和 replay trace record 更新 UI-neutral `ClientStatus`，TUI 仍只负责渲染。
- [x] TUI 只订阅 core 事件，不直接依赖 provider SDK 或 SQLite internals。

### GUI Preparation

- [x] GUI 不另起 runtime：架构约束已写入 `docs/gui-ready-architecture.md`、`docs/technical-architecture.md` 和 `docs/crate-boundaries.md`。
- [x] GUI 技术选型 ADR：默认产品 GUI 方向为 Tauri 2 + TypeScript/React/Vite；GUI 实现仍等待 `client` 边界。
- [x] UI-neutral view model：`tessera-client` 已提供 `ClientIntent`、`ClientStatus`、`ClientProjection`、`ClientSnapshot`，TUI 已切换为复用该模型。
- [x] GUI 技术选型 spike：已新增 `apps/gui-tauri` Tauri 2 + React/Vite shell 和 `tessera-gui-bridge`，只接 mock/replay 与 read-only projection。
- [~] Live event bridge：core/CLI/TUI 已共享同一套 `EventFrame` 流；future GUI 复用契约已明确，待 GUI shell spike 验证。

### Quality Gates

- [x] `PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check`。
- [x] `PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings`。
- [x] `PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace`。
- [x] `tessera doctor --json` mock smoke。
- [x] `tessera chat --provider mock --prompt "hello"` smoke。
- [x] replay golden trace gate。
- [x] live provider smoke gate，默认跳过；OpenAI-compatible manual final smoke 已完成。

## 3. 下一步执行顺序

按稳定发展优先级，下一阶段不要先做 UI 大改，也不要直接做 tool/agent。

1. [x] Config-driven provider routing。
2. [x] OpenAI-compatible live smoke test，默认跳过。
3. [x] Ollama live smoke test，默认跳过。
4. [x] Thread / Turn / Item repository 查询 API。
5. [x] Replay fixture 和 golden trace test。
6. [x] `rusqlite/bundled` 发布可移植性配置。
7. [x] 真实 provider smoke 验证：OpenAI-compatible / OneAPI-compatible endpoint + `deepseek-v4-pro` 已验证；Ollama `localhost:11434` 仍不可达但不阻塞 v0.1.0。
8. [x] 最小 TUI chat loop：view/input/event reducer、terminal event loop、CLI `tui` 入口已完成。
9. [x] TUI profile switch 入口。
10. [x] Live event bridge：core event sink、CLI bridge、TUI live channel 已完成。
11. [x] cancellation / timeout / backpressure 基础语义。
12. [x] GUI 技术架构和选型 ADR：先锁定 Tauri-first 产品 GUI 路线和 AI-ready IPC/权限边界，不引入 v0.1 GUI 依赖。
13. [x] GUI-ready client model 边界：已抽出 `tessera-client`，TUI 的 intent、message projection 和 status projection 复用 UI-neutral API。
14. [x] `/new`、`/save`、`/export` 基础入口。
15. [x] v0.1 release checklist 和 tag 计划。
16. [x] `v0.1.0-alpha.1` pre-tag gate：release notes section、本地门禁、mock smoke、clean tree、CI 均已确认，下一步可打 alpha tag。
17. [x] `v0.1.0` final gate：OpenAI-compatible live smoke 已完成，并检查真实 provider 成功路径 trace 无 secret-like 内容；下一步可打 final tag。
18. [x] 纯 CLI REPL 初版：`tessera chat` 缺省进入交互壳，保留 `--prompt` 单轮模式；命令解析和 session projection 有 contract tests。
19. [x] CLI runtime v2：`tessera init`、`/sessions`、`/resume <trace_id>` 已完成；history/resume 基于 read-only runtime reader，不绕过 core。
20. [x] CLI resumed-session continuation：provider-neutral `ProviderMessage` / `ConversationRequest.history` 已接入 provider/core/CLI；恢复 session 后下一次 prompt 会携带恢复出的 user/assistant 历史。
21. [x] CLI startup resume：`tessera chat --resume <trace_id>` 已接入 interactive chat 启动路径，contract test 覆盖启动恢复后继续对话。
22. [x] CLI top-level sessions：`tessera sessions [--json]` 已接入 config/data-dir resolution，便于脚本和用户在 REPL 外发现可恢复 trace。
23. [x] CLI top-level transcript：`tessera transcript <trace_id> [--json]` 已接入 trace projection，便于在 REPL 外查看会话内容后再 resume。
24. [x] CLI stdin prompt：`tessera chat --stdin` 已接入 one-shot chat path，可从管道读取 prompt 并继续通过 core/provider/storage 完整链路执行。
25. [x] CLI file prompt：`tessera chat --file <path>` 已接入 one-shot chat path，可从 UTF-8 文件读取 prompt 并继续通过 core/provider/storage 完整链路执行。
26. [x] CLI JSON chat output：`tessera chat --json` 已接入 one-shot chat path，输出稳定 `trace_id` / `assistant_text` JSON，并拒绝无 prompt source 的交互模式。
27. [x] CLI continue latest session：`tessera chat --continue` 已复用 read-only session list 找到最近 trace，并进入与 `--resume` 相同的交互恢复路径。
28. [x] CLI replay command：`tessera replay <trace_id> [--json]` 已接入 core `ReplayRunner`，可离线重建 assistant text 和 event kind summary。
29. [x] CLI events command：`tessera events <trace_id> [--json] [--since <seq>] [--limit <n>]` 已接入 `RuntimeReader::list_events`，可分页检查 trace event records。
30. [x] CLI profiles command：`tessera profiles [--json]` 已接入 config resolution，可只读列出 provider id/kind/default_model/base_url/api_key_env 名称。
31. [x] CLI config validate command：`tessera config validate [--json]` 已接入 config/data-dir resolution，可在 chat/tui 前只读检查 provider config 和 secret env presence。
32. [x] CLI doctor text details：`tessera doctor` 已输出 data_dir、trace_writable、sqlite_index_healthy 和 provider_profiles，和 `--json` 共享同一个 `DoctorReport`。
33. [x] CLI chat command discovery：`tessera chat --list-commands` 复用 REPL `/help` formatter，并在 config/data-dir resolution 前直接返回。
34. [x] CLI REPL startup context and doctor command：交互式 `tessera chat` 启动时显示 active profile、data_dir、available_profiles，并支持 `/doctor` 复用顶层 doctor runtime health reporter。
35. [x] CLI REPL local ergonomics：`/commands`、`/history`、`/clear` 已接入本地 REPL command path；只读取或重置 `ClientSnapshot`，不调用 provider、不修改 trace。
36. [x] CLI numbered session resume：`format_session_lines` 已输出 1-based 编号，`/resume <number>` / `chat --resume <number>` 会按当前 read-only session list 映射到 trace_id 后再投影恢复。

## 4. v0.2 Checklist

- [x] Context workbench 初版：`tessera-protocol` 定义 `ContextReference` / `ContextSource` / `ContextPlacement` / `ContextBudget`，`tessera-core` 提供纯内存 `ContextWorkbench` add/remove/list/budget summary；先区分 stable prefix、append-only transcript 和 volatile scratch，不读取文件内容、不构建 prompt。
- [x] Read-only runtime API 初版：`tessera-core` 提供 `RuntimeReader`，支持按 `trace_id` / `since_seq` / `limit` 读取 trace event page，并通过 core 查询 thread / turn / item / task / artifact ID 索引；HTTP/SSE 服务仍留到 v0.4。
- [x] Task registry v1：`RuntimeReader::list_tasks` 可从 trace 重建 read-only task summaries，`tessera-client` 可从 live events / replay trace records 投影 `ClientTask` 和 task status summary，TUI 仍只渲染。
- [x] Tauri GUI shell spike：`apps/gui-tauri` 提供 Tauri 2 + React/Vite shell，`tessera-gui-bridge` 提供 typed mock/replay commands、bounded GUI event buffer、submit/cancel/replay 投影；不依赖 provider SDK、storage internals 或 TUI。
- [x] Rust-to-TypeScript DTO 生成策略：`tessera-gui-bindings` 使用 `ts-rs` 从 `protocol` / `client` / `gui-bridge` 的 GUI DTO 生成 `apps/gui-tauri/src/generated/bindings.ts`，并用 contract test 校验 checked-in 文件与 Rust 生成一致；只生成 DTO，不生成 runtime。
- [x] GUI automation smoke check：`apps/gui-tauri/src/App.smoke.test.tsx` 用 Vitest + Testing Library 覆盖 mock/replay load、submit、cancel、new-thread 和 icon action accessible names；真实 Playwright/browser screenshot gate 仍留作环境稳定后的后续检查。
- [x] Usage/cache/cost/context telemetry summary：`tessera-client` 已从标准 live events 和 replay trace records 聚合 provider-neutral summary，TUI 只负责渲染。
- [x] Model router 草案：`tessera-core` 已提供 draft `ModelRouter`，只记录 manual/default `RouteDecision` 和 `decision_reason`，不默认自动路由。
- [x] No-progress loop detection 草案：`tessera-protocol` 定义 `NoProgressLoop` / `no_progress_loop_detected`，`tessera-core` 提供 draft `NoProgressDetector`，连续只读/重复 repair/无输出循环先 stop / ask / summarize，不直接升档到更贵模型。
- [x] Artifact handle projection：`RuntimeReader::list_artifacts` 可从 `artifact_created` 和 `artifact_refs` 重建 read-only artifact summaries，`tessera-client` 可投影 `ClientArtifact` 和 artifact status summary，TUI 仍只渲染。
- [x] Skill registry schema：`tessera-protocol` 定义 `SkillManifest` / `SkillSource` / `SkillEntrypoint` / requirements / policy，`tessera-core` 提供只读 `SkillRegistry` list/find；兼容 `SKILL.md` metadata，不执行 skill runtime。
- [x] Snapshot/checkpoint schema 设计：`tessera-protocol` 定义 `SnapshotId`、`WorkspaceCheckpoint`、`SnapshotKind` 和 `snapshot_created` event，`RuntimeReader::list_snapshots` 可只读投影 checkpoint metadata；不实现 restore/revert。
- [x] 分发计划：`docs/distribution-plan.md` 已定义 Cargo、GitHub Releases、Homebrew、npm wrapper、Docker 的 channel ownership、release assets、checksum、Cargo 发布顺序、镜像变量和 v0.3+ acceptance checklist；v0.2 不实际发布渠道。

## 5. v0.3-v0.4 Checklist

- [x] Release identity metadata：`tessera --version` 输出 crate version 和构建 git SHA，作为 GitHub Releases、Homebrew、npm wrapper 和 Docker 的共同前置。
- [x] Tool descriptor：`tessera-protocol` 定义 `ToolDescriptor` / `ToolId` / `ToolPermission` / `ToolSideEffect`，`tessera-core` 提供只读 `ToolRegistry` list/find；不执行工具。
- [x] Tool descriptor `parallel_safe` 字段：默认 false，第三方/MCP tool 必须显式 opt in。
- [x] Policy gate：`tessera-protocol` 定义 `ToolCallRequest` / `ToolPolicyDecision` / `ToolApproval` 和 tool policy trace events，`tessera-core` 提供 draft `PolicyGate`，只输出 allow / ask_user / deny metadata，不执行工具。
- [x] Tool dispatcher ordered result append：`tessera-protocol` 定义 `ToolDispatch` / `ToolResult` 和 `tool_dispatch_started` / `tool_dispatch_completed` / `tool_result` events，`tessera-core` 提供 `OrderedToolResultBuffer`，允许底层并发完成但只按声明顺序释放 trace/model-visible result；不执行工具。
- [x] Tool-call repair telemetry：`tessera-protocol` 定义 `ToolRepairReport` / `ToolRepairKind` 和 `tool_repair_reported` event，`tessera-core` 提供 `ToolRepairTelemetry` helper，记录 flatten/scavenge/truncation/storm 等 provider-neutral 修复摘要；不把 provider 原始 reasoning/raw text 写入 trace。
- [x] Approval UI：`tessera-client` 投影 pending/resolved tool approvals，提供 `/approve` / `/deny` UI-neutral intents 和 approval summary，TUI status-line 可展示 pending approval；GUI bridge 接受 typed approval intents 但不执行工具。
- [x] Workspace guardrail / sandbox decision schema：`tessera-protocol` 定义 `WorkspaceScope` / `WorkspaceGuardrail` / `SandboxDecision` 和 `sandbox_decision_recorded` event，`tessera-core` 提供 draft `WorkspaceGuardrailChecker`，只做词法路径归一和 metadata 判定；不执行工具、不读写文件、不提供 OS sandbox。
- [x] OS sandbox profile foundation：`tessera-protocol` 定义 `OsSandboxProfile` / `OsSandboxMode` / filesystem/network/shell metadata 和 `os_sandbox_profile_selected` event，`tessera-core` 提供 `OsSandboxPlanner`，只根据 tool descriptor 选择 read-only / workspace-write / network-required / denied profile；不启动 OS sandbox、不执行工具、不打开网络。
- [x] Workspace checkpoint foundation：`tessera-core` 提供 `WorkspaceCheckpointPlanner`，只在 sandbox profile 标记 `requires_checkpoint` 时生成 `WorkspaceCheckpoint` metadata 和 `tessera://snapshots/<id>` URI；不创建 side-git、不读写文件、不 restore/revert。
- [x] MCP adapter foundation：`tessera-core` 提供 `McpToolAdapter` / `McpToolSpec` / `McpToolAnnotations`，把 MCP tool metadata 和 call arguments 转成 Tessera `ToolDescriptor` / `ToolCallRequest`；MCP annotations 只作为不可信 hint，`parallel_safe` 默认 false，不连接 MCP server、不执行 tool。
- [x] HTTP/SSE runtime API foundation：`tessera-core` 提供 `RuntimeHttpApi` / `RuntimeHttpEventRequest` / `RuntimeSseFrame`，复用 `RuntimeReader` 输出只读 event page JSON 和 SSE frame 编码；不启动 HTTP server、不监听端口、不拥有第二套 runtime。
- [x] Diagnostics / LSP event foundation：`tessera-protocol` 定义 `DiagnosticReport` / `Diagnostic` / `DiagnosticRange` / `DiagnosticSeverity` 和 `diagnostics_reported` event，`tessera-core` 提供 `DiagnosticsReporter` helper；不启动 LSP server、不运行 compiler、不读取文件。
- [x] Memory proposal UI foundation：`tessera-protocol` 定义 `MemoryProposal` / `MemoryProposalStatus` 和 memory proposal events，`tessera-client` 投影 pending/applied/rejected proposals，提供 `/remember` / `/forget` UI-neutral intents，TUI status-line 展示 pending memory proposal；GUI bridge 接受 typed memory intents 但不写入长期 memory runtime。

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
- [ ] 没有 no-progress loop detection 和用户可见 route/escalation reason 前，不上线自动升档。
- [ ] 没有 structured handoff 和 reviewer gate 前，不上线 swarm。
- [ ] 没有 scope schema 前，不上线长期 memory runtime。

## 8. 每次阶段推进必须更新

- [ ] 更新本文件对应 checklist。
- [ ] 更新 `CHANGELOG.md` 的 `Unreleased`。
- [ ] 如果改变 v0.1 范围，更新 `docs/v0.1-plan.md`。
- [ ] 如果改变 crate 边界，更新 `docs/crate-boundaries.md`。
- [ ] 如果改变 trace/protocol，更新 `docs/protocol-v0.md` 和 `docs/trace-schema-v0.md`。
- [ ] 跑完质量门禁后再提交。
