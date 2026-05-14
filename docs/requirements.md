# Tessera Requirements

日期：2026-05-14

## 1. 项目定位

Tessera 是一个模型无关、面向 agent 演进、支持多任务多窗口、工具调用可审计、运行可回放的终端大模型工作台。

英文定位：

> Tessera is a model-agnostic, agent-ready terminal workbench built on typed events, auditable tools, replayable runs, and composable skills.

Tessera 不应只是聊天 TUI，也不应绑定单一模型。它应成为一个高质量本地 AI runtime，TUI、CLI、runtime API、工具系统、agent 系统和未来自学习系统都共享同一套协议和状态模型。

## 2. 当前阶段目标

当前阶段只做需求和架构设计，不做实现。

本阶段输出：

- 项目 README。
- 产品需求说明。
- 架构设计文档。
- DeepSeek-TUI 对照审查。
- v0.1 范围边界。

本阶段不做：

- Rust workspace scaffold。
- TUI 代码。
- Provider adapter。
- 工具执行。
- Agent runtime。
- MCP runtime。
- 多窗口 UI 实现。

## 3. 核心用户需求

### 3.1 通用模型工作台

Tessera 应支持多个模型和 provider：

- OpenAI-compatible API。
- Ollama。
- DeepSeek。
- Anthropic。
- Gemini。
- Qwen 或其他兼容服务。

内部协议不能被某一家 provider 的私有结构污染。Provider 专属能力可以通过 extension metadata 暴露。

### 3.2 质量优先

Tessera 选择 Rust-first 路线，目标是长期质量、本地安全、可维护性和可分发性。

质量要求：

- 单一真实 runtime 来源。
- Headless core 优先。
- 类型化协议。
- 可回放事件。
- 可审计工具调用。
- 可恢复任务。
- 明确 schema version。
- 不把核心逻辑沉淀在 TUI。

### 3.3 Agent-ready 架构

第一版不急于实现完整 agent，但架构必须支持：

- 单 agent loop。
- Persistent sub-agent sessions。
- 多 agent handoff。
- Swarm scheduler。
- Skill 系统。
- Memory 系统。
- Learning proposal 系统。
- Tool approval 和 policy。

### 3.4 多任务和多窗口

Tessera 必须从设计上区分 Task 和 Window：

- Task 是运行时对象。
- Window 是观察和控制任务的视图。
- 关闭 Window 不应隐式取消 Task。
- 一个 Task 可以绑定多个 Window。
- 一个 Window 可以切换绑定到不同 Task。

### 3.5 可审计工具系统

所有工具调用必须经过统一 policy：

- shell。
- filesystem。
- git。
- http。
- MCP tool。
- agent tool。

工具执行应记录 trace、approval、输入、输出、错误和 artifact。

### 3.6 可回放运行记录

Tessera 应记录完整运行过程：

- Thread。
- Turn。
- Item。
- Task。
- Artifact。
- EventFrame。
- Trace JSONL。

目标是离线复现、调试、审计和 AI 辅助修复。

### 3.7 Skill 系统

Skill 是可复用能力包，不是任意执行代码的插件。

Skill 应支持：

- `SKILL.md` frontmatter 兼容。
- 后续可扩展 `skill.toml`。
- instructions。
- workflows。
- fixtures。
- evals。
- tool requirements。
- policy requirements。

### 3.8 Memory 系统

Memory 不能只是 prompt 拼接。它应具备：

- scope。
- type。
- source trace。
- confidence。
- expiry。
- review/proposal flow。

长期记忆不应默认全局共享。

### 3.9 自学习系统

自学习系统默认只生成提案，不静默修改行为。

闭环：

```text
observe -> extract -> propose -> verify -> approve -> apply
```

应用前必须经过用户批准和 replay/eval 验证。

## 4. v0.1 范围

v0.1 应建立最小真实 runtime，而不是只做漂亮 TUI。

v0.1 包含：

- Rust workspace 边界。
- `protocol` 基础类型。
- `core` 事件协议。
- OpenAI-compatible provider。
- Ollama provider。
- `secrets` 环境变量读取和本地安全存储占位。
- Headless `cli chat`。
- `cli doctor --json`。
- 最小 Ratatui TUI。
- 流式输出。
- 本地 session 保存。
- JSONL trace。
- 基础 TaskId / WindowId / task-window binding schema。
- `/model`、`/new`、`/save`、`/export`。
- provider mock 和基础测试。

v0.1 不包含：

- 多 agent runtime。
- Swarm runtime。
- MCP 完整生态。
- 自动改代码。
- 自动执行 shell。
- 复杂多窗口布局。
- 自学习执行 runtime。
- 长期记忆 runtime。

## 5. 成功标准

需求设计阶段完成时：

- README 清楚说明项目定位和当前状态。
- requirements 文档清楚定义 v0.1 范围和非目标。
- architecture 文档明确核心边界、路线、风险和 DeepSeek-TUI 对照结论。
- 仓库中没有实现代码。

v0.1 实现完成时：

- CLI 可以发起一次流式对话。
- TUI 可以完成一次流式对话。
- 可以在 OpenAI-compatible 和 Ollama profile 之间切换。
- 每次运行都有 JSONL trace。
- 会话可以保存和恢复。
- `cargo test` 覆盖 core、provider mock、storage 基础行为。
- TUI 层没有 provider SDK 依赖。
- provider adapter 没有 tool 执行逻辑。
- 没有 API key 明文进入仓库或 session 文件。
