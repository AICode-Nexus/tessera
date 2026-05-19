# Tessera Version Plan

日期：2026-05-19

本文是 Tessera v0.1 到 v0.9 的版本路线图源文件。它回答每个版本“为什么存在、包含什么、不包含什么、怎样算完成”。`docs/global-plan.md` 只记录当前进度和下一步执行顺序；本文件记录跨版本边界。

## 1. Roadmap Rules

- 每个版本必须保留单一 headless runtime：CLI、TUI、GUI、replay、runtime API 都不得创建第二套执行系统。
- JSONL trace 是事件真相；SQLite、GUI projection、session list、task list 和 runtime API 都是可重建投影。
- Provider 只把 provider-specific stream 转成 provider-neutral events；不得执行工具、写 storage、做 policy。
- UI 只渲染和分发 intent；TUI/GUI 不直接调用 provider SDK、不读写 storage internals。
- Foundation 完成不等于用户可用 runtime 完成。凡是只定义 schema、registry、projection、planner 或 metadata helper 的项目，必须标为 foundation。
- Tool、agent、MCP、swarm、learning、long-term memory、workspace restore 等能力必须按版本门禁进入，不能因为有 schema 就宣称 runtime 已支持。

## 2. Status Terms

| Status | Meaning |
| --- | --- |
| Released | 已打 tag 或已作为稳定里程碑落地。 |
| Complete | 当前版本目标已完成并通过本地质量门禁。 |
| Foundation complete | schema / registry / projection / helper 已完成，但 runtime 执行能力尚未上线。 |
| In progress | 已有可运行切片，但版本目标尚未完整闭环。 |
| Planned | 设计方向明确，尚未实现。 |
| Blocked | 需要前置版本或门禁完成后才能实现。 |

## 3. Version Matrix

| Version | Theme | Current Status | Runtime Claim |
| --- | --- | --- | --- |
| v0.1 | Trace-first local runtime | Released | CLI/TUI/mock/live-provider basics are usable; no tools or agents. |
| v0.2 | Read-only projection and GUI-ready surfaces | Complete | Read-only runtime/query/projection foundations are usable; no runtime HTTP server. |
| v0.3 | Tool policy and sandbox foundations | Foundation complete | Tool metadata, policy, approval, sandbox and checkpoint planners exist; no tool execution. |
| v0.4 | Runtime API, MCP, diagnostics, memory foundations | Foundation complete | API/MCP/diagnostics/memory shapes exist; no listening server, MCP runtime, LSP runtime, or memory store. |
| v0.5 | Single-agent and resumable task foundations | In progress | Pause/resume chat path and context handles are usable; single agent loop and skill runtime are not. |
| v0.6 | Persistent sub-agents and structured review | Planned | No persistent sub-agent runtime yet. |
| v0.7 | Project coding-agent workflow | Planned | No file-modifying coding-agent workflow yet. |
| v0.8 | Swarm scheduler | Blocked | No swarm until structured handoff and reviewer gate exist. |
| v0.9 | Learning proposal system | Planned | No automatic learning or self-modification; proposals only. |

## 4. v0.1: Trace-First Local Runtime

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

## 5. v0.2: Read-Only Projection And GUI-Ready Surfaces

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

## 6. v0.3: Tool Policy And Sandbox Foundations

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

## 7. v0.4: Runtime API, MCP, Diagnostics, Memory Foundations

**Goal:** Shape integration surfaces while keeping execution ownership in core.

**Status:** Foundation complete.

**Included:**

- Metadata-only MCP adapter from MCP tool specs to Tessera tool descriptors/call requests.
- `RuntimeHttpApi` shape for JSON event pages and SSE frame encoding.
- Provider-neutral diagnostics report schema and `DiagnosticsReporter`.
- Memory proposal events and UI projection for pending/applied/rejected proposals.
- GUI typed handling for memory proposal intents without writing long-term memory.

**Excluded:**

- MCP client/server runtime, listening HTTP/SSE server, LSP server, compiler/test runner integration, diagnostics file scanning, long-term memory store, and automatic memory writes.

**Exit Criteria:** Integrations can be replayed from trace and cannot mutate runtime state through side channels.

## 8. v0.5: Single-Agent And Resumable Task Foundations

**Goal:** Move from single chat runs toward a controlled single-agent runtime, while making pause/resume honest and trace-first.

**Status:** In progress.

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

**Remaining:**

- Single agent loop.
- Skill runtime v1.
- Durable background task ownership.
- Background reattach.
- Non-chat task resume.
- Real checkpoint restore semantics.

**Excluded Until Later:**

- Provider socket freezing, workspace restore/revert, tool execution, sub-agent persistence, swarm scheduling, and learning runtime.

**Exit Criteria:** A single-agent loop can run against provider-neutral observations, obey policy/sandbox gates, write trace, stop on no-progress conditions, and be replayed without relying on UI state.

## 9. v0.6: Persistent Sub-Agents And Structured Review

**Goal:** Add multiple durable agent sessions only after the single-agent loop and trace contracts are stable.

**Status:** Planned.

**Planned Scope:**

- Persistent sub-agent sessions.
- Structured handoff records.
- Reviewer gate.
- Parent/child task linkage.
- Agent transcript artifact handles.
- Per-agent scope, cost, recursion, and concurrency limits.

**Excluded:**

- Swarm scheduler, autonomous file mutation without reviewer gate, invisible child-agent state, unbounded recursion, and cross-agent memory writes without scope schema.

**Dependencies:**

- v0.5 single-agent loop.
- Stable task lifecycle and trace replay.
- Tool policy and approval surfaces.

**Exit Criteria:** Parent agents receive structured summaries/evidence/metrics; detailed child transcripts remain trace/artifact-backed and replayable.

## 10. v0.7: Project Coding-Agent Workflow

**Goal:** Support codebase modification workflows through explicit diff, test, checkpoint, and rollback contracts.

**Status:** Planned.

**Planned Scope:**

- Coding-agent workflow over a bounded workspace scope.
- Apply-patch tool.
- Diff preview and approval.
- Test runner integration through policy gates.
- Checkpoint creation before mutating operations.
- Rollback/restore trace events.
- Run summary and evidence bundle.

**Excluded:**

- Unapproved shell/file/git execution, YOLO mode without trace, hidden patch application, and restore/revert without explicit trace events.

**Dependencies:**

- v0.3 policy/sandbox/checkpoint foundations.
- v0.5 single-agent loop.
- v0.6 reviewer gate and structured handoff.

**Exit Criteria:** Every file mutation is traceable, reviewable, testable, and recoverable through documented checkpoint semantics.

## 11. v0.8: Swarm Scheduler

**Goal:** Add coordinated multi-agent scheduling after single-agent and sub-agent review contracts are stable.

**Status:** Blocked.

**Planned Scope:**

- Swarm scheduler.
- Ordered parallel exploration.
- Planner/workers/reviewer topology.
- Cost and concurrency guardrails.
- Swarm graph projection.
- Swarm replay summary.

**Excluded:**

- Swarm before structured handoff, reviewer gate, per-agent scope, and no-progress detection are stable.

**Dependencies:**

- v0.6 persistent sub-agent sessions and reviewer gate.
- v0.7 project coding-agent workflow if swarm can mutate code.

**Exit Criteria:** Parallel work can complete out of order internally while trace/model-visible results remain deterministic and review-gated.

## 12. v0.9: Learning Proposal System

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

**Excluded:**

- Automatic self-modification, automatic policy changes, hidden memory writes, and unreviewed skill updates.

**Dependencies:**

- Stable trace schema.
- Stable skill runtime and policy gates.
- Evaluation and reviewer workflows.

**Exit Criteria:** Learning outputs are auditable proposals with evidence, risk labels, and explicit user or reviewer approval before application.

## 13. Mandatory Cross-Version Gates

- No provider behavior expansion without replay fixtures.
- No file mutation tools without policy gate, sandbox decision, and checkpoint semantics.
- No shell/git/file runtime without approval UI and trace events.
- No Auto router execution without usage/cache/cost telemetry and user-visible route reasons.
- No automatic route escalation without no-progress loop detection.
- No swarm before structured handoff and reviewer gate.
- No long-term memory runtime before scope schema, proposal review, and trace-backed apply/reject records.
- No learning apply path before proposal review and replay/eval evidence.
