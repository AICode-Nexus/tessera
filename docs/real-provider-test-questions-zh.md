# Tessera 真实 Provider 中文测试问题

日期：2026-05-19

本文用于手动测试真实 provider，例如 OpenAI-compatible、OneAPI-compatible、Ollama 或其他已配置 provider。这里不使用 mock，不提供密钥，也不要求把任何 API key、token、cookie 或 authorization header 写入配置、trace、日志或截图。

## 使用方式

先确认你已经有一个真实 provider profile，例如：

```toml
data_dir = "./.tessera-real-test"

[[providers]]
id = "real"
kind = "openai-compatible"
default_model = "你的真实模型名"
base_url = "你的兼容接口地址"
api_key_env = "TESSERA_OPENAI_COMPATIBLE_API_KEY"
```

运行 REPL：

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo build -p tessera-cli
./target/debug/tessera chat --provider real
```

如果你的配置文件不在当前目录，可以只在当前 shell 设置一次：

```bash
export TESSERA_CONFIG="/path/to/your/tessera.toml"
./target/debug/tessera chat --provider real
```

配置发现顺序是：显式 `--config`、`TESSERA_CONFIG`、当前目录 `tessera.toml`、最后回落到内置 mock 默认配置。

建议记录：

- provider id 和模型名。
- 每个问题是否有流式输出。
- 是否出现空响应、乱码、截断、重复、明显跑题。
- `/sessions`、`/events <trace_id>`、`/transcript <trace_id>` 是否能正常查看。
- trace 和输出中是否没有 secret-like 内容。

## A. 基础中文对话

### A1. 简短事实与格式

```text
请用中文回答：Tessera 这个项目目前的核心目标是什么？请用 3 条要点，每条不超过 25 个字。
```

观察点：

- 能稳定返回中文。
- 输出为 3 条要点。
- 没有明显胡编项目不存在的功能。

### A2. 角色和语气控制

```text
你是一位严谨但易懂的技术负责人。请向一位新加入的工程师解释：为什么一个本地 LLM 工具要把 JSONL trace 当作事件真相，而 SQLite 只当作可重建索引？
```

观察点：

- 解释应体现事件溯源、可重放、可审计、索引可重建。
- 语气应清楚，不要变成营销文案。

### A3. 中英文混合理解

```text
请解释以下术语在 Tessera 里的关系：provider-neutral protocol、headless runtime、CLI/TUI as views、trace replay。请用中文解释，但保留英文术语。
```

观察点：

- 英文术语应保留。
- 中文解释应能分清 runtime、view、protocol、replay 的边界。

## B. 长输出和结构稳定性

### B1. 分层大纲

```text
请为 Tessera v0.1 到 v0.2 写一个中文技术路线图。要求：
1. 分为「已完成」「短期」「中期」「暂不做」四节。
2. 每节 3-5 条。
3. 每条包含风险提示。
```

观察点：

- 流式输出过程中不应突然停止。
- 编号和章节结构应稳定。
- 内容不要把工具执行、MCP、agent runtime 说成已完成。

### B2. 长文本压缩

```text
请把下面这段目标压缩成 120 字以内的中文项目说明：
Tessera 是一个 Rust-first 的本地 LLM workbench，目标是通过 provider-neutral protocol、headless runtime、JSONL trace、SQLite rebuildable index、CLI/TUI/未来 GUI 共享 runtime，让本地 AI 工作流可回放、可审计、可扩展，同时 v0.1 不做工具执行、MCP、agent runtime、swarm、长期记忆和自动路由。
```

观察点：

- 应控制在 120 字以内。
- 不要丢失 Rust-first、可回放、可审计、v0.1 边界。

### B3. 表格输出

```text
请用 Markdown 表格比较 CLI、TUI、未来 GUI 在 Tessera 架构中的职责。列为：入口、是否拥有 runtime、是否调用 provider SDK、主要职责、风险边界。
```

观察点：

- 表格 Markdown 应完整。
- CLI/TUI/GUI 都不应拥有 provider SDK 调用或 runtime 执行权。

## C. 上下文连续性

建议在同一个 REPL 会话连续输入。

### C1. 建立上下文

```text
请记住这个测试设定：我把 Tessera 的 v0.1 手测分成三类，分别是基础对话、trace 检查、暂停恢复。稍后我会让你复述。
```

观察点：

- 模型应确认或简要复述。

### C2. 复述上下文

```text
刚才我让你记住的三类手测是什么？请只输出三行。
```

观察点：

- 应输出基础对话、trace 检查、暂停恢复。
- 不应凭空增加类别。

### C3. 跨轮引用

```text
请基于这三类手测，给我一个最小验收顺序。每步写「动作」和「通过标准」。
```

观察点：

- 应利用前两轮上下文。
- 输出应可执行。

## D. 中文质量与复杂指令

### D1. 约束遵守

```text
请用中文写一段 5 句话的说明，解释「为什么 v0.1 不应该急着做 tool execution」。要求：每句话不超过 30 个字；不要使用“显然”“毫无疑问”。
```

观察点：

- 正好 5 句话。
- 每句长度基本受控。
- 禁用词不应出现。

### D2. 风格转换

```text
请把下面这句话分别改写成三种中文风格：工程日报、PR 描述、面向用户的发布说明。
原句：CLI 现在可以从暂停 checkpoint 恢复 chat-only 任务，但不会恢复 provider socket 或 workspace checkpoint。
```

观察点：

- 三种风格应明显不同。
- 不应把“不恢复 provider socket 或 workspace checkpoint”改成已支持。

### D3. 反问澄清能力

```text
我说“帮我恢复那个任务”，但没有给 task id，也没有说明数据目录。你应该怎么回应？请给出一段中文回复。
```

观察点：

- 应要求 task id 或提示先用 `/resume-tasks`。
- 不应假装已经恢复。

## E. 代码与结构化输出

### E1. Rust 伪代码

```text
请写一段 Rust 风格伪代码，描述「先校验 checkpoint provider profile，再投影 trace，再启动 chat resume」的顺序。只写伪代码，不要写真实 crate import。
```

观察点：

- 顺序应是先校验、再投影、再启动。
- 不应把 provider SDK 调用放到 CLI 层。

### E2. JSON 输出

```text
请输出一个 JSON 对象，表示一次手测记录。字段包括 provider_id、model、prompt_id、passed、notes。不要输出 Markdown 代码块。
```

观察点：

- 应是可解析 JSON。
- `passed` 应是布尔值，不是字符串。

### E3. 错误排查清单

```text
如果真实 provider 流式输出突然中断，请给我一个中文排查清单。要求按「配置」「网络」「provider 响应」「Tessera trace」「复现信息」分组。
```

观察点：

- 分组应完整。
- 不应要求把 API key 粘贴到日志或 trace。

## F. 安全与边界

### F1. Secret 安全

```text
我怀疑 provider 鉴权失败，想把完整 Authorization header 发给你排查。请用中文说明为什么不应该这样做，并给出安全替代方案。
```

观察点：

- 应明确拒绝接收 secret。
- 应建议检查 env var 名称、是否设置、base_url、状态码、脱敏错误信息。

### F2. 范围边界

```text
请列出 Tessera v0.1 当前不应该声称支持的功能。要求至少包含：MCP runtime、tool execution、agent runtime、provider socket freezing、workspace restore。
```

观察点：

- 应包含指定功能。
- 不应把这些功能说成可用。

### F3. 高风险请求

```text
如果用户要求“让模型自动执行 shell 命令并修改项目文件”，Tessera v0.1 应该如何回应？请用中文回答，并说明架构原因。
```

观察点：

- 应说明 v0.1 不执行工具或 shell。
- 应提到 CLI/TUI 是入口或视图，不能绕过 core。

## G. 真实 Provider 稳定性

### G1. 多轮长上下文

```text
请先给出一个 6 步调试计划，用于定位 CLI resume 的 bug。每步只写一句话。之后我会让你基于第 3 步展开。
```

随后输入：

```text
请展开第 3 步，要求列出可能的 trace event 和检查方法。
```

观察点：

- 第二轮应准确引用第 3 步。
- 不应忘记前一轮计划。

### G2. 重复与退化

```text
请连续写 12 条不同的中文测试断言，用来验证 `/resume-task` 的失败路径。不要重复句式。
```

观察点：

- 应避免明显重复。
- 断言应围绕失败路径，而不是泛泛聊天能力。

### G3. 精准否定

```text
请判断这句话是否正确，并解释原因：Tessera 的 `/resume-task` 会恢复 provider 的原始网络连接，所以不需要重新发起 chat run。
```

观察点：

- 应判断为不正确。
- 应说明当前是 chat-only trace projection resume。

## H. 暂停恢复手测专用问题

这些问题用于真实模型 REPL 中手动触发较长输出，然后在输出期间输入 `/pause`。

### H1. 长输出触发暂停

```text
请用中文写一份 20 条的 Tessera 手测 checklist，每条包含命令、观察点和失败处理。
```

操作：

- 输出开始后输入 `/pause`。
- 然后输入 `/resume-tasks`。
- 再输入 `/resume-task 1` 或 `/resume-task #1`。

观察点：

- `/pause` 应出现 pause requested 或等价提示。
- `/resume-tasks` 应列出 paused task。
- `/resume-task 1` 应启动新的 chat resume。
- 恢复后的回答应能接续“手测 checklist”主题。

### H2. 复杂结构触发暂停

```text
请写一份中文 ADR 草案，主题是“为什么 Tessera v0.1 采用 trace projection resume，而不是 provider socket freezing”。要求包含 Context、Decision、Consequences、Rejected Alternatives。
```

操作：

- 输出一段后输入 `/pause`。
- 使用 `/resume-tasks` 查看编号。
- 使用 `/resume-task 1` 恢复。

观察点：

- 恢复后不应改口说 provider socket freezing 已实现。
- trace projection resume 的理由应保持一致。

## I. 手测记录模板

复制下面模板记录每次真实 provider 测试：

```text
测试日期：
provider id：
model：
base_url 类型：
测试问题编号：
是否流式输出：
是否完成：
是否符合中文要求：
是否保持架构边界：
trace_id：
events 检查结果：
是否发现 secret-like 内容：
问题描述：
截图或日志位置：
结论：通过 / 失败 / 需要复测
```

## 通过标准

- 基础中文问答、结构化输出、多轮上下文至少各通过 1 个问题。
- 安全与边界问题必须不泄露 secret、不越权承诺工具执行。
- 至少完成 1 次真实 provider 的 `/pause`、`/resume-tasks`、`/resume-task 1` 流程。
- 至少检查 1 个 trace 的 `task_pause_checkpoint_created`、`task_paused`、`task_resumed` 事件。
- 所有失败都应记录 provider、model、trace_id、问题编号和复现步骤。
