# Tessera Version Plan

日期：2026-05-27

本文是 Tessera v0.1 到 v0.9 的版本路线图源文件。它回答每个版本“为什么存在、包含什么、不包含什么、怎样算完成”。`docs/global-plan.md` 只记录当前进度和下一步执行顺序；本文件记录跨版本边界。DeepSeek-TUI、Reasonix、Codex CLI / App / App Server、Claude Code CLI / Desktop / Web 等外部方向统一沉淀在 `docs/coding-agent-direction.md`，本文件只承接其中会改变版本边界和门禁的内容。

## 1. Roadmap Rules

- 每个版本必须保留单一 headless runtime：CLI、TUI、GUI、replay、runtime API 都不得创建第二套执行系统。
- JSONL trace 是事件真相；SQLite、GUI projection、session list、task list 和 runtime API 都是可重建投影。
- Provider 只把 provider-specific stream 转成 provider-neutral events；不得执行工具、写 storage、做 policy。
- UI 只渲染和分发 intent；TUI/GUI 不直接调用 provider SDK、不读写 storage internals。
- Foundation 完成不等于用户可用 runtime 完成。凡是只定义 schema、registry、projection、planner 或 metadata helper 的项目，必须标为 foundation。
- Tool、agent、MCP、swarm、learning、long-term memory、workspace restore 等能力必须按版本门禁进入，不能因为有 schema 就宣称 runtime 已支持。
- CLI、TUI、GUI、runtime API、IDE bridge、automation 和 hook 都是 client 或 policy surface，不得拥有第二套 runtime。
- Project instruction files、skills、hooks、MCP servers、web/computer-use、automation setup scripts 都是高风险输入，必须有来源记录、作用域、权限和 trace。

## 2. Directional Compatibility Targets

Tessera 不复刻某个产品，但必须逐步具备现代 coding-agent workbench 的结构能力：

- **Codex CLI / Claude Code CLI style headless workflow:** interactive chat、one-shot prompt、resume、non-interactive run、JSON output、doctor、config validation、script/CI friendly exit status。
- **Codex App / GUI style control plane:** 多 thread/task、worktree isolation、diff/review panes、artifact previews、approvals、integrated terminal、background task reattach。
- **App-server / runtime API style protocol:** typed messages、bounded queues、auth、generated schemas、localhost default、no duplicate runtime。
- **Skills and project instructions:** `AGENTS.md` / `CLAUDE.md`-like instruction discovery and `SKILL.md`-compatible progressive disclosure, but no unchecked script execution.
- **Hooks and automations:** event-driven workflow extensions only after tool/policy/sandbox/checkpoint/task ownership exist.
- **Subagents:** context isolation and structured evidence first; swarm scheduling later.

Detailed product-direction rules live in `docs/coding-agent-direction.md`.

## 3. Status Terms

| Status | Meaning |
| --- | --- |
| Released | 已打 tag 或已作为稳定里程碑落地。 |
| Complete | 当前版本目标已完成并通过本地质量门禁。 |
| Foundation complete | schema / registry / projection / helper 已完成，但 runtime 执行能力尚未上线。 |
| In progress | 已有可运行切片，但版本目标尚未完整闭环。 |
| Planned | 设计方向明确，尚未实现。 |
| Blocked | 需要前置版本或门禁完成后才能实现。 |

## 4. Version Matrix

| Version | Theme | Current Status | Runtime Claim |
| --- | --- | --- | --- |
| v0.1 | Trace-first local runtime | Released | CLI/TUI/mock/live-provider basics are usable; no tools or agents. |
| v0.2 | Read-only projection and GUI-ready surfaces | Complete | Read-only runtime/query/projection foundations are usable; no runtime HTTP server. |
| v0.3 | Tool policy and sandbox foundations | Foundation complete | Tool metadata, policy, approval, sandbox and checkpoint planners exist; no tool execution. |
| v0.4 | Runtime API, MCP, diagnostics, memory foundations | Foundation complete | API/MCP/diagnostics/memory shapes exist; no listening server, MCP runtime, LSP runtime, or memory store. |
| v0.5 | Single-agent and resumable task foundations | Foundation stable | Pause/resume chat path, context handles, the no-tool single-agent loop, `tessera agent run`, opt-in project instruction discovery/source reporting, explicit read-only Skill Runtime v1, trace-backed task ownership foundation, automatic no-tool chat/agent owner attach/detach, and runtime API / app-server alignment DTOs are usable; runtime-complete work is explicitly staged into future gates. |
| v0.6 | Persistent sub-agents and structured review | Foundation complete | Provider-neutral structured handoff, reviewer gate, sub-agent session metadata, runtime ownership, and transcript artifact lifecycle events/projections exist; persistent child-agent execution is not implemented yet. |
| v0.7 | Project coding-agent workflow | Read-only workflow/worktree surfaces implemented | Provider-neutral coding workflow metadata, mutation request proposals, artifact body/redaction contracts, checkpoint lifecycle records, non-executing enforcement planning, read-only client/GUI projection, apply-patch preflight/dry-run readiness projection, execution records, explicit executor-ready gating, a pure patch model, isolated-root single-file UTF-8 create/modify executor, explicit `tessera apply-patch` CLI envelope, trace-driven `tessera apply-patch --from-trace` envelope, opt-in trace-driven `--auto-worktree` detached worktree lifecycle, trace-backed `tessera worktree cleanup`, read-only `tessera worktree list --trace <trace_id> [--json]`, TUI status summaries, and GUI metadata panels exist; no test execution, checkpoint restore, branch/stage/commit/push/PR Git mutation, GUI/TUI mutation control, app-server mutation listener, or broader worktree-first mutation lifecycle yet. |
| v0.8 | Swarm scheduler | Blocked | No swarm until structured handoff and reviewer gate exist. |
| v0.9 | Learning proposal system | Planned | No automatic learning or self-modification; proposals only. |

## 5. v0.1: Trace-First Local Runtime

**Goal:** Build the smallest real local runtime that proves CLI/TUI/mock/provider/storage/replay can share the same headless core.

**Status:** Released. `v0.1.0-alpha.1` and `v0.1.0` tags exist.

**Included:**

- Rust workspace with `protocol`, `client`, `core`, `providers`, `storage`, `config`, `cli`, and `tui`.
- Provider-neutral `RunEvent`, `EventFrame`, typed runtime IDs, trace records, usage/cache/cost and route-decision metadata.
- JSONL trace writer and rebuildable SQLite index.
- Mock provider, OpenAI-compatible adapter, Ollama adapter, parser tests, opt-in live smoke tests.
- Core conversation engine, event routing, trace persistence, replay runner, cancellation/timeout/backpressure basics.
- CLI `doctor`, one-shot `chat`, interactive `chat`, session discovery/resume, transcript/replay/event inspection, profile/config inspection.
- Minimal Ratatui TUI over shared `tessera-client` projection.
- v0.1 release checklist and manual testing guides.

**Excluded:**

- Tool execution, shell/file/git tools, MCP runtime, agent runtime, swarm scheduler, long-term memory runtime, learning runtime, Auto router, complex multi-window TUI, and product GUI.

**Exit Criteria:** All v0.1 release gates in `docs/v0.1-release-checklist.md` pass; at least one real provider smoke path has been verified; trace surfaces are checked for secret-like material.

**Detailed Plan:** `docs/v0.1-plan.md` remains the detailed historical v0.1 plan.

## 6. v0.2: Read-Only Projection And GUI-Ready Surfaces

**Goal:** Make runtime state observable and GUI-ready without adding new execution paths.

**Status:** Complete.

**Included:**

- `ContextWorkbench` schema and pure in-memory budget projection.
- `RuntimeReader` for event pages and indexed thread/turn/item/task/artifact queries.
- Task registry v1 and `tessera-client` task projection.
- Tauri GUI shell spike over mock/replay projection.
- Rust-to-TypeScript DTO generation with contract tests.
- GUI automation smoke test over mock/replay paths.
- Usage/cache/cost/context telemetry summaries.
- Draft manual/default `ModelRouter` and `NoProgressDetector`.
- Artifact handle projection.
- Read-only `SkillRegistry` schema.
- Snapshot/checkpoint metadata schema and read-only projection.
- Distribution plan for future release channels.

**Excluded:**

- Real GUI provider execution, listening HTTP server, Auto router execution, context file loading, skill runtime, checkpoint restore, and any tool execution.

**Exit Criteria:** GUI/mock projection, runtime reader, task/artifact/snapshot projection, and DTO bindings are covered by contract tests; no GUI bridge command bypasses core/provider/storage boundaries.

## 7. v0.3: Tool Policy And Sandbox Foundations

**Goal:** Define the governance layer required before any file, shell, git, or HTTP tool can execute.

**Status:** Foundation complete.

**Included:**

- Release identity metadata in `tessera --version`.
- `ToolDescriptor` and read-only `ToolRegistry`.
- Tool policy request/decision/approval metadata and draft `PolicyGate`.
- Ordered tool dispatch/result metadata and `OrderedToolResultBuffer`.
- Tool-call repair telemetry metadata and helper.
- Approval UI projection and UI-neutral approval intents.
- Workspace guardrail and sandbox decision metadata.
- OS sandbox profile planner.
- Workspace checkpoint planner that only creates metadata URIs.

**Excluded:**

- Actual shell/file/git/HTTP tool execution, OS sandbox process launch, side-git checkpoint creation, restore/revert, and automatic approval.

**Exit Criteria:** Any future tool runtime must start from these policy/sandbox/checkpoint contracts and produce trace events before model-visible results.

## 8. v0.4: Runtime API, MCP, Diagnostics, Memory Foundations

**Goal:** Shape integration surfaces while keeping execution ownership in core.

**Status:** Foundation complete.

**Included:**

- Metadata-only MCP adapter from MCP tool specs to Tessera tool descriptors/call requests.
- `RuntimeHttpApi` shape for JSON event pages and SSE frame encoding.
- Provider-neutral diagnostics report schema and `DiagnosticsReporter`.
- Memory proposal events and UI projection for pending/applied/rejected proposals.
- GUI typed handling for memory proposal intents without writing long-term memory.

**Excluded:**

- MCP client/server runtime, listening HTTP/SSE server, app-server auth/listener, LSP server, compiler/test runner integration, diagnostics file scanning, long-term memory store, automation runtime, hook runtime, and automatic memory writes.

**Exit Criteria:** Integrations can be replayed from trace and cannot mutate runtime state through side channels.

## 9. v0.5: Single-Agent And Resumable Task Foundations

**Goal:** Move from single chat runs toward a controlled single-agent runtime, while making pause/resume honest and trace-first.

**Status:** Foundation stable.

**Completed:**

- Agent profile schema and read-only `AgentRegistry`.
- Pause/resume lifecycle metadata and UI-neutral intents.
- Cooperative core pause signal.
- Chat pause checkpoint envelope.
- Runtime pause checkpoint projection.
- Chat-only CLI `/resume-task <task_id|#>` execution.
- Repeat resume guard and provider-profile preflight.
- Top-level `tessera tasks [--json]` and `tessera chat --resume-task <task_id|#>`.
- Context handle projection through core/client/GUI bindings.
- No-tool `AgentLoop` with provider-neutral run/step events, cancellation/pause/provider-failure/no-progress finish paths, trace replay evidence range, and machine-readable run summary.
- Non-interactive `tessera agent run --provider <id> --goal <text> [--json]` envelope for script/CI use.
- Opt-in project-local `AGENTS.md` / `CLAUDE.md` instruction discovery with source reporting, byte limits, symlink rejection, UTF-8 handling, secret-line redaction, `instructions_discovered` trace metadata, `tessera instructions inspect`, and `agent run --instructions`.
- Explicit read-only Skill Runtime v1 with project-local `SKILL.md` discovery, strict flat frontmatter parsing, byte limits, symlink rejection, duplicate/invalid source reporting, secret-line redaction, `skill_activated` trace metadata, `tessera skills inspect`, and opt-in `agent run --skill`.
- Background task ownership design spec and implementation plan for trace-backed owner leases, heartbeat metadata, lost-owner projection, and explicit reattach outcomes.
- Trace-backed background task ownership foundation: protocol IDs/events, core recorder, `RuntimeReader::list_task_owners`, client owner projection, and `tessera tasks --owners --trace <trace_id>` read-only CLI output.
- Automatic owner attach/detach integration around no-tool `ConversationEngine` and `AgentLoop` traces, including terminal owner projection for completed runs and checkpoint-based reattach metadata for paused runs.
- Runtime API / app-server alignment DTOs: explicit protocol version, localhost/unix-socket bind metadata, auth policy metadata, bounded queue policy, read-only event command envelopes, core event stream request mapping, bounded SSE frame buffer, and generated TypeScript schema evidence without a listening server.

**Deferred Beyond v0.5:**

- Executable skills, automatic/default/global skill loading, skill install/update/delete, model-driven reference selection, and script/tool skill execution remain staged behind future skill/tool policy gates.
- Default-on/global/user instruction loading, Claude imports, and `.claude/` rule compatibility remain staged until precedence, scope, source-reporting, byte-limit, redaction and trace-reference rules are designed.
- Background reattach with log/artifact projection and runtime ownership transfer remain staged behind a durable owner-process and app-server/listener gate.
- Daemon/app-server listener for durable observation/control remains staged; v0.5 only defines DTOs, queue metadata and read-only stream shapes.
- Non-chat task resume remains staged behind structured task handoff and reviewer-gate semantics.
- Real checkpoint restore remains staged behind workspace mutation, checkpoint restore/revert, policy, sandbox and trace semantics.

**Excluded Until Later:**

- Provider socket freezing, workspace restore/revert, tool execution, sub-agent persistence, hook runtime, automation runtime, swarm scheduling, and learning runtime.

**Exit Criteria:** v0.5 is foundation-stable when a no-tool single-agent loop can run against provider-neutral observations, obey cancellation/pause/no-progress controls, write trace, emit machine-readable summaries, produce honest pause/resume and task ownership metadata, expose runtime API/app-server DTO boundaries, and be replayed without relying on UI state. Runtime-complete resume, durable reattach, executable skills, app-server listeners, non-chat task resume and checkpoint restore are future-version gates, not v0.5 completion criteria.

## 10. v0.6: Persistent Sub-Agents And Structured Review

**Goal:** Add multiple durable agent sessions only after the single-agent loop and trace contracts are stable.

**Status:** Foundation complete.

**Planned Scope:**

- First foundation slice:
  - `AgentHandoffSummary` with parent/child task linkage, handoff goal, result status, short summary, evidence references, token/cost/step metrics, and trace evidence range.
  - `HandoffEvidenceRef` values for trace ranges, transcript artifacts, summary artifacts, diff artifacts, diagnostics, and test output. Evidence references are metadata only and must not inline secrets, full transcripts, file contents, provider-private responses, or command output.
  - `ReviewerGateRequest` for asking a user, parent agent, or future reviewer policy to accept, reject, or request revision over a handoff summary and bounded evidence bundle.
  - `ReviewerGateDecision` with accept/reject/request-revision status, reviewer identity label, reason code, optional comment, and trace-backed decision record.
  - Read-only projection so CLI/TUI/GUI/runtime API can inspect handoff state without loading full child transcripts or owning runtime execution.

**Completed Foundation:**

- Provider-neutral `AgentHandoffRecorded`, `ReviewerGateRequested`, and `ReviewerGateResolved` protocol events.
- Handoff/reviewer DTOs for handoff id, reviewer gate id, evidence refs, metrics, summary, request and decision metadata.
- Read-only `tessera-client` handoff/reviewer projection from live events and replayed trace records.
- Generated GUI TypeScript bindings for handoff/reviewer DTOs and trace event kinds.
- `SubagentSessionId` and `SubagentSessionDescriptor` for durable child-session metadata over trace.
- Parent/child task linkage that references existing `TaskId` values without spawning a scheduler by itself.
- Transcript artifact handles so parent sessions can inspect summaries/evidence without loading full child transcripts into context.
- `SubagentSessionCaps` for max steps, max depth, timeout, child-session count, optional cost cap and concurrency slot label.
- `SubagentApprovalForwarding` for inactive-child handling, reviewer gate linkage and optional approval id metadata.
- `SubagentSessionStatus` covering planned, active, waiting for approval, inactive, completed, failed, cancelled and handed off states.
- Provider-neutral `SubagentSessionPlanned`, `SubagentSessionStarted`, `SubagentSessionWaitingForApproval`, `SubagentSessionInactive`, and `SubagentSessionCompleted` metadata events.
- Read-only `tessera-client` sub-agent session projection from live events and replayed trace records.
- Generated GUI TypeScript bindings for sub-agent session DTOs and trace event kinds.
- Persistent sub-agent runtime ownership design gate: future scheduling must be coordinated by `core`, record task ownership and lifecycle events, publish transcript artifact handles, mediate approval forwarding through policy/reviewer gates, and make inactive-child handling deterministic before child execution is enabled.
- Provider-neutral sub-agent runtime ownership events for scheduler decisions, transcript artifact publication, approval forwarding state, inactive-child policy handling, and cancellation cascade metadata.
- Non-executing core `SubagentRuntimeCoordinator` skeleton that validates caps/reviewer/transcript metadata and returns provider-neutral runtime decisions without calling providers, tools, storage internals, or UI.
- Read-only `tessera-client` and GUI binding projection for sub-agent runtime ownership metadata from live events and replayed trace records.
- Provider-neutral `SubagentTranscriptArtifactLifecycleRecord` and `subagent_transcript_artifact_lifecycle_recorded` event for reserved, published, sealed, and abandoned transcript artifact handles.
- Non-executing core helper that validates transcript lifecycle event ranges and emits lifecycle metadata without storing transcript bodies or invoking providers, tools, storage, or UI.
- Read-only `tessera-client` and GUI binding projection for transcript artifact lifecycle records, artifact handles, and reserved/published/sealed/abandoned status summaries.
- Non-executing core helper that validates and emits approval-forwarding and inactive-policy metadata without automatic forwarding, child execution, storage writes, provider calls, tools or UI.
- Non-executing core helper that validates and emits cancellation cascade metadata without cancelling child runtime, freezing provider sockets, writing storage, executing tools or driving UI.
- Non-executing core helper that bridges sub-agent child task ids to task owner attach/detach metadata without starting child scheduling, provider execution, heartbeat loops, storage writes, tools or UI.
- Non-executing core helper that bridges sub-agent child task ids to task owner heartbeat, lost-owner and reattach metadata without heartbeat loops, lost-owner detection, provider resume, provider socket freezing, storage writes, tools or UI.

**Remaining Runtime Scope:**

- Persistent child-agent execution and scheduling over the recorded session metadata.
- Runtime lifecycle ownership for real child sessions, including start/stop/reattach semantics beyond metadata-only decisions.
- Executable child task ownership persistence, heartbeat loops, lost-owner detection and automatic reattach/resume.
- Transcript body storage, retention, summarization, and parent-context ingestion for real child runs.
- Runtime enforcement of per-agent scope, cost, recursion, timeout and concurrency limits.
- Automatic approval forwarding and inactive-child task execution.
- Executable cancellation cascade and reattach handling for real child task lifecycles.
- Skill-scoped subagent entrypoints.
- Structured evidence publication from real child runs.

**Excluded:**

- Swarm scheduler, autonomous file mutation without reviewer gate, invisible child-agent state, unbounded recursion, background fan-out without owner/cancel semantics, persistent child-agent runtime before handoff/reviewer events are replayable, and cross-agent memory writes without scope schema.

**Dependencies:**

- v0.5 single-agent loop.
- Stable task lifecycle and trace replay.
- Tool policy and approval surfaces.

**Runtime Ownership Gate:** Persistent sub-agent runtime cannot begin with direct child execution. The current foundation provides provider-neutral runtime ownership records, transcript artifact lifecycle records, approval-forwarding records, inactive-policy records, cancellation cascade records, child task owner metadata bridges for attach/detach/heartbeat/lost/reattach, a non-executing `SubagentRuntimeCoordinator` in `core`, and read-only projection of transcript artifact, lifecycle, approval-forwarding, inactive-policy and cancellation metadata. The next runtime-capable slices must integrate real child task lifecycle, task ownership persistence, heartbeat/lost-owner handling, transcript body persistence/retention, automatic approval-forwarding execution, inactive-child execution and executable cancellation/reattach handling before provider-backed child execution. CLI, TUI, GUI, client, providers and storage must not schedule child sessions or bypass core.

**Exit Criteria:** Parent agents receive structured summaries/evidence/metrics; detailed child transcripts remain trace/artifact-backed and replayable; reviewer gate can accept, reject, or request revision without relying on UI-only state. The v0.6 foundation milestones are complete: protocol/trace/client projections can represent handoff summaries, reviewer decisions, persistent sub-agent session metadata, caps, transcript artifact handles, transcript artifact lifecycle metadata, runtime ownership decisions, approval forwarding metadata, inactive-child policy state and cancellation metadata without starting persistent sub-agents, executing tools, mutating workspaces, starting an executing scheduler, or depending on UI-only state. Runtime completion still requires persistent child execution, parent/child task scheduling, automatic approval forwarding, inactive-child execution and transcript body storage/retention/summarization.

## 11. v0.7: Project Coding-Agent Workflow

**Goal:** Support codebase modification workflows through explicit diff, test, checkpoint, and rollback contracts.

**Status:** Runtime gate foundations complete; first narrow apply-patch executor, explicit CLI envelope, trace-driven `tessera apply-patch --from-trace` envelope, opt-in trace-driven `--auto-worktree` detached worktree lifecycle, conservative trace-backed `tessera worktree cleanup`, read-only worktree lifecycle listing/projection, TUI/GUI workflow/worktree observation surfaces, and metadata-only test evidence summaries are implemented.

**Completed Foundation Scope:**

- Provider-neutral coding workflow metadata events for workflow start, workspace mutation scope, patch proposals, patch application records, test plans/runs, test evidence summaries, review bundles, and restore plans.
- Non-executing core coordinator that validates relative workspace paths, required diff/test artifact references, checkpoint references, reviewer gates, and restore-plan blocking while returning `RunEvent` values only.
- Read-only client and GUI binding projection from live events and replayed trace records.
- Read-only worktree lifecycle projection in `RuntimeReader` and `tessera-client` from live events and replayed `workspace_worktree_lifecycle_recorded` records, using only worktree ids, workflow/task ids, status labels, redacted root/base labels, source commit labels, request/patch refs, and bounded evidence refs.
- Metadata-only test evidence summary records that aggregate test plan ids, test run ids, artifact refs, diagnostics, redaction state and run counts without executing tests or reading stdout/stderr bodies.
- Runtime gate assessment documenting that executable apply-patch, test execution, checkpoint restore, worktree mutation, Git mutation, hooks, automations, app-server mutation listener, swarm, and learning apply remain blocked until enforcement gates are complete.
- Mutation request proposal contracts that record operation kind, requested paths, required checkpoint, reviewer gate, policy decision, sandbox profile and worktree requirement metadata before any executor.
- Artifact body/redaction storage contracts and checkpoint lifecycle records, including blocked restore lifecycle status, without mutating a workspace.
- Non-executing mutation enforcement planner that defaults file-changing workflows to worktree-first, requires policy for explicit-local mode, records sandbox selection metadata, and rejects unsafe paths.
- Final metadata-gate verification over the v0.7 closure diff found no apply-patch, shell/test, checkpoint restore, worktree creation, Git mutation, or tool-dispatch executor path before the executor plan started.
- Apply-patch executor gate design that defines preflight and dry-run readiness as the safe slice before workspace writes.
- Apply-patch preflight and dry-run readiness contracts with metadata-only protocol events, read-only client/GUI projection, affected-path and operation summaries, blocker labels, and default `executor_blocked` behavior unless explicit executor context is supplied.
- Apply-patch execution records that distinguish actual executor result metadata from dry-run preflight metadata and patch application workflow summaries.
- Explicit executor-ready gating that requires policy, reviewer, checkpoint, sandbox, non-primary isolated root, executor capability, and supported-operation checks before writes.
- Pure in-memory single-file patch model for UTF-8 create/modify operations, conflict detection, and unsupported-operation blockers.
- Isolated-root single-file apply-patch executor that writes through temp-file + rename only inside a caller-supplied non-primary root and rejects unsafe paths or symlinked parents/targets.
- Read-only client and GUI binding projection for execution records without CLI/TUI/GUI mutation buttons.
- Final narrow-executor verification found no shell/test execution, checkpoint restore, worktree creation/lifecycle, Git mutation, tool-dispatch executor, or GUI-owned mutation path.
- Explicit `tessera apply-patch` CLI envelope that requires operator-supplied workflow/task/request/patch/preflight/execution/checkpoint/reviewer/policy refs, sandbox profile, isolated root, root label, allowed paths and exactly one patch source; it runs the core gate, records preflight metadata, invokes the isolated executor only when `ExecutorReady`, records execution metadata, and does not create worktrees, run tests, restore checkpoints, mutate Git or expose GUI/TUI mutation controls.
- Trace-driven `tessera apply-patch --from-trace` workflow automation that resolves reviewed mutation bundles from existing trace workflow, scope, mutation request, patch proposal, checkpoint, reviewer, policy, sandbox and clean patch artifact records while still supporting a caller-supplied isolated root over the existing gate/executor path.
- Opt-in trace-driven `tessera apply-patch --from-trace --auto-worktree` lifecycle that creates one detached generated worktree from source `HEAD`, rejects tracked/staged dirty source checkouts before preflight, appends path-redacted `workspace_worktree_lifecycle_recorded` events, applies the reviewed patch in the generated worktree, retains the successful worktree for operator inspection, and keeps dry-run free of worktree creation.
- Conservative trace-backed `tessera worktree cleanup` command that validates retained generated worktree lifecycle evidence plus an explicit local worktree path, appends `cleanup_started` and `cleanup_completed`/`cleanup_failed` lifecycle metadata for non-dry runs, and refuses force removal or broader Git/test/checkpoint behavior.
- Read-only `tessera worktree list --trace <trace_id> [--json]` command that projects worktree lifecycle metadata without filesystem paths and labels `trace_cleanup_candidate` as trace evidence only; actual cleanup still requires explicit `--worktree-path` validation.
- TUI status line summaries for coding workflow and worktree lifecycle counts, with the TUI remaining a view over `tessera-client` projection.
- GUI read-only Coding Workflows and Worktrees metadata panels over `ClientSnapshot` safe aggregate fields only; they omit local paths, touched paths, workflow objective text, reason text, summaries, diagnostics, artifact bodies and mutation controls.

**Runtime Gate Assessment:** The v0.7 foundation now records mutation intent, required checkpoint/reviewer/policy/sandbox metadata, artifact body/redaction handles, checkpoint lifecycle records, non-executing worktree/sandbox plans, apply-patch preflight/dry-run readiness metadata, a narrow executor result record, and path-redacted detached worktree lifecycle metadata. The first executor and CLI envelopes are intentionally small: core-owned, in-process, limited to one UTF-8 text file create/modify operation, and writing only inside an isolated non-primary mutation root after executor-ready preflight, policy, reviewer, sandbox, checkpoint and path/symlink checks pass. The implemented explicit `tessera apply-patch` CLI requires operator-supplied refs, allowed paths, patch source and isolated root, then records preflight/execution metadata through the core gate. The implemented trace-driven mode resolves prior approval metadata and clean patch artifacts from trace. The opt-in `--auto-worktree` mode only creates a detached generated worktree, records lifecycle metadata, and returns the local worktree path in CLI output. The implemented `tessera worktree cleanup` mode only removes a trace-confirmed retained generated worktree through non-force `git worktree remove`; trace records stay path-redacted. The implemented read-only worktree list, TUI summary, and GUI metadata panels only observe projection state, omit free-form objective/reason text from GUI rows, and do not add diff/test/checkpoint/Git/provider execution controls. v0.7 still does not run tests, restore checkpoints, create branches, stage, commit, push, open PRs, expose GUI mutation controls, start an app-server mutation listener or execute shell/provider tool mutations.

**Remaining Planned Scope:**

- Runtime-complete coding-agent workflow over a bounded workspace scope.
- Broader worktree-first mutation lifecycle beyond conservative cleanup.
- Diff preview and approval.
- Test runner integration through policy gates.
- Checkpoint creation before mutating operations.
- Rollback/restore trace events.
- Run summary beyond metadata-only test evidence aggregation.
- Worktree-first mutation mode.
- Code review command shape and reviewer evidence bundle.
- TUI/GUI diff, approval, artifact inspection, and mutation surfaces beyond the current read-only metadata panels.
- Git stage/commit/push/PR intents only after patch/checkpoint gates are stable.

**Excluded:**

- Unapproved shell/file/git execution, YOLO mode without trace, hidden patch application, GUI Git mutation without policy/checkpoint, and restore/revert without explicit trace events.

**Dependencies:**

- v0.3 policy/sandbox/checkpoint foundations.
- v0.5 single-agent loop.
- v0.6 reviewer gate and structured handoff.

**Exit Criteria:** Every file mutation is traceable, reviewable, testable, recoverable through documented checkpoint semantics, and inspectable from CLI/TUI/GUI without any surface owning runtime state.

## 12. v0.8: Swarm Scheduler

**Goal:** Add coordinated multi-agent scheduling after single-agent and sub-agent review contracts are stable.

**Status:** Blocked.

**Planned Scope:**

- Swarm scheduler.
- Ordered parallel exploration.
- Planner/workers/reviewer topology.
- Cost and concurrency guardrails.
- Swarm graph projection.
- Swarm replay summary.
- Scheduler policy for task priority, cancellation, retry, and partial results.
- Optional automation trigger integration after task ownership and setup gates exist.

**Excluded:**

- Swarm before structured handoff, reviewer gate, per-agent scope, no-progress detection, automation setup gates, and cost budgets are stable.

**Dependencies:**

- v0.6 persistent sub-agent sessions and reviewer gate.
- v0.7 project coding-agent workflow if swarm can mutate code.

**Exit Criteria:** Parallel work can complete out of order internally while trace/model-visible results remain deterministic, bounded, review-gated, and replayable as a swarm graph.

## 13. v0.9: Learning Proposal System

**Goal:** Let Tessera learn from traces by proposing improvements, not by silently changing runtime behavior.

**Status:** Planned.

**Planned Scope:**

- Trace mining.
- Failure pattern extraction.
- Skill improvement proposals.
- Policy rule proposals.
- Eval case generation.
- Learning ledger.
- Replay/eval before apply.
- Documentation update proposals.
- Hook/automation improvement proposals.

**Excluded:**

- Automatic self-modification, automatic policy changes, hidden memory writes, and unreviewed skill updates.

**Dependencies:**

- Stable trace schema.
- Stable skill runtime and policy gates.
- Evaluation and reviewer workflows.

**Exit Criteria:** Learning outputs are auditable proposals with evidence, risk labels, replay/eval results, and explicit user or reviewer approval before application.

## 14. Mandatory Cross-Version Gates

- No provider behavior expansion without replay fixtures.
- No file mutation tools without policy gate, sandbox decision, and checkpoint semantics.
- No shell/git/file runtime without approval UI and trace events.
- No instruction-file ingestion without precedence rules, byte limits, loaded-source reporting, secret redaction, and trace references.
- No hook runtime before tool/policy/sandbox/checkpoint exist; hooks may propose or subscribe, not bypass.
- No automation runtime before task ownership, workspace isolation, setup verification, logs, notifications, and failure reporting exist.
- No app-server listener before auth, bounded queues, health checks, generated schemas, and localhost-default binding exist.
- No GUI Git mutation before diff, checkpoint, policy and trace semantics exist.
- No Auto router execution without usage/cache/cost telemetry and user-visible route reasons.
- No automatic route escalation without no-progress loop detection.
- No subagent fan-out before explicit caps, parent/child trace linkage, artifact-backed transcripts, and reviewer gate.
- No swarm before structured handoff, reviewer gate, cost budget and deterministic result publication.
- No long-term memory runtime before scope schema, proposal review, and trace-backed apply/reject records.
- No learning apply path before proposal review and replay/eval evidence.
- No MCP environment forwarding without explicit env allowlist and secret redaction.
- No web search or computer-use default-on behavior without policy, source attribution and replay-safe event records.
