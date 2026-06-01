# Coding-Agent Direction For Tessera

日期：2026-05-27

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

当前 v0.7 coding-agent runtime 已包含 read-only workflow/worktree observation 和 conservative worktree cleanup：`tessera worktree list --trace <trace_id> [--json]`、TUI status summary 和 GUI metadata panels 只展示 trace/client projection；`tessera worktree cleanup` 只能清理 trace 证明由 Tessera 生成并 retained 的 detached worktree，并要求调用者提供本地路径。它们都不是 test runner、checkpoint restore、branch/stage/commit/push/PR、GUI Git mutation、GUI mutation control、app-server mutation listener 或通用 `git worktree prune`。

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

- v0.5 已收口为 foundation-stable：chat/resume/tasks、no-tool single-agent loop、opt-in project instructions、explicit read-only skills、task ownership metadata 和 runtime API DTOs 可用，但不声称完整 coding-agent CLI。
- v0.5 的 non-interactive agent run 只提供无工具、无文件修改的输入/输出 envelope。
- v0.5 的 project instruction discovery 只支持 opt-in `AGENTS.md` / `CLAUDE.md` source report；default/global/user loading、Claude imports 和 `.claude/` 兼容性继续等待 precedence、byte limit、redaction 和 trace-reference gate。
- v0.7 已具备 metadata-only coding workflow foundation、窄的 explicit `tessera apply-patch` CLI envelope、trace-driven `tessera apply-patch --from-trace` CLI envelope、opt-in `--auto-worktree` detached worktree lifecycle、read-only `tessera worktree list --trace <trace_id> [--json]`、TUI/GUI workflow/worktree observation，以及 trace-backed `tessera worktree cleanup`；diff/test execution、checkpoint restore/rollback、broader worktree lifecycle、GUI mutation controls、app-server mutation listener 和 code review command 仍按后续 runtime gate 推进。
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
- v0.5-v0.6 应先让 GUI 展示真实 task lifecycle、approvals、artifacts、task ownership metadata、handoff evidence 和 runtime events；background reattach 仍等待 app-server/listener/daemon owner gate。
- v0.7 metadata foundation 让 GUI 可以只读展示 coding workflow 和 worktree lifecycle 的安全 aggregate fields；当前 GUI panels 不展示本地路径、touched paths、reason text、summaries、diagnostics、artifact bodies，也不提供 apply/cleanup/restore/test/Git/shell/provider mutation controls。当前 apply-patch mutation 只允许通过 explicit 或 trace-driven CLI envelope，automatic worktree lifecycle 也只通过 `--from-trace --auto-worktree` / `worktree cleanup` CLI paths 暴露本地 worktree path 和清理入口，Git/diff/review/patch controls 仍必须等待对应 typed client intent、policy、checkpoint 和 trace event。
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

- v0.5 skill runtime v1 优先兼容 `SKILL.md`，并记录 explicit activation metadata；executable skills、default/global loading 和 skill install/update/delete 仍等待 future skill/tool policy gate。
- v0.6 subagents 必须先有 handoff summary、evidence refs、reviewer gate、per-agent scope、model、tool permissions、memory scope、timeout、cost and depth limits。
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

### v0.5 Is Foundation-Stable

v0.5 is closed as foundation-stable, not runtime-complete. It includes:

- Agent run envelope: input, active profile, context refs, instruction sources, stop policy, output summary.
- Non-interactive run shape: machine-readable result, trace id, task id, evidence refs, exit status.
- Project instruction discovery foundation: loaded-source report, precedence, byte limit, fallback names, redaction.
- Skill runtime v1 foundation: `SKILL.md` discovery, activation trace, read-only references, no unchecked scripts.
- Background task ownership: owner client metadata, cancel/pause semantics, heartbeat/lost-owner projection and explicit reattach outcome metadata.
- App-server design alignment: JSON / JSON-RPC-compatible message shape, bounded queues, generated DTO/schema, localhost default. Current foundation covers metadata/schema shape only; listener, daemon ownership, remote control and provider execution remain gated.

Deferred beyond v0.5:

- Executable/default/global skills and skill install/update/delete.
- Default/global/user instruction loading, Claude imports and `.claude/` compatibility.
- Durable background reattach, app-server listener, provider socket freezing and runtime ownership transfer after process exit.
- Non-chat task resume and real checkpoint restore.

### v0.6 Must Be Review-First

v0.6 should start with structured handoff and reviewer gate foundations. Persistent subagents should arrive only after these contracts are replayable:

- `AgentHandoffSummary` records with parent/child linkage, objective, status, short summary, metrics and evidence range.
- `HandoffEvidenceRef` metadata for trace ranges, transcript artifacts, summary artifacts, diffs, diagnostics and test output, without inlining full transcripts or secrets.
- `ReviewerGateRequest` records that ask for accept/reject/request-revision against a bounded evidence bundle.
- `ReviewerGateDecision` records that make reviewer outcome trace-backed and inspectable from CLI/TUI/GUI/runtime API.
- `SubagentSessionDescriptor` records that make child-session scope, caps, transcript artifacts, approval forwarding and inactive-child policy visible before any persistent scheduler exists.
- Persistent sub-agent scheduler/runtime ownership must live in core, reuse task lifecycle and task ownership records, and publish runtime decisions before any child provider call.
- Parent/child task linkage.
- Per-agent profile, scope, model, permissions, timeout and cost caps.
- Transcript artifact handles instead of context dumping.
- Reviewer gate before code-modifying results are accepted.
- Approval forwarding semantics for inactive child tasks as trace metadata first; automatic forwarding remains gated behind policy/reviewer/runtime implementation.
- Inactive child handling must be deterministic: pause parent, queue decision, or require reviewer. Silent continuation is not a valid subagent UX.

### v0.7 Must Be Coding-Workflow Complete

v0.7 should be the first version that can honestly claim coding-agent workflow. The current foundation can represent and replay workflow scope, patch proposals, patch application records, test plans/runs, metadata-only test evidence summaries, review bundles, restore plans and worktree lifecycle records through protocol/core/client/GUI bindings. It also has narrow explicit and trace-driven `tessera apply-patch` CLI envelopes for one gated isolated-root file mutation, opt-in automatic detached worktree creation for trace-driven apply-patch, conservative cleanup, read-only worktree listing, TUI summaries and GUI metadata panels, but it still is not a full coding-agent diff/test/checkpoint/Git workflow.

Remaining runtime-complete work:

- Broader trace-backed worktree lifecycle mutation beyond current auto-worktree creation, read-only listing and conservative cleanup.
- Test and lint runner integration through policy.
- Worktree-first mutation mode.
- Checkpoint before mutation and restore/revert trace events.
- Review command and evidence bundle.
- GUI/TUI diff, approval, artifact inspection and mutation surfaces beyond current read-only metadata panels.
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
- No subagent fan-out before explicit caps, parent/child trace linkage, core-owned runtime decisions, transcript artifact handles and reviewer gate exist.
- No MCP environment forwarding without explicit env allowlist and secret redaction.
- No web search or computer-use default-on behavior without policy, source attribution and replay-safe event records.

## 6. Documentation Control

When this direction changes:

- Update `docs/version-plan.md` if the version boundary, gate or exit criterion changes.
- Update `docs/global-plan.md` if current execution order or status changes.
- Update `docs/technical-architecture.md` if runtime, UI, provider, storage or API ownership changes.
- Update `docs/gui-ready-architecture.md` if GUI surface or app-server expectations change.
- Update `docs/crate-boundaries.md` if a new crate or forbidden dependency direction is introduced.
