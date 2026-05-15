# GUI-Ready Architecture

日期：2026-05-14

本文用于提前约束 Tessera 的 GUI 方向。GUI 不进入 v0.1 交付范围，但从现在开始不能让 CLI/TUI 的实现方式阻断后续 GUI。

## 1. 原则

- GUI 不是第二套 runtime。
- GUI 不直接调用 provider。
- GUI 不直接读写 SQLite internals。
- GUI 不执行工具。
- GUI、TUI、CLI 必须共享同一套 core、protocol、trace 和 client projection。
- GUI 的第一版应该能用 mock/replay 数据启动，不依赖真实 provider。

## 2. 分层

未来 GUI 的推荐分层：

```text
GUI shell
  |
  v
client model
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

- `GUI shell` 只负责窗口、菜单、布局、快捷键、可访问性和渲染。
- `client model` 负责 UI-neutral intent、message projection、status projection、task projection。
- `core runtime` 是唯一真实执行来源。
- `trace` 是 GUI debug、replay 和 AI 辅助修复的共同事实。

## 3. 技术选型策略

v0.1 不锁定 GUI toolkit。先完成共享 client model 和 live event bridge，再做小样选择。

候选：

| Option | 适合场景 | 风险 |
| --- | --- | --- |
| Tauri | 复杂桌面 UI、Web 生态、未来可复用前端组件 | 需要管理 Web/Rust 双栈边界 |
| egui | Rust-first、单二进制、轻量调试和设置面板 | 复杂产品 UI 和可访问性需要额外验证 |
| GPUI | 原生高性能桌面体验 | 生态成熟度和跨平台交付需要验证 |

默认推荐顺序：

1. 先不选 toolkit，只固化 `client model`。
2. v0.2 做 GUI shell spike，只接 mock/replay 或 read-only runtime。
3. 如果要产品级跨平台桌面，优先验证 Tauri。
4. 如果要 Rust-first 本地工作台，优先验证 egui。

## 4. Client Model

当前 TUI 的 view-state reducer 已经给未来 client model 打了底：

- 用户输入转成 `ClientIntent`，其中 profile switch 和 prompt submit 使用同一套 UI-neutral intent。
- core event / trace record 转成消息列表。
- provider/profile/reasoning/cache/cost 进入状态栏投影。

后续应抽出的 UI-neutral 能力：

- `ClientIntent`：SubmitPrompt、SwitchProfile、NewThread、SaveThread、ExportThread、CancelTask。
- `ClientStatus`：profile、model、reasoning、cache、cost、task state。
- `ClientMessage`：role、content、reasoning、streaming、trace refs。
- `ClientProjection`：从 EventFrame / TraceRecord 生成稳定 view state。

TUI 可以把 terminal key event 映射成 `ClientIntent`。GUI 可以把按钮、菜单、快捷键映射成同一套 `ClientIntent`。

## 5. v0.1 到 v0.2 的准备项

- v0.1：保持 TUI 代码里的 view-state reducer 小而纯，禁止混入 provider/storage 访问。
- v0.1：profile switch 已按 client intent 设计，不把 profile 选择写成 Ratatui 私有逻辑。
- v0.1：live event bridge 已让 core/CLI/TUI 消费同一套 `EventFrame` 流，并保证 GUI 后续复用同一契约。
- v0.2：抽出 `client` crate 或 `core::client` 模块。
- v0.2：做 GUI shell spike，验证 toolkit、布局、快捷键、可访问性和分发体积。

## 6. 不做

- 不在 v0.1 开 GUI crate。
- 不为了 GUI 提前引入 Web build system。
- 不让 GUI 直接访问 provider SDK。
- 不让 GUI 直接读 SQLite。
- 不让 GUI 拥有独立 session/task 状态机。
