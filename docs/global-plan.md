# Tessera Global Plan

日期：2026-05-22

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

- **用户可用能力**：CLI/TUI/mock chat、trace/replay、session resume、chat-only paused task resume、opt-in project instructions、explicit read-only skill activation 等。
- **foundation 能力**：tool schema、policy gate draft、sandbox planner、MCP metadata adapter、runtime HTTP/SSE shape、runtime API / app-server DTO alignment、diagnostics event、memory proposal UI、trace-backed task ownership metadata/recorder/projection、no-tool chat/agent run owner attach/detach、structured handoff/reviewer gate、sub-agent session metadata/projection 等。
- **尚未支持能力**：真实 tool execution、executable/default/global skill runtime、tool-using/full coding-agent runtime、MCP runtime、background reattach、daemon/app-server listener、default-on/global project instruction ingestion、hook runtime、automation runtime、workspace restore、persistent sub-agent runtime、swarm、learning apply。

## 4. Version Status Matrix

| Version | Theme | Status | Current Evidence |
| --- | --- | --- | --- |
| v0.1 | Trace-first local runtime | [x] Released | `v0.1.0-alpha.1` and `v0.1.0` tags exist; release checklist complete. |
| v0.2 | Read-only projection and GUI-ready surfaces | [x] Done | RuntimeReader, task/artifact/snapshot projection, GUI shell spike, DTO bindings, distribution plan. |
| v0.3 | Tool policy and sandbox foundations | [~] Foundation complete | Tool descriptor, policy gate, approval projection, sandbox decision, OS sandbox planner, checkpoint planner; no execution. |
| v0.4 | Runtime API, MCP, diagnostics, memory foundations | [~] Foundation complete | RuntimeHttpApi shape, metadata-only MCP adapter, diagnostics event, memory proposal UI; no server/runtime/store. |
| v0.5 | Single-agent and resumable task foundations | [~] Foundation stable | Agent profile registry, context handles, cooperative pause, chat-only task resume, top-level paused task CLI, no-tool `AgentLoop`, `tessera agent run`, opt-in project instruction discovery/source reporting, explicit read-only Skill Runtime v1, trace-backed task ownership foundation, automatic no-tool chat/agent owner attach/detach, and runtime API / app-server DTO alignment; runtime-complete gaps are staged into future gates. |
| v0.6 | Persistent sub-agents and structured review | [~] Foundation complete | Structured handoff/reviewer gate and sub-agent session metadata protocol events, read-only client projection, and GUI bindings exist; persistent sub-agent runtime is not implemented yet. |
| v0.7 | Project coding-agent workflow | [ ] Planned | No apply-patch, diff/test/checkpoint/rollback, worktree mutation, or GUI/TUI diff workflow yet. |
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

## 6. Current Gaps

### Needs Verification, Not New Architecture

- [~] Ollama real streaming path: code and opt-in smoke test exist; live local verification is still missing.
- [~] GUI live event bridge: core/CLI/TUI share `EventFrame`; GUI shell still uses mock/replay projection and typed bridge foundation.

### Deferred Beyond v0.5

- [ ] Executable skills, automatic/default/global skill loading, skill install/update/delete, model-driven reference selection, and script/tool skill execution: future skill/tool policy gate.
- [ ] Default-on/global/user instruction loading, Claude imports, and `.claude/` rule compatibility: future precedence/source-reporting/byte-limit/redaction/trace-reference gate.
- [ ] Background reattach / runtime ownership transfer after process restart: future durable owner-process and app-server/listener gate.
- [ ] Daemon or app-server listener for durable task observation/control: future listener gate; current work is DTO/schema/read-only shape only.
- [ ] Non-chat task resume: future structured task handoff/reviewer gate.
- [ ] Real checkpoint restore semantics: future workspace mutation/checkpoint restore/revert/policy/sandbox/trace gate.

### v0.6-v0.9 Planned Or Blocked

- [ ] Provider-neutral sub-agent runtime ownership events for scheduler decisions, transcript artifacts, approval forwarding, inactive policy, and cancellation.
- [ ] Core `SubagentRuntimeCoordinator` skeleton without provider calls or child execution.
- [ ] Persistent sub-agent runtime scheduling after coordinator skeleton, ownership events, and transcript artifact lifecycle are replayable.
- [ ] Runtime transcript artifact lifecycle for real child-agent runs.
- [ ] Runtime approval forwarding and inactive-child handling over reviewer-gated session metadata.
- [ ] Hook runtime.
- [ ] Automation runtime.
- [ ] Coding-agent diff/test/checkpoint/rollback workflow.
- [ ] Apply-patch tool.
- [ ] Worktree-first mutation mode.
- [ ] GUI/TUI diff and review surfaces.
- [ ] Swarm scheduler.
- [ ] Learning proposal system.

## 7. Recommended Execution Order

The next implementation slices should stay conservative and preserve the current foundation-first pattern.

1. [ ] If local time permits, run Ollama live smoke; otherwise keep it documented as unverified.
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
17. [ ] Add provider-neutral sub-agent runtime ownership event contract before scheduler implementation.
18. [ ] Add a non-executing core `SubagentRuntimeCoordinator` skeleton that validates caps/policy and records decisions without provider calls.
19. [ ] Add read-only client and GUI projection for runtime ownership metadata.
20. [!] Do not start hook or automation runtime until tool/policy/sandbox/checkpoint/task ownership gates exist.
21. [!] Do not start apply-patch/file mutation workflow until policy, sandbox, checkpoint, single-agent loop, background ownership, and reviewer gate are all stable.
22. [!] Do not start swarm until structured handoff, reviewer gate, cost budgets and deterministic result publication exist.
23. [!] Do not start learning apply path until skill runtime, policy, review, and replay/eval evidence exist.

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
