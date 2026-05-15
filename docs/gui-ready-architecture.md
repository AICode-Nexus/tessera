# GUI-Ready Architecture

日期：2026-05-15

本文用于提前约束 Tessera 的 GUI 方向。GUI 不进入 v0.1 交付范围，但从现在开始不能让 CLI/TUI 的实现方式阻断后续 GUI。

当前决策见 [ADR-001: GUI Architecture and Toolkit Direction](adr/ADR-001-gui-architecture-and-toolkit.md)。

## 1. 结论

- 产品级 GUI 默认方向：Tauri 2 + TypeScript/React/Vite shell。
- Runtime 仍然 Rust-first：provider、storage、trace、policy、task lifecycle 都在 Rust core / future runtime API 内。
- `egui` 只作为 Rust-first 内部诊断面板或轻量 inspector 候选，不作为产品 GUI 默认方向。
- `GPUI` 继续观察，不进入 v0.1/v0.2 默认路径。
- v0.1 不引入 GUI 依赖；UI-neutral `client` 边界已先行抽出。

这不是立即开始 GUI 实现。它是为了让后续 `client`、TUI、CLI bridge、trace projection 的实现不走偏。

## 2. 原则

- GUI 不是第二套 runtime。
- GUI 不直接调用 provider。
- GUI 不直接读写 SQLite internals。
- GUI 不执行工具。
- GUI 不持有真实 task/session 状态机。
- GUI、TUI、CLI 必须共享同一套 core、protocol、trace 和 client projection。
- GUI 第一版必须能用 mock/replay 数据启动，不依赖真实 provider。
- 所有 GUI IPC 必须 typed、versioned、fixture-backed，不能靠散落的字符串事件约定。

## 3. 目标架构

```text
apps/gui-tauri web shell
  |
  | typed commands / bounded event channel
  v
apps/gui-tauri src-tauri bridge
  |
  v
crates/client
  |
  +--> UI-neutral ClientIntent
  +--> ClientProjection / ClientSnapshot
  +--> message / status / task projection
  |
  v
core runtime / future runtime_api
  |
  +--> providers
  +--> storage
  +--> config
  v
protocol + trace
```

其中：

- `apps/gui-tauri web shell` 只负责窗口、菜单、布局、快捷键、可访问性和渲染。
- `src-tauri bridge` 只暴露最小命令和事件通道，不实现业务状态机。
- `client` 负责 UI-neutral intent、message projection、status projection、task projection。
- `core runtime` 是唯一真实执行来源。
- `trace` 是 GUI debug、replay 和 AI 辅助修复的共同事实。

TUI 后续也应逐步复用 `client`：

```text
terminal key / GUI action
  -> ClientIntent
  -> core/runtime_api
  -> EventFrame / TraceRecord
  -> ClientProjection
  -> TUI renderer / GUI renderer
```

## 4. 技术选型

| 选项 | 角色 | 判断 |
| --- | --- | --- |
| Tauri 2 + TypeScript/React/Vite | 产品级 GUI 默认方向 | Rust backend + system WebView，适合复杂工作台 UI、可访问性、组件化、截图测试和跨平台分发；代价是引入 Web/Rust 双栈 |
| egui / eframe | 内部诊断面板候选 | Rust-first、轻量、单二进制友好，适合 inspector 和 debug console；复杂产品布局、长 scrollback、可访问性和设计系统能力需要额外验证 |
| GPUI | 观察项 | Rust 原生、高性能方向有吸引力，但仍在活跃开发、pre-1.0，v0.2 不以它作为默认路径 |
| Electron | 非默认 | UI 生态成熟，但 runtime 重、Node 边界更宽，不符合当前 Rust-first 和最小本地安全面目标 |
| Native Swift/WinUI | 非默认 | 单平台体验好，但会制造多套 GUI 和多套状态绑定 |

推荐顺序：

1. v0.1：不引入 GUI toolkit，先抽 `client`。
2. v0.2：做 Tauri shell spike，只接 mock/replay 或 read-only runtime。
3. 如果需要 Rust-only 调试面板，再单独评估 `egui` inspector。
4. GPUI 等生态、跨平台和文档稳定后再复评。

## 5. Client Model

当前 `tessera-client` 已经承接 TUI 和未来 GUI 共享的 client model：

- 用户输入转成 `ClientIntent`，其中 profile switch 和 prompt submit 使用同一套 UI-neutral intent。
- core event / trace record 转成 `ClientProjection` 消息列表。
- provider/profile/reasoning/cache/cost/task state/artifact state 进入 `ClientStatus` 投影。

后续应抽出的 UI-neutral 能力：

- `ClientIntent`：`SubmitPrompt`、`SwitchProfile`、`NewThread`、`SaveThread`、`ExportThread`、`CancelTask`。
- `ClientStatus`：profile、model、reasoning、cache、cost、task state。
- `ClientMessage`：role、content、reasoning、streaming、trace refs。
- `ClientTask`：task id、kind、status、started/completed/finished、cancel reason、error summary，已由 task registry 初版补齐。
- `ClientArtifact`：artifact id、kind、关联 thread/turn/task/item、created timestamp、referencing event kinds，已由 artifact handle projection 补齐。
- `ClientProjection`：从 `EventFrame` / `TraceRecord` 生成稳定 view state。
- `ClientSnapshot`：GUI/TUI 初始加载和 replay 恢复使用的完整投影。

TUI 可以把 terminal key event 映射成 `ClientIntent`。GUI 可以把按钮、菜单、快捷键映射成同一套 `ClientIntent`。

## 6. GUI IPC Contract

Tauri bridge 只允许暴露 typed command：

- `list_profiles`
- `load_client_snapshot`
- `submit_client_intent`
- `start_chat`
- `cancel_task`
- `load_trace_projection`
- `export_thread`

禁止暴露：

- `call_provider`
- `read_sql`
- `write_trace`
- `execute_shell`
- `run_tool`
- `read_env_secret`

Rust 到前端的事件必须是 versioned DTO：

```text
ClientEvent::Frame(EventFrame)
ClientEvent::ProjectionPatch(ClientProjectionPatch)
ClientEvent::TaskStatus(ClientTask)
ClientEvent::Error(NormalizedError)
```

规则：

- DTO 从 Rust 类型生成 TypeScript 类型，避免手写两套 schema。
- 事件通道必须 bounded；满了要回传 backpressure/cancel，而不是无限堆积。
- 前端只消费 `ClientEvent` 或 `ClientSnapshot`，不解析 provider 私有响应。
- 大内容只通过 artifact handle 展示，前端不接收无限 transcript blob。

## 7. Security And Permissions

GUI 第一版默认最小权限：

- 不启用 shell/process plugin。
- 不启用 frontend SQL plugin。
- 不让 frontend 直接发 provider HTTP 请求。
- 不让 frontend 读取 env、cookie、API key 或配置文件原文。
- 文件打开/导出通过 Rust command 做路径校验和 policy 记录。
- Tauri capability/permission 采用 allowlist，按窗口和 command 最小授权。
- 所有 secret 只以 env var 名称、keychain handle 或 redacted marker 出现在 UI。

未来工具执行进入 GUI 时，必须先有：

- `ToolDescriptor`
- `PolicyDecision`
- approval UI
- sandbox/checkpoint
- trace event

## 8. AI-Friendly Rules

GUI 代码必须 AI-ready，而不是只追求 UI 能跑：

- 一个 UI action 对应一个 `ClientIntent`。
- 一个 backend command 对应一个清晰 DTO 输入和 DTO 输出。
- 每个 projection 有 fixture。
- 每个复杂组件有 mock/replay story 数据。
- 前端组件不持有业务状态机，只接收 snapshot/patch。
- 组件文件保持小而明确，避免巨型 `App.tsx`。
- 所有跨边界类型从 Rust schema 生成或由单一 schema 派生。
- UI 文案、快捷键、命令名集中配置，避免散落硬编码。
- 关键控件保留稳定 selector，方便 Playwright 和 AI 自动化验证。
- GUI bug 优先用 replay trace 复现，而不是依赖真实 provider。

AI 修改 GUI 时优先做小任务：

- 一个 projection。
- 一个 command DTO。
- 一个组件。
- 一个 mock fixture。
- 一个截图/交互测试。

避免一次性同时改 Rust runtime、IPC、React state、布局和 provider adapter。

## 9. v0.1 到 v0.2 的准备项

- v0.1：TUI 状态投影已下沉到 `tessera-client`，TUI 只保留 terminal input、live event wrapper 和 Ratatui renderer。
- v0.1：profile switch 已按 client intent 设计，不把 profile 选择写成 Ratatui 私有逻辑。
- v0.1：live event bridge 已让 core/CLI/TUI 消费同一套 `EventFrame` 流，并保证 GUI 后续复用同一契约。
- v0.1：cancellation / timeout / backpressure 已进入 core/CLI/TUI 基础语义。
- v0.1：已抽出 `tessera-client` crate，包含 `ClientIntent`、`ClientStatus`、`ClientProjection` 和 `ClientSnapshot`。
- v0.2：做 Tauri GUI shell spike，验证 toolkit、布局、快捷键、可访问性、IPC、分发体积和 mock/replay 启动。

Tauri spike 验收标准：

- 不接真实 provider 也能展示 mock/replay 会话。
- 不直接依赖 provider SDK 或 SQLite internals。
- 只通过 typed commands 和 bounded event channel 接入 Rust。
- 至少覆盖一个 submit/cancel/replay 的 UI 路径。
- 有可重复的截图或浏览器自动化验证方案。

## 10. 不做

- 不在 v0.1 开 GUI crate/app。
- 不为了 GUI 提前引入 Web build system。
- 不让 GUI 直接访问 provider SDK。
- 不让 GUI 直接读 SQLite。
- 不让 GUI 拥有独立 session/task 状态机。
- 不让 GUI 引入 shell/file/git tool 执行旁路。
- 不把 TUI 私有状态当成 GUI 的公共模型。

## 11. References

- Tauri Architecture: https://v2.tauri.app/concept/architecture/
- Tauri Calling Rust From Frontend: https://v2.tauri.app/develop/calling-rust/
- Tauri Calling Frontend From Rust: https://v2.tauri.app/develop/calling-frontend/
- Tauri Permissions: https://v2.tauri.app/security/permissions/
- egui: https://github.com/emilk/egui
- GPUI: https://github.com/zed-industries/zed/tree/main/crates/gpui
