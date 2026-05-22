# Coding-Agent Direction For Tessera

日期：2026-05-19

本文把 DeepSeek-TUI、Reasonix、Codex CLI / App / App Server、Claude Code CLI / Desktop / Web 的方向沉淀为 Tessera 的产品和架构约束。它不是竞品功能清单，也不是要求逐项复刻。它回答一个问题：Tessera 要发展成现代 coding agent workbench 时，哪些能力必须提前设计成一等公民。

## 1. Sources And Scope

已纳入的参考输入：

- `docs/deepseek-tui-lessons.md`
- `docs/reasonix-lessons.md`
- OpenAI Codex docs: [CLI features](https://developers.openai.com/codex/cli/features), [App features](https://developers.openai.com/codex/app/features), [App Server](https://developers.openai.com/codex/app-server), [AGENTS.md](https://developers.openai.com/codex/guides/agents-md), [Skills](https://developers.openai.com/codex/skills), [Subagents](https://developers.openai.com/codex/subagents)
- OpenAI Codex repository: [openai/codex](https://github.com/openai/codex)
- Claude Code docs: [Overview](https://docs.anthropic.com/en/docs/claude-code/overview), [Subagents](https://docs.anthropic.com/en/docs/claude-code/sub-agents), [Skills](https://docs.anthropic.com/en/docs/claude-code/skills), [Hooks](https://docs.anthropic.com/en/docs/claude-code/hooks)

Tessera 只吸收长期结构：

- CLI / TUI / GUI / app-server 都是同一 runtime 的不同 client。
- Thread、Task、Artifact、Approval、Trace、Policy 是产品对象，不是 UI 临时状态。
- Coding-agent 能力必须可审计、可恢复、可回放。
- Skills、hooks、subagents、automations 都必须进入 policy、scope 和 trace，而不是成为旁路插件。
- GUI 的价值不只是漂亮界面，而是并行任务、worktree、diff/review、approval、artifact 和 terminal 的统一控制面。

## 2. Directional Compatibility Targets

这些目标用于指导路线图，不要求早期版本完全兼容外部工具。

### Codex CLI Direction

应吸收：

- 终端入口不仅是 chat，还要支持交互式会话、单次 prompt、resume、non-interactive execution 和 JSON 输出。
- approval mode 和 sandbox mode 是两个不同维度：一个回答“什么时候问用户”，一个回答“技术上能访问哪里”。
- 项目指令文件需要有确定的发现顺序、覆盖规则和大小上限。
- Web search、image input、shell、apply-patch、MCP、skills、subagents 都是可组合能力，但必须受 profile、policy、trace 和 sandbox 限制。
- `app-server` 类协议说明富客户端不应复制 runtime；它们应通过 typed messages、bounded queues、auth 和 generated schemas 与 runtime 通信。

Tessera 处理方式：

- v0.5 前只保留 chat/resume/tasks 等 headless primitives；不急着追平完整 coding-agent CLI。
- v0.5 设计单 agent loop 时，同时定义 non-interactive agent run 的输入/输出 envelope，但先禁止 file mutation。
- v0.5 设计 project instruction discovery：`AGENTS.md`、未来 `CLAUDE.md` / fallback names 只能作为 context references 进入 trace，必须有 precedence、byte limit、redaction 和 loaded-source report。
- v0.7 才进入 apply-patch、diff/test/checkpoint/rollback 和 code review command 方向。
- runtime API / app-server 必须是 core 的薄协议壳，不能成为第二套 runtime。

### Codex App / GUI Direction

应吸收：

- 一个窗口管理多个项目和多个线程，线程可运行在 local、worktree 或 cloud-like environment。
- Worktree 是并行编码任务和自动化的默认隔离手段。
- GUI 应有 diff pane、review comments、stage/revert/commit/push/PR 等 Git 控制面，但这些操作必须经过 trace、checkpoint 和 policy。
- 每个 thread 可带 integrated terminal，terminal 输出可被 agent 引用为 evidence。
- Automations 与 skills 组合后能处理重复任务，但必须有 schedule、workspace、setup、model、权限和输出边界。
- In-app browser、computer use、artifact previews 是验证和反馈面，不应绕过 runtime policy。

Tessera 处理方式：

- v0.2 的 GUI shell 只做 mock/replay 和 read-only projection 是正确的。
- v0.5-v0.6 应先让 GUI 展示真实 task lifecycle、approvals、artifacts、background reattach 和 runtime events。
- v0.7 后 GUI 才能提供 Git/diff/review/patch controls，并且每个操作都要映射为 typed client intent 和 trace event。
- Worktree mode 应成为 coding-agent workflow 的首选写入模式；local mode 只能在明确 scope 下启用。
- Automations 后置到 task runtime、skills、worktree、sandbox、notifications 和 failure reporting 稳定之后。

### Claude Code CLI Direction

应吸收：

- CLI 要可组合：pipe input、script mode、CI mode、计划/执行/验证流程都应是稳定入口。
- `CLAUDE.md`、skills、hooks、subagents、background agents、scheduled tasks 和 remote handoff 共同构成“工作流平台”，不是单纯聊天命令。
- Subagent 的核心价值是隔离上下文和工具工作；父会话只接收 summary/evidence/metrics。
- Hooks 能连接开发流程，但 shell hook 风险很高，必须被当成 tool/policy/runtime event，而不是无约束 shell。
- Skills 应优先兼容 `SKILL.md` progressive disclosure，支持引用文件和脚本，但脚本执行必须经 tool/policy。
- 多 surface 使用同一底层 engine；terminal、IDE、desktop、web、CI 不应分裂状态。

Tessera 处理方式：

- v0.5 skill runtime v1 优先兼容 `SKILL.md`，并记录 skill activation / step events。
- v0.6 subagents 必须有 per-agent scope、model、tool permissions、memory scope、timeout、cost and depth limits。
- v0.6 reviewer gate 是 subagent workflow 的最小安全闭环；没有 reviewer gate 不做 swarm。
- Hook runtime 不早于 tool/policy/sandbox/checkpoint；hook 只能订阅标准 event，输出只能是 context、decision request 或 traced command proposal。
- Background task 不是“把进程藏起来”；它必须有 owner、cancel、pause、resume/reattach、log/artifact、failure summary 和 replay contract。

### DeepSeek-TUI / Reasonix Direction

已纳入的关键点：

- Runtime、task、trace、policy、sandbox、snapshot 和 app-server 比 UI 外观更重要。
- Cache-stable context、append-only transcript、volatile scratch 和 visible cost control 是长期质量能力。
- Ordered parallel tool dispatch 需要 deterministic model-visible result order。
- Tool repair telemetry 应 provider-neutral，不能把 raw reasoning 或 secret 写进 trace。
- Subagent 是上下文控制机制，不是 v0.1/v0.2 的 swarm 能力。

Tessera 处理方式：

- 继续保持 Rust-first、headless-first、trace-first。
- 保持 Foundation complete 与 runtime complete 的清晰区别。
- 所有 provider-specific 优化只能进入 capability、safe extension 或 route decision。

## 3. Product Surface Model

Tessera 后续产品面应按同一运行时对象展开。

| Surface | Purpose | Runtime Rule |
| --- | --- | --- |
| CLI | scriptable headless entry, CI, non-interactive execution, resume, doctor, replay | CLI never bypasses core or writes storage internals. |
| TUI | local terminal workbench, live approvals, task switching, compact review | TUI is a view over client projection and runtime events. |
| GUI | multi-project/thread control plane, diff/review, artifact previews, integrated terminal | GUI is a rich client, not a runtime owner. |
| App server / runtime API | typed protocol for GUI, IDE, remote, automation clients | Thin wrapper over core; bounded queues, auth, generated schemas. |
| Automations | scheduled or event-triggered task creation | Requires task ownership, worktree/sandbox, setup gate, logs and notification policy. |
| Hooks | event subscribers and decision/context contributors | No raw shell bypass; all side effects become tool calls or traced proposals. |
| Skills | reusable workflows and reference bundles | Progressive disclosure; scripts/tools still require policy. |
| Subagents | isolated context workers with structured handoff | Explicit scope, caps, trace, evidence, reviewer gate. |

## 4. Roadmap Implications

### v0.5 Must Tighten

v0.5 should not only say "single agent loop". It must include:

- Agent run envelope: input, active profile, context refs, instruction sources, stop policy, output summary.
- Non-interactive run shape: machine-readable result, trace id, task id, evidence refs, exit status.
- Project instruction discovery foundation: loaded-source report, precedence, byte limit, fallback names, redaction.
- Skill runtime v1 foundation: `SKILL.md` discovery, activation trace, read-only references, no unchecked scripts.
- Background task ownership: owner client, cancel/pause semantics, logs/artifacts, reattach summary.
- App-server design alignment: JSON / JSON-RPC-compatible message shape, bounded queues, generated DTO/schema, localhost default. Current foundation covers metadata/schema shape only; listener, daemon ownership, remote control and provider execution remain gated.

### v0.6 Must Be Review-First

v0.6 should add subagents only with:

- Structured handoff records.
- Parent/child task linkage.
- Per-agent profile, scope, model, permissions, timeout and cost caps.
- Transcript artifact handles instead of context dumping.
- Reviewer gate before code-modifying results are accepted.
- Approval forwarding semantics for inactive child tasks.

### v0.7 Must Be Coding-Workflow Complete

v0.7 should be the first version that can honestly claim coding-agent workflow:

- Apply-patch / edit tool with diff preview.
- Test and lint runner integration through policy.
- Worktree-first mutation mode.
- Checkpoint before mutation and restore/revert trace events.
- Review command and evidence bundle.
- GUI/TUI diff and approval surfaces.
- Git stage/commit/push/PR intents only after patch/checkpoint gates exist.

### v0.8 Must Not Be "More Agents"

v0.8 swarm scheduler must be a deterministic scheduler over already-safe agent sessions:

- Planner/worker/reviewer topology.
- Bounded parallelism and cost budgets.
- Ordered result publication.
- Failure isolation and partial result reporting.
- Replayable swarm graph.

### v0.9 Must Stay Proposal-First

Learning should produce:

- Skill improvement proposals.
- Policy proposals.
- Eval cases.
- Documentation update proposals.
- Failure pattern summaries.

It must not silently update skills, hooks, memory, policy, prompts or routes.

## 5. New Gates

These gates complement `docs/version-plan.md`.

- No instruction-file ingestion without precedence rules, byte limits, source reporting and secret redaction.
- No hook runtime before tool/policy/sandbox/checkpoint exist; hooks may propose, not bypass.
- No automation runtime before task ownership, logs, notifications, setup verification and workspace isolation exist.
- No app-server listener before auth, bounded queues, health checks and generated schemas exist; the current `RuntimeApi*` DTOs satisfy schema/auth/queue shape but not listener readiness by themselves.
- No GUI Git mutation before diff, checkpoint, policy and trace semantics exist.
- No subagent fan-out before explicit caps, parent/child trace linkage, transcript artifact handles and reviewer gate exist.
- No MCP environment forwarding without explicit env allowlist and secret redaction.
- No web search or computer-use default-on behavior without policy, source attribution and replay-safe event records.

## 6. Documentation Control

When this direction changes:

- Update `docs/version-plan.md` if the version boundary, gate or exit criterion changes.
- Update `docs/global-plan.md` if current execution order or status changes.
- Update `docs/technical-architecture.md` if runtime, UI, provider, storage or API ownership changes.
- Update `docs/gui-ready-architecture.md` if GUI surface or app-server expectations change.
- Update `docs/crate-boundaries.md` if a new crate or forbidden dependency direction is introduced.
