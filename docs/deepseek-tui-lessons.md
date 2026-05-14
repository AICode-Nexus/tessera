# DeepSeek-TUI Lessons For Tessera

日期：2026-05-14

来源：`/Users/admin/work/draft/docs/superpowers/specs/2026-05-14-deepseek-tui-intro-wechat-design.md`

## 1. 目的

本文把 DeepSeek-TUI 解析稿中的优秀设计转化为 Tessera 的架构输入。

吸收原则：

- 学设计，不照搬产品形态。
- 先吸收 runtime、trace、policy、tool surface、distribution 等长期结构。
- 不把 DeepSeek-specific 能力硬编码进 Tessera core。
- 不因为 DeepSeek-TUI 已有很多功能，就把 Tessera v0.1 做成大而全。
- 所有采纳项必须落到明确阶段：v0.1、v0.2-v0.4、v0.5+ 或暂不做。

## 2. DeepSeek-TUI 值得吸收的设计

### 2.1 模型体验

值得吸收：

- 模型 profile 不只是 provider + model，还应包含 reasoning、cost、cache 和 routing 相关能力。
- 思考模式流式输出应作为 provider-neutral capability 表达，而不是 DeepSeek 专属 UI hack。
- Auto 模式的思路很好：先用轻量模型/低推理强度判断任务，再路由到合适模型。
- 前缀缓存稳定性和成本估算是用户可感知的质量能力，不应只藏在日志里。

Tessera 处理方式：

- v0.1：定义 provider capability、reasoning delta、cache usage、cost estimate 的协议和 trace 字段。
- v0.1：只做手动 profile 选择，不实现 Auto router。
- v0.2：引入本地 model router 草案和 cost/cache telemetry UI。
- v0.3+：允许 provider-specific routing strategy，但结果必须写入 trace。

### 2.2 工具与审批模式

值得吸收：

- Plan / Agent / YOLO 三模式清晰表达权限边界。
- 所有工具需要统一注册表和声明式 schema。
- shell/file/git/web/apply-patch/LSP/MCP 都必须经过同一 policy surface。
- apply-patch、edit_file、shell 输出需要结构化结果和失败原因。

Tessera 处理方式：

- v0.1：不执行工具，但协议中保留 tool、approval、policy event。
- v0.2：设计 mode model：ReadOnly / ApprovalRequired / TrustedWorkspace。
- v0.3：实现 tool descriptor、policy gate、approval UI、artifact handles。
- v0.3：shell 和 filesystem tool 进入时必须先有 OS sandbox 或 workspace path guardrail。

### 2.3 沙箱、快照和恢复

值得吸收：

- OS 级 sandbox 是 coding agent 的基础能力，不是高级功能。
- 工作区快照和回滚应独立于用户项目 `.git`。
- side-git 或等价机制能避免污染用户历史。
- `/restore`、`revert_turn` 这类恢复能力必须与 trace/task 绑定。

Tessera 处理方式：

- v0.1：只定义 artifact、task cancellation、trace event，不做文件修改。
- v0.2：设计 snapshot/checkpoint schema。
- v0.3：工具执行上线时同步上线 sandbox 和 checkpoint。
- v0.7：coding agent 工作流必须包含 diff、test、checkpoint、rollback。

### 2.4 任务、后台运行和 runtime API

值得吸收：

- 持久化任务队列让长任务可以跨重启存活。
- HTTP/SSE runtime API 让 TUI 不是唯一入口。
- `doctor --json` 是可自动化运维和 AI 调试的基础。
- 终端通知和后台任务读取接口能减少 UI 阻塞。

Tessera 处理方式：

- v0.1：Task schema、JSONL trace、SQLite index、`doctor --json`。
- v0.2：task registry v1、read-only runtime API、`since_seq` event query。
- v0.3：background task cancellation、approval wait state。
- v0.4：HTTP/SSE runtime API 扩展到 thread/task/event 查询。

### 2.5 Sub-agent 与上下文控制

值得吸收：

- sub-agent 不应只是一次函数调用，而应是可追踪、可回放的 session/task。
- 并发必须有上限。
- 父 agent 不应该把所有子 agent transcript 塞进上下文。
- `var_handle` / `handle_read` 的思想值得吸收：大结果先引用化，需要时切片读取。
- handoff 需要结构化 summary、evidence、metrics。

Tessera 处理方式：

- v0.1：Artifact 和 reserved AgentEvent 先打底。
- v0.2：artifact handle 和 large output projection 进入设计。
- v0.5：single agent loop。
- v0.6：persistent sub-agent sessions、handoff、reviewer gate。
- v0.6+：引入 `context_handle` / `artifact_slice` / structured handoff。
- v0.8：swarm 只能建立在 stable agent session 之上。

### 2.6 MCP、ACP、skills 和生态

值得吸收：

- MCP 是工具扩展面，但不能绕过内部 Tool/Policy/Trace。
- ACP 或类似编辑器协议应通过 runtime API 接入，而不是让编辑器另起状态机。
- skill discovery 要兼容现有生态路径，而不是只支持自家 manifest。
- bundled skills 能降低首启门槛，但必须有版本、来源和 policy。

Tessera 处理方式：

- v0.1：只定义 skill manifest 兼容目标，不执行 skill runtime。
- v0.2：内置只读 skill registry schema。
- v0.4：MCP adapter，把 MCP tool 转成 Tessera ToolCall。
- v0.4：runtime API 为后续 ACP/editor integration 预留。
- v0.5：skill runtime v1，优先兼容 `SKILL.md` frontmatter，再扩展 `skill.toml`。

### 2.7 分发、安装和国内可用性

值得吸收：

- Rust 二进制分发要早规划。
- npm wrapper 作为二进制下载器可降低安装门槛，但 runtime 不依赖 Node。
- Cargo/Homebrew/GitHub Releases/Docker 多路径覆盖不同人群。
- 国内镜像和 release asset mirror 是真实可用性问题。
- `doctor --json` 应检查安装、配置、provider、data dir、sandbox、LSP 等。

Tessera 处理方式：

- v0.1：只要求本地 build 和 doctor schema。
- v0.2：分发计划文档。
- v0.3+：GitHub Releases、Cargo、Homebrew、npm wrapper、Docker。
- 国内镜像变量和文档后置到分发阶段，但 config schema 可预留 release base URL。

### 2.8 多语言、成本和本地化

值得吸收：

- locale 不只是 UI 文案，也影响 currency、价格提示和国内使用体验。
- 成本面板、token usage、cache hit/miss 需要在 trace 中有原始数据。
- provider price 变化快，价格表不应硬编码到 core。

Tessera 处理方式：

- v0.1：trace 记录 usage/cache/cost estimate 的可选字段。
- v0.2：locale/currency 配置和 read-only cost summary。
- v0.3+：provider pricing registry 独立于 core，可更新、可禁用。

## 3. 纳入 Tessera 的路线调整

### v0.1 新增或强化

- Provider capability schema。
- Reasoning delta 作为可选事件。
- Usage 扩展：cache read/write/miss tokens、estimated cost、currency、latency。
- Trace extension 明确支持 route decision、cache telemetry、safe provider metadata。
- Doctor schema 提前覆盖 provider profile、data dir、trace writable、SQLite index health。
- Artifact handle 作为大输出和未来 sub-agent transcript 的统一引用。

### v0.2 新增或强化

- Model router 设计草案，先不默认开启。
- Cost/cache telemetry UI。
- Read-only runtime API：thread/event/task 查询。
- Task registry v1。
- Context/artifact handle projection。
- Snapshot/checkpoint schema。
- Skill registry schema。

### v0.3-v0.4 新增或强化

- Tool descriptor + policy gate + approval UI。
- OS sandbox / workspace path guardrail。
- Filesystem、shell、git、http、apply-patch tool。
- LSP diagnostics 作为工具或 diagnostics crate 接入。
- MCP adapter。
- HTTP/SSE runtime API。
- 分发计划。

### v0.5+ 新增或强化

- Single agent loop。
- Skill runtime v1。
- Persistent sub-agent sessions。
- Structured handoff。
- `context_handle` / `handle_read` 等大上下文控制。
- Reviewer gate。
- Coding agent diff/test/checkpoint/rollback。
- Swarm scheduler。
- Learning proposal system。

## 4. 明确不吸收或暂缓

- 不把 Tessera 做成 DeepSeek 专用客户端。
- 不在 v0.1 实现 Auto router。
- 不在 v0.1 实现工具执行。
- 不在 v0.1 实现 YOLO 模式。
- 不在 v0.1 实现 sub-agent 并发。
- 不把 provider 价格硬编码进 core。
- 不让 MCP tool 直接执行，必须通过 Tessera Tool/Policy/Trace。
- 不让 TUI 成为 runtime API 或 task manager 的真实来源。

## 5. 对现有文档的更新要求

本文件要求同步更新：

- `docs/technical-architecture.md`：加入 DeepSeek-TUI lessons 和调整后的 future path。
- `docs/v0.1-plan.md`：强化 provider capability、usage/cache/cost、doctor schema。
- `docs/protocol-v0.md`：补充 reasoning delta、provider capability、route decision、extended usage。
- `docs/trace-schema-v0.md`：补充 usage/cache/cost/latency/route/sandbox/snapshot 的 trace 预留。
- `docs/crate-boundaries.md`：补充分发、diagnostics、sandbox、runtime_api 等未来拆分门槛。
- `README.md`：加入本文档入口。
