# Tessera Global Plan

日期：2026-05-27

本文是 Tessera 当前进度仪表盘和执行控制面。它不再承担完整版本路线图职责；完整 v0.1-v0.9 版本边界见 [Version Plan](version-plan.md)。v0.1 的详细历史计划仍保留在 [v0.1 Plan](v0.1-plan.md)。

## 1. Document Hierarchy

| Document | Role |
| --- | --- |
| `docs/version-plan.md` | v0.1-v0.9 的版本目标、边界、退出标准和跨版本门禁。 |
| `docs/global-plan.md` | 当前状态、下一步执行顺序、未完成项和更新规则。 |
| `docs/v0.1-plan.md` | v0.1 已发布阶段的详细历史实施计划。 |
| `docs/v0.1-release-checklist.md` | v0.1 alpha/final tag 门禁和 release notes。 |
| `docs/technical-architecture.md` | 当前技术架构和不可破坏的 runtime/UI/provider/storage 边界。 |
| `docs/coding-agent-direction.md` | DeepSeek-TUI、Reasonix、Codex CLI/GUI/App Server、Claude Code CLI/Desktop/Web 的方向吸收和产品门禁。 |
| `docs/crate-boundaries.md` | crate 依赖方向、职责和禁止事项。 |
| `docs/protocol-v0.md` / `docs/trace-schema-v0.md` | provider-neutral 协议和 trace schema 合约。 |
| `CHANGELOG.md` | 用户可见变化和 release history。 |

## 2. Status Legend

- `[x]` Done：已实现并通过对应门禁。
- `[~]` Partial：已有 foundation 或局部可用路径，但不能宣称完整用户能力。
- `[ ]` Planned：尚未实现。
- `[!]` Blocked：前置门禁未满足，不应开始实现。

## 3. Current Implementation State

当前 `main` 基线已完成 v0.1 release，并已将 v0.2-v0.5 收口为 foundation-stable 工作。必须明确区分：

- **用户可用能力**：CLI/TUI/mock chat、trace/replay、session resume、chat-only paused task resume、opt-in project instructions、explicit read-only skill activation、explicit and trace-driven `tessera apply-patch` isolated-root CLI envelopes、trace-driven `--auto-worktree` detached worktree apply-patch、read-only `tessera worktree list --trace <trace_id> [--json]`、trace-backed `tessera worktree cleanup`、TUI workflow/worktree status summaries、GUI read-only workflow/worktree metadata panels 等。
- **foundation 能力**：tool schema、policy gate draft、sandbox planner、MCP metadata adapter、runtime HTTP/SSE shape、runtime API / app-server DTO alignment、diagnostics event、memory proposal UI、trace-backed task ownership metadata/recorder/projection、no-tool chat/agent run owner attach/detach、structured handoff/reviewer gate、sub-agent session/runtime ownership/transcript lifecycle metadata projection、metadata-only coding-agent workflow protocol/core/client projection、worktree lifecycle RuntimeReader/client projection、v0.7 runtime gate assessment、runtime gate closure foundations、apply-patch execution record、explicit executor-ready gate、pure patch model、isolated-root single-file apply-patch executor 和 automatic isolated worktree lifecycle metadata/runner boundary 等。
- **尚未支持能力**：真实 tool execution、executable/default/global skill runtime、tool-using/full coding-agent runtime、MCP runtime、background reattach、daemon/app-server listener、app-server mutation listener、default-on/global project instruction ingestion、hook runtime、automation runtime、workspace restore、persistent sub-agent runtime、完整 coding-agent diff/test/checkpoint/Git execution workflow、test runner、checkpoint restore、branch/stage/commit/push/PR Git mutation、GUI mutation controls、broader worktree-first mutation lifecycle、swarm、learning apply。

## 4. Version Status Matrix

| Version | Theme | Status | Current Evidence |
| --- | --- | --- | --- |
| v0.1 | Trace-first local runtime | [x] Released | `v0.1.0-alpha.1` and `v0.1.0` tags exist; release checklist complete. |
| v0.2 | Read-only projection and GUI-ready surfaces | [x] Done | RuntimeReader, task/artifact/snapshot projection, GUI shell spike, DTO bindings, distribution plan. |
| v0.3 | Tool policy and sandbox foundations | [~] Foundation complete | Tool descriptor, policy gate, approval projection, sandbox decision, OS sandbox planner, checkpoint planner; no execution. |
| v0.4 | Runtime API, MCP, diagnostics, memory foundations | [~] Foundation complete | RuntimeHttpApi shape, metadata-only MCP adapter, diagnostics event, memory proposal UI; no server/runtime/store. |
| v0.5 | Single-agent and resumable task foundations | [~] Foundation stable | Agent profile registry, context handles, cooperative pause, chat-only task resume, top-level paused task CLI, no-tool `AgentLoop`, `tessera agent run`, opt-in project instruction discovery/source reporting, explicit read-only Skill Runtime v1, trace-backed task ownership foundation, automatic no-tool chat/agent owner attach/detach, and runtime API / app-server DTO alignment; runtime-complete gaps are staged into future gates. |
| v0.6 | Persistent sub-agents and structured review | [x] Foundation complete | Structured handoff/reviewer gate, sub-agent session/runtime ownership, and transcript artifact lifecycle metadata events/projections exist; persistent child-agent execution is not implemented yet. |
| v0.7 | Project coding-agent workflow | [~] Read-only workflow/worktree surfaces implemented | Provider-neutral workflow/scope/patch/test/review/restore metadata events, mutation request proposals, artifact body/redaction contracts, checkpoint lifecycle records, non-executing enforcement planner, read-only client/GUI projection, apply-patch preflight/dry-run readiness projection, execution records, explicit executor-ready gate, pure patch model, isolated-root single-file UTF-8 apply-patch executor, explicit `tessera apply-patch` CLI envelope, trace-driven `tessera apply-patch --from-trace` envelope, opt-in `--auto-worktree` detached worktree lifecycle for trace-driven apply-patch, trace-backed `tessera worktree cleanup`, read-only `tessera worktree list --trace <trace_id> [--json]`, TUI workflow/worktree summaries, and GUI read-only metadata panels exist; no test runner, checkpoint restore, branch/stage/commit/push/PR Git mutation, GUI/TUI mutation workflow, app-server mutation listener, or broader worktree-first mutation lifecycle yet. |
| v0.8 | Swarm scheduler | [!] Blocked | Requires v0.6 structured handoff and reviewer gate. |
| v0.9 | Learning proposal system | [ ] Planned | No learning runtime; proposals only by design. |

## 5. Completed Capability Ledger

### v0.1 Runtime

- [x] Rust workspace established: `protocol`, `client`, `core`, `providers`, `storage`, `config`, `cli`, `tui`.
- [x] Strong typed IDs, runtime objects, `EventFrame`, `TraceRecord`, and provider-neutral `RunEvent`.
- [x] JSONL trace writer and rebuildable SQLite index.
- [x] Mock provider, OpenAI-compatible adapter, Ollama adapter, parser tests, opt-in live smoke tests.
- [x] Core `ConversationEngine`, provider/storage coordination, cancellation/timeout/backpressure, replay runner.
- [x] CLI doctor/chat/config/profiles/sessions/transcript/replay/events/interactive REPL.
- [x] TUI chat shell over shared `tessera-client` projection.
- [x] v0.1 release gates and manual testing docs.

### v0.2 Projection And GUI-Ready Work

- [x] Context workbench foundation.
- [x] Read-only runtime API through `RuntimeReader`.
- [x] Task registry v1 and client task projection.
- [x] Artifact handle and snapshot/checkpoint metadata projection.
- [x] GUI shell spike with `tessera-gui-bridge`.
- [x] Rust-to-TypeScript DTO generation and binding contract tests.
- [x] GUI mock/replay smoke tests.
- [x] Usage/cache/cost/context telemetry summaries.
- [x] Draft model router and no-progress loop detector.
- [x] Read-only skill registry schema.
- [x] Distribution plan.

### v0.3-v0.4 Foundations

- [x] Release identity metadata in `tessera --version`.
- [x] Tool descriptor and read-only tool registry.
- [x] Tool policy, approval, ordered result, and repair telemetry metadata.
- [x] Approval UI projection.
- [x] Workspace guardrail, sandbox decision, OS sandbox profile planner.
- [x] Workspace checkpoint planner with metadata URI only.
- [x] Metadata-only MCP adapter.
- [x] Runtime HTTP/SSE JSON/SSE shape helper without listening server.
- [x] Diagnostics event and reporter.
- [x] Memory proposal events and UI projection.

### v0.5 Foundation-Stable Work

- [x] Agent profile schema and read-only `AgentRegistry`.
- [x] Pause/resume protocol metadata and UI-neutral intents.
- [x] Cooperative pause signal in core.
- [x] Chat pause checkpoint envelope.
- [x] Runtime pause checkpoint projection.
- [x] CLI/TUI active pause wiring.
- [x] Chat-only `/resume-task <task_id|#>` execution.
- [x] Resume repeat guard and provider preflight.
- [x] `tessera tasks [--json]` and `tessera chat --resume-task <task_id|#>`.
- [x] Context handle projection through core/client/GUI bindings.
- [x] v0.5 single-agent loop design spec and implementation plan.
- [x] No-tool single-agent loop with `agent_run_*` / `agent_step_*` trace events, cancellation/pause/provider-failure/no-progress finish paths, and RuntimeReader task projection.
- [x] Script-friendly `tessera agent run --provider <id> --goal <text> [--json]` entrypoint.
- [x] v0.5 project instruction discovery design spec and implementation plan.
- [x] Project-local `AGENTS.md` / `CLAUDE.md` instruction discovery with source reporting, byte limits, symlink rejection, UTF-8 handling, redaction, `instructions_discovered` trace metadata, `tessera instructions inspect`, and opt-in `agent run --instructions`.
- [x] v0.5 Skill Runtime v1 design spec and implementation plan.
- [x] Explicit Skill Runtime v1 implementation with project-local `SKILL.md` discovery, strict flat frontmatter parsing, trace-safe `skill_activated` metadata, read-only references, `tessera skills inspect`, and opt-in `agent run --skill`.
- [x] v0.5 background task ownership design spec and implementation plan for trace-backed owner leases, heartbeat metadata, lost-owner projection, and explicit reattach outcomes.
- [x] Trace-backed background task ownership foundation: protocol IDs/events, core `TaskOwnershipRecorder`, `RuntimeReader::list_task_owners`, client owner projection, and `tessera tasks --owners --trace <trace_id>` read-only CLI output.
- [x] Automatic no-tool chat/agent run owner attach/detach events in `ConversationEngine` and `AgentLoop`, with terminal projection for completed runs and checkpoint-based reattach metadata for paused runs.
- [x] Runtime API / app-server alignment foundation: typed `RuntimeApi*` DTOs for protocol version, localhost/unix-socket bind metadata, auth policy metadata, bounded queue policy, read-only event command envelopes, event stream request mapping, bounded SSE frame buffer, and generated TypeScript schema evidence.

### v0.6 Foundation Work

- [x] Provider-neutral structured handoff and reviewer gate protocol events: `agent_handoff_recorded`, `reviewer_gate_requested`, and `reviewer_gate_resolved`.
- [x] Handoff/reviewer DTOs for handoff ids, reviewer gate ids, evidence refs, metrics, summaries, requests and decisions.
- [x] Read-only `tessera-client` projection for handoff summaries and reviewer gates from live events and replayed trace records.
- [x] Generated GUI TypeScript bindings for handoff/reviewer DTOs and trace event kinds.
- [x] Provider-neutral sub-agent session metadata events: `subagent_session_planned`, `subagent_session_started`, `subagent_session_waiting_for_approval`, `subagent_session_inactive`, and `subagent_session_completed`.
- [x] Sub-agent session DTOs for parent/child task linkage, transcript artifact handles, scope labels, caps, approval forwarding metadata and inactive-child policy.
- [x] Read-only `tessera-client` and GUI binding projection for sub-agent session metadata from live events and replayed trace records.
- [x] Persistent sub-agent scheduler/runtime ownership design gate: future runtime must route through core, task ownership, transcript artifact handles, policy-mediated approval forwarding, deterministic inactive-child handling, and explicit cancellation/reattach metadata before child execution.
- [x] Provider-neutral sub-agent runtime ownership event contract for scheduler decisions, transcript artifact publication, approval forwarding state, inactive-child policy handling, and cancellation cascade metadata.
- [x] Non-executing core `SubagentRuntimeCoordinator` skeleton that validates caps/reviewer/transcript metadata and returns runtime decisions without provider calls, tool execution, storage internals, or UI scheduling.
- [x] Read-only `tessera-client` and GUI binding projection for sub-agent runtime ownership metadata from live events and replayed trace records.
- [x] Provider-neutral sub-agent transcript artifact lifecycle metadata for reserved, published, sealed, and abandoned artifact handles.
- [x] Non-executing core transcript lifecycle helper that validates event ranges and emits lifecycle metadata without provider calls, tool execution, transcript body storage, storage internals, or UI scheduling.
- [x] Read-only `tessera-client` and GUI binding projection for transcript artifact lifecycle records, artifact handles, and reserved/published/sealed/abandoned status summaries.
- [x] Non-executing core approval-forwarding and inactive-policy helper that validates and emits metadata without automatic forwarding, child execution, provider calls, tool execution, storage internals, or UI scheduling.
- [x] Non-executing core cancellation cascade helper that validates linked source tasks and emits metadata without child runtime cancellation, provider socket freezing, provider calls, tool execution, storage internals, or UI scheduling.
- [x] Non-executing core sub-agent task owner bridge that maps child task ids to task owner attach/detach metadata without child scheduling, provider execution, heartbeat loops, lost-owner detection, storage internals, or UI scheduling.
- [x] Non-executing core sub-agent owner heartbeat/lost/reattach bridge that maps child task ids to task owner heartbeat, lost-owner and reattach metadata without heartbeat loops, lost-owner detection, automatic reattach, provider resume, storage internals, or UI scheduling.

### v0.7 Metadata And Runtime Gate Foundation Work

- [x] Provider-neutral coding workflow metadata events for workflow start, workspace mutation scope, patch proposals, patch application records, test plans/runs, test evidence summaries, review bundles, and restore plans.
- [x] Non-executing core `CodingWorkflowCoordinator` that validates scope/path/evidence/checkpoint/reviewer requirements and returns `RunEvent` values only.
- [x] Read-only `tessera-client` and GUI binding projection for coding workflow metadata from live events and replayed trace records.
- [x] Generated GUI TypeScript bindings for coding workflow projection and related DTOs, including snapshot IDs referenced by patch/checkpoint metadata.
- [x] Metadata-only test evidence summary contract for aggregating test plan/run artifact refs, diagnostics and redaction state without running tests or reading stdout/stderr bodies.
- [x] Mutation request proposal contracts with operation kind, requested paths, required checkpoint, reviewer gate, policy decision, sandbox profile and worktree requirement metadata.
- [x] Artifact body/redaction storage contracts and checkpoint lifecycle records without workspace restore or executor behavior.
- [x] Non-executing mutation enforcement planner for worktree-first/default-local policy gates, sandbox selection metadata and unsafe-path rejection.
- [x] Final v0.7 metadata-gate verification confirmed no apply-patch, shell/test, checkpoint restore, worktree creation, Git mutation, or tool-dispatch executor was added before the executor plan started.
- [x] Apply-patch preflight and dry-run readiness gate with metadata-only protocol/client/GUI projection and no file writes.
- [x] Apply-patch executor design and implementation plan for a core-owned, worktree-first, single-file text patch executor.
- [x] Provider-neutral apply-patch execution records that keep actual executor results distinct from dry-run preflight metadata and workflow summary projection.
- [x] Explicit executor-ready gate that preserves default `executor_blocked` behavior unless policy, reviewer, checkpoint, sandbox, executor context and isolated-root checks pass.
- [x] Pure in-memory single-file patch application model for UTF-8 create/modify operations, with conflict and unsupported-operation blockers before filesystem writes.
- [x] Isolated-root single-file apply-patch executor that writes through temp-file + rename only inside an explicit non-primary root and rejects unsafe paths or symlinks.
- [x] Read-only client and GUI binding projection for apply-patch execution records without adding CLI/TUI/GUI mutation controls.
- [x] Final narrow-executor verification confirms no shell/test execution, checkpoint restore, worktree creation/lifecycle, Git mutation, tool-dispatch executor, or GUI-owned mutation path was added.
- [x] Explicit `tessera apply-patch` CLI envelope over the narrow executor with operator-supplied refs, isolated root, allowed paths, patch source, preflight trace event, execution trace event, and no automatic worktree/test/checkpoint/Git/UI behavior.
- [x] Trace-driven `tessera apply-patch --from-trace` CLI envelope that resolves reviewed mutation metadata and clean patch artifacts from trace records before reusing the same gate/executor path, still requiring a caller-supplied non-primary isolated root.
- [x] Automatic isolated worktree lifecycle for trace-driven `tessera apply-patch --from-trace --auto-worktree`, with detached worktree creation from source `HEAD`, path-redacted lifecycle trace records, dry-run without worktree creation, and retained worktree output for operator inspection.
- [x] Conservative `tessera worktree cleanup` command for retained Tessera-generated detached worktrees, requiring trace lifecycle evidence and an explicit local path while forbidding force removal and broader Git/test/checkpoint execution.
- [x] Read-only worktree lifecycle projection in `RuntimeReader` and `tessera-client`, including generated GUI bindings, status summaries and replay support from `workspace_worktree_lifecycle_recorded` records.
- [x] Read-only `tessera worktree list --trace <trace_id> [--json]` over lifecycle trace metadata, including conservative `trace_cleanup_candidate` evidence labels and improved cleanup diagnostics without filesystem paths.
- [x] TUI status line summaries and GUI read-only Coding Workflows / Worktrees metadata panels over safe aggregate `ClientSnapshot` fields only, with no paths, touched paths, reason text, summaries, diagnostics, artifact bodies or mutation controls.

## 6. Current Gaps

### Needs Verification, Not New Architecture

- [x] Ollama real streaming path: opt-in smoke test passed locally on 2026-05-25 with `TESSERA_OLLAMA_MODEL=qwen3-vl:4b`; the run took 101.10s, so it remains opt-in rather than part of the default gate.
- [~] GUI live event bridge: core/CLI/TUI share `EventFrame`; GUI shell still uses mock/replay projection and typed bridge foundation.

### Deferred Beyond v0.5

- [ ] Executable skills, automatic/default/global skill loading, skill install/update/delete, model-driven reference selection, and script/tool skill execution: future skill/tool policy gate.
- [ ] Default-on/global/user instruction loading, Claude imports, and `.claude/` rule compatibility: future precedence/source-reporting/byte-limit/redaction/trace-reference gate.
- [ ] Background reattach / runtime ownership transfer after process restart: future durable owner-process and app-server/listener gate.
- [ ] Daemon or app-server listener for durable task observation/control: future listener gate; current work is DTO/schema/read-only shape only.
- [ ] Non-chat task resume: future structured task handoff/reviewer gate.
- [ ] Real checkpoint restore semantics: future workspace mutation/checkpoint restore/revert/policy/sandbox/trace gate.

### v0.6-v0.9 Planned Or Blocked

- [x] Provider-neutral sub-agent runtime ownership events for scheduler decisions, transcript artifacts, approval forwarding, inactive policy, and cancellation.
- [x] Core `SubagentRuntimeCoordinator` skeleton without provider calls or child execution.
- [x] Read-only client and GUI projection for sub-agent runtime ownership metadata.
- [x] Provider-neutral transcript artifact lifecycle metadata and read-only projection are replayable.
- [x] Core approval-forwarding and inactive-policy metadata helper without automatic forwarding or inactive-child execution.
- [x] Core cancellation cascade metadata helper without child runtime cancellation or provider socket freezing.
- [x] Core sub-agent task owner attach/detach metadata bridge without child runtime scheduling or heartbeat loops.
- [x] Core sub-agent owner heartbeat/lost/reattach metadata bridge without heartbeat loops, lost-owner detection or provider resume.
- [ ] Persistent sub-agent runtime scheduling after coordinator skeleton, ownership/lifecycle/policy/cancellation events, task ownership persistence, approval forwarding and cancellation gates are executable.
- [ ] Transcript body storage, retention, summarization and parent-context ingestion for real child-agent runs.
- [ ] Automatic approval forwarding and inactive-child execution over reviewer-gated session metadata.
- [ ] Hook runtime.
- [ ] Automation runtime.
- [x] Metadata-only coding-agent workflow contract and read-only projection for diff/test/checkpoint/review/restore evidence.
- [x] v0.7 runtime gate assessment documenting why apply-patch, test execution, checkpoint restore, worktree mutation, Git mutation, hooks, automations, app-server mutation listener, swarm and learning apply remain blocked.
- [x] v0.7 runtime gate closure foundations for mutation request proposals, artifact body/redaction contracts, checkpoint lifecycle contracts, and worktree/sandbox enforcement planning before any executor.
- [x] v0.7 apply-patch executor gate design defining preflight and dry-run readiness as the next safe slice while keeping workspace writes blocked.
- [x] Apply-patch preflight and dry-run readiness gate without file writes.
- [x] v0.7 apply-patch executor design defining the core-owned, worktree-first, single-file text patch execution boundary.
- [x] Provider-neutral apply-patch execution records and read-only client/GUI projection for actual executor result metadata.
- [x] Explicit executor-ready gate while default preflight/dry-run records remain `executor_blocked`.
- [x] Pure in-memory single-file patch model and isolated-root UTF-8 create/modify executor with path/symlink guardrails.
- [x] v0.7 explicit apply-patch CLI tool design for a gated, operator-supplied isolated-root envelope over the narrow executor.
- [x] User-facing explicit apply-patch CLI tool over the narrow executor.
- [x] v0.7 trace-driven apply-patch workflow automation design for resolving reviewed mutation bundles from trace and patch artifacts.
- [x] Trace-driven apply-patch workflow automation through `tessera apply-patch --from-trace`, with trace-backed selector resolution and artifact body loading before shared gate/executor execution.
- [x] v0.7 automatic isolated worktree lifecycle design for future trace-driven apply-patch `--auto-worktree` mode.
- [x] Trace-driven apply-patch automatic isolated worktree lifecycle through `tessera apply-patch --from-trace --auto-worktree`, with detached generated worktree creation only and no test/checkpoint/Git mutation expansion.
- [x] Automatic worktree cleanup command for retained Tessera-generated detached worktrees.
- [x] Read-only worktree lifecycle list/projection and workflow/worktree UI metadata observation surfaces.
- [ ] Executable coding-agent diff/test/checkpoint/rollback workflow.
- [ ] Broader worktree-first mutation lifecycle.
- [~] GUI/TUI coding workflow visibility: compact status and safe read-only metadata panels exist; diff/review/approval/artifact inspection and mutation controls remain planned.
- [ ] Swarm scheduler.
- [ ] Learning proposal system.

## 7. Recommended Execution Order

The next implementation slices should stay conservative and preserve the current foundation-first pattern.

1. [x] If local time permits, run Ollama live smoke; otherwise keep it documented as unverified.
2. [x] Design v0.5 single-agent loop as a separate spec before code: provider-neutral observations, max-step limits, stop/no-progress handling, trace replay, run summary and machine-readable result.
3. [x] Design project instruction discovery with `AGENTS.md` first and future `CLAUDE.md` compatibility: precedence, byte limits, source report, redaction and trace refs.
4. [x] Implement single-agent loop without tools first.
5. [x] Implement opt-in project instruction discovery and source reporting before skill runtime.
6. [x] Design skill runtime v1 after single-agent loop is trace-stable: `SKILL.md` discovery, activation events, read-only references, no unchecked scripts.
7. [x] Implement Skill Runtime v1 foundation from `docs/superpowers/plans/2026-05-20-v0.5-skill-runtime-v1.md`.
8. [x] Design durable background task ownership and reattach before any long-running agent task.
9. [x] Implement trace-backed background task ownership foundation from `docs/superpowers/plans/2026-05-20-v0.5-background-task-ownership-v1.md`.
10. [x] Decide and implement automatic no-tool run owner attach/detach as a separate behavior-changing slice for current `AgentLoop` and `ConversationEngine` traces.
11. [x] Align runtime API / app-server shape before GUI live-provider path: auth, bounded queues, generated schemas, localhost default, no duplicate runtime.
12. [x] After v0.5 is foundation-stable, design v0.6 structured handoff and reviewer gate.
13. [x] Implement provider-neutral handoff/reviewer protocol events and read-only projection before any persistent sub-agent runtime.
14. [x] Design persistent sub-agent session foundation over existing handoff/reviewer events: parent/child task linkage, transcript artifacts, caps, approval forwarding and inactive-child handling.
15. [x] Implement provider-neutral sub-agent session metadata events and read-only projection before any persistent scheduler.
16. [x] Design persistent sub-agent scheduler/runtime ownership over session metadata: parent/child lifecycle, transcript artifact lifecycle, approval forwarding runtime, inactive-child handling, and cancellation/reattach boundaries.
17. [x] Add provider-neutral sub-agent runtime ownership event contract before scheduler implementation.
18. [x] Add a non-executing core `SubagentRuntimeCoordinator` skeleton that validates caps/policy and records decisions without provider calls.
19. [x] Add read-only client and GUI projection for runtime ownership metadata.
20. [x] Add provider-neutral transcript artifact lifecycle metadata, non-executing core validation helper, and read-only client/GUI projection.
21. [x] Add non-executing core approval-forwarding and inactive-policy metadata helper.
22. [x] Add non-executing core cancellation cascade metadata helper.
23. [x] Add non-executing core sub-agent task owner bridge metadata helper.
24. [x] Add non-executing core sub-agent owner heartbeat/lost/reattach metadata helper.
25. [x] Design v0.7 coding-agent workflow foundation: patch proposal, diff/test evidence, checkpoint requirement, review bundle, restore plan and worktree-first mutation boundaries without execution.
26. [x] Implement metadata-only v0.7 coding-agent workflow protocol, core coordinator, client projection and GUI bindings before any apply-patch execution.
27. [x] Audit v0.7 runtime gate readiness and document missing execution prerequisites before any apply-patch runtime.
28. [x] Execute the v0.7 runtime gate closure plan for proposal contracts, artifact/redaction contracts, checkpoint lifecycle contracts, and worktree/sandbox enforcement planning while keeping mutation execution blocked.
29. [x] Design the v0.7 apply-patch executor gate as a non-mutating preflight and dry-run slice before any file writes.
30. [x] Implement apply-patch preflight and dry-run readiness contracts without file writes, worktree creation, shell/test execution, checkpoint restore or Git mutation.
31. [x] Design the v0.7 apply-patch executor implementation boundary: core-owned orchestration, explicit executor-ready preflight, isolated non-primary mutation root, first runtime limited to one UTF-8 text file create/modify operation, and no shell/test/Git/checkpoint-restore/UI-owned execution.
32. [x] Add provider-neutral apply-patch execution records before runtime writes so actual execution is distinct from metadata-only patch application summaries.
33. [x] Gate executor readiness explicitly while preserving the default `executor_blocked` behavior for preflight/dry-run records.
34. [x] Add a pure in-memory single-file patch application model before filesystem writes.
35. [x] Add isolated-root single-file apply-patch execution only after protocol, readiness, policy, sandbox, checkpoint, reviewer and path/symlink checks are verified.
36. [x] Design the explicit `tessera apply-patch` CLI envelope over the narrow executor: operator-supplied isolated root, allowed paths, gate refs, patch source, preflight trace event, execution trace event, and no automatic worktree/test/checkpoint/Git/UI behavior.
37. [x] Implement the explicit `tessera apply-patch` CLI envelope before trace-driven workflow automation.
38. [x] Design trace-driven apply-patch workflow automation only after explicit CLI semantics and trace append behavior are verified.
39. [x] Implement trace-driven apply-patch workflow automation from trace-backed workflow, approval and patch artifact records, reusing the existing explicit apply-patch gate/executor path.
40. [x] Design automatic isolated worktree lifecycle for trace-driven apply-patch before adding any worktree creation, cleanup, test execution, checkpoint restore, or Git mutation behavior.
41. [x] Implement automatic isolated worktree lifecycle for trace-driven apply-patch with detached worktree creation only, no branch/stage/commit/push/PR mutation, no tests, and no checkpoint restore.
42. [x] Design and implement a conservative worktree cleanup command before adding broad cleanup, test execution, checkpoint restore, or Git workflow completion.
43. [x] Implement read-only worktree lifecycle projection/listing and workflow/worktree UI metadata surfaces without mutation controls or sensitive path/evidence fields.
44. [!] Do not start hook or automation runtime until tool/policy/sandbox/checkpoint/task ownership gates exist.
45. [!] Do not start test runner, checkpoint restore, branch/stage/commit/push/PR Git mutation, GUI mutation controls, app-server mutation listener, swarm, or learning apply until their stated gates exist.
46. [x] Add a metadata-only test evidence summary contract before executable test runner integration.

## 8. Mandatory Gates

- [!] No provider behavior expansion without replay fixtures.
- [!] No file mutation tools without policy gate, sandbox decision, and checkpoint semantics.
- [!] No shell/file/git runtime without approval UI and trace events.
- [!] No default-on/global/user instruction loading, Claude import expansion, or `.claude/` rule compatibility without precedence rules, byte limits, source reporting, redaction and trace refs.
- [!] No hook runtime before tool/policy/sandbox/checkpoint exist.
- [!] No automation runtime before task ownership, workspace isolation, setup verification, logs, notifications and failure reporting exist.
- [!] No app-server listener before auth, bounded queues, generated schema and localhost-default binding exist.
- [!] No GUI Git mutation before diff, checkpoint, policy and trace semantics exist.
- [!] No Auto router execution without usage/cache/cost telemetry and user-visible route reasons.
- [!] No automatic route escalation without no-progress loop detection.
- [!] No subagent fan-out before explicit caps, parent/child trace linkage, transcript artifact handles and reviewer gate.
- [!] No swarm before structured handoff, reviewer gate, cost budget and deterministic result publication.
- [!] No long-term memory runtime before scope schema, proposal review, and trace-backed apply/reject records.
- [!] No learning apply path before proposal review and replay/eval evidence.
- [!] No MCP environment forwarding without explicit env allowlist and secret redaction.
- [!] No web search or computer-use default-on behavior without policy, source attribution and replay-safe event records.

## 9. Update Protocol

Every stage change must update the right layer:

- Update `docs/version-plan.md` when a version scope, dependency, exit criterion, or cross-version gate changes.
- Update `docs/coding-agent-direction.md` when external coding-agent product direction changes or a reference target is added/removed.
- Update `docs/global-plan.md` when an item is completed, added, removed, deferred, or re-scoped.
- Update `CHANGELOG.md` for user-visible changes.
- Update `docs/crate-boundaries.md` when crate responsibilities or dependency rules change.
- Update `docs/protocol-v0.md` and `docs/trace-schema-v0.md` when protocol or trace events change.
- Update release checklists only for release/tag process changes.
- Run the documented verification gate before claiming completion.

## 10. Verification Gate

For documentation-only changes:

```bash
git diff --check
```

For implementation changes:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check
PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings
PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace
git diff --check
```
