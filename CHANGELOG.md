# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added

- Added an initial read-only runtime API in `tessera-core` for trace event pagination with `since_seq` / `limit` and indexed thread/turn/item/task/artifact ID queries without exposing storage internals or adding an HTTP/SSE server.
- Added official Reasonix lessons covering cache-stable context, ordered parallel tool dispatch, tool-call repair telemetry, visible cost control, and no-progress loop policy without changing Tessera's model-agnostic scope.
- Added UI-neutral cache/cost status projection in `tessera-client` from live `UsageReported` events and replayed `usage_reported` trace records.
- Added UI-neutral usage/cache/cost/context telemetry summaries in `tessera-client` from standard live events and replayed trace records, with TUI status rendering kept as a view-only consumer.
- Added Task registry v1 with read-only runtime task summaries in `tessera-core` and UI-neutral `ClientTask` projection in `tessera-client` from live events and replayed trace records.
- Added artifact handle projection with read-only runtime artifact summaries in `tessera-core` and UI-neutral `ClientArtifact` projection in `tessera-client` from `artifact_created` events and `artifact_refs`.
- Added a draft `ModelRouter` in `tessera-core` that records manual/default route decisions with explicit route reasons while keeping auto routing disabled.
- Added a draft no-progress loop detector and provider-neutral `no_progress_loop_detected` event so no-output, repeated read-only, and repeated repair loops stop/ask/summarize before any future route escalation.
- Added a read-only skill registry schema with `SKILL.md`-compatible manifest metadata in `tessera-protocol` and a non-executing `SkillRegistry` in `tessera-core`.
- Added workspace checkpoint schema with provider-neutral `snapshot_created` trace events and read-only `RuntimeReader::list_snapshots` projection, without restore/revert execution.
- Added an initial context workbench schema and pure in-memory budget projection for stable prefix, append-only transcript, and volatile scratch references without loading file contents.
- Added UI-neutral context handle projection with `ContextWorkbench::projection`, client context handle DTOs and status summary, and generated GUI TypeScript bindings without reading source content, building prompts, or writing context trace events.
- Added TUI rendering for projected context handle summaries and GUI TypeScript fixture/build coverage for generated context handle DTOs.
- Added the first Tauri GUI shell spike with a tested `tessera-gui-bridge`, typed mock/replay commands, bounded GUI event backpressure, and a React/Vite shell that renders shared `ClientSnapshot` projection without provider or storage access.
- Added `tessera-gui-bindings` to generate GUI TypeScript DTOs from Rust `protocol` / `client` / `gui-bridge` types, plus a contract test that keeps `apps/gui-tauri/src/generated/bindings.ts` in sync.
- Added a deterministic GUI smoke test for the Tauri shell covering mock/replay load, prompt submission, cancellation, new-thread reset, and toolbar action accessibility names.
- Added a v0.2 distribution plan covering GitHub Releases, Cargo, Homebrew, npm wrapper, Docker, checksums, publish ordering, mirror knobs, and v0.3+ acceptance gates.
- Added a roadmap governance refresh with `docs/version-plan.md` as the v0.1-v0.9 source of truth and `docs/global-plan.md` as the current progress dashboard.
- Added `docs/coding-agent-direction.md` to align DeepSeek-TUI, Reasonix, Codex CLI/App/App Server, and Claude Code CLI/Desktop/Web lessons with Tessera's version gates, GUI direction, hooks, automations, skills, subagents, and app-server boundaries.
- Added the v0.5 no-tool single-agent loop design spec and implementation plan covering agent run envelopes, step events, trace summaries, CLI entrypoint scope, and verification gates.
- Added the v0.5 project instruction discovery design spec and implementation plan covering `AGENTS.md` priority, `CLAUDE.md` fallback compatibility, source reporting, byte limits, redaction, trace metadata, CLI inspection, and opt-in agent context.
- Added the v0.5 Skill Runtime v1 design spec and implementation plan covering `SKILL.md` discovery, explicit activation, read-only references, trace-safe activation metadata, and no unchecked script/tool execution.
- Added the v0.5 no-tool single-agent loop in `tessera-core` plus `tessera agent run --provider <id> --goal <text> [--json]`, recording provider-neutral agent run/step trace events without tool execution, file mutation, or background reattach.
- Added opt-in project instruction discovery with `tessera instructions inspect --workspace <path>` and `tessera agent run --instructions --workspace <path>`, recording `instructions_discovered` source metadata while keeping instruction text out of trace.
- Added explicit Skill Runtime v1 foundations with `tessera skills inspect`, opt-in `agent run --skill`, trace-safe `skill_activated` metadata, and read-only `SKILL.md` context loading without executing scripts or tools.
- Added the v0.5 background task ownership design spec and implementation plan covering trace-backed owner leases, heartbeat metadata, lost-owner projection, and explicit reattach outcomes without adding a daemon, provider socket freezing, or tool execution.
- Added trace-backed task ownership metadata and read-only owner projection foundations with `TaskOwnershipRecorder`, `RuntimeReader::list_task_owners`, client owner status projection, and `tessera tasks --owners --trace <trace_id>` without adding a daemon, provider socket freezing, or tool execution.
- Added automatic task owner attach/detach events around no-tool chat and agent runs, including terminal owner projection for completed runs and checkpoint-based reattach metadata for paused runs.
- Added v0.5 runtime API / app-server alignment DTOs for localhost-safe bind metadata, auth policy, bounded queues, command envelopes, event stream requests, generated TypeScript schemas, and a core bounded event buffer without starting a listener or executing providers through an app-server.
- Added v0.5 foundation-stable roadmap closure, explicitly staging executable skills, default/global instruction loading, durable background reattach, app-server listeners, non-chat task resume, and checkpoint restore into future version gates.
- Added the v0.6 structured handoff and reviewer gate foundation design, covering handoff summaries, bounded evidence refs, reviewer requests, reviewer decisions, and trace/client projection boundaries before persistent sub-agent runtime.
- Added v0.6 provider-neutral `agent_handoff_recorded`, `reviewer_gate_requested`, and `reviewer_gate_resolved` protocol events with bounded evidence references and reviewer decision metadata, without starting persistent sub-agents or tool execution.
- Added read-only `tessera-client` and GUI binding projection for v0.6 handoff summaries and reviewer gates from live events and replayed trace records.
- Added the v0.6 persistent sub-agent session foundation design, covering parent/child task linkage, transcript artifact handles, caps, approval forwarding metadata, and inactive-child handling before any scheduler or fan-out runtime.
- Added v0.6 provider-neutral sub-agent session metadata events for planned, started, waiting-for-approval, inactive and completed session states without starting a scheduler or child runtime.
- Added read-only `tessera-client` and GUI binding projection for v0.6 sub-agent session metadata from live events and replayed trace records.
- Added the v0.6 persistent sub-agent scheduler/runtime ownership design gate, requiring core-owned scheduler decisions, task ownership, transcript artifact lifecycle, policy-mediated approval forwarding, inactive-child policy records, and cancellation/reattach metadata before child execution.
- Added v0.6 provider-neutral sub-agent runtime ownership events for scheduler decisions, transcript artifact publication, approval forwarding state, inactive-child policy handling, and cancellation cascade metadata without starting child execution.
- Added v0.6 provider-neutral sub-agent transcript artifact lifecycle metadata for reserved, published, sealed, and abandoned artifact handles without storing transcript bodies or starting child execution.
- Added a non-executing core `SubagentRuntimeCoordinator` skeleton that validates caps/reviewer/transcript metadata and returns provider-neutral runtime decisions without calling providers, tools, storage internals, or UI.
- Added a non-executing core helper for validating and emitting sub-agent transcript artifact lifecycle records without storing transcript bodies or invoking providers, tools, storage, or UI.
- Added a non-executing core helper for validating and emitting sub-agent approval-forwarding and inactive-policy records without automatic forwarding, child scheduling, provider calls, tools, storage, or UI.
- Added a non-executing core helper for validating and emitting sub-agent cancellation cascade records without cancelling child runtime, calling providers, executing tools, writing storage, or driving UI.
- Added a non-executing core helper for bridging sub-agent child task ids to task owner attach/detach metadata without starting child scheduling, provider execution, heartbeat loops, storage writes, tools, or UI.
- Added a non-executing core helper for bridging sub-agent child task ids to task owner heartbeat, lost-owner, and reattach metadata without heartbeat loops, lost-owner detection, provider resume, storage writes, tools, or UI.
- Added read-only `tessera-client` and GUI binding projection for v0.6 sub-agent transcript artifact lifecycle records, including artifact handles and reserved/published/sealed/abandoned status summaries.
- Added read-only `tessera-client` and GUI binding projection for v0.6 sub-agent runtime ownership metadata from live events and replayed trace records.
- Added a metadata-only v0.7 coding-agent workflow foundation with provider-neutral protocol events, a non-executing core coordinator, and read-only client/GUI projection for workspace mutation scope, patch proposals, patch application records, test plans/runs, review bundles, and restore plans without apply-patch, file mutation, shell/test execution, checkpoint restore, or Git mutation.
- Added a metadata-only v0.7 test evidence summary contract with protocol/core/client/GUI projection for aggregating test plan/run artifact refs and diagnostics without running tests or reading stdout/stderr bodies.
- Added the v0.7 apply-patch executor gate design and implementation plan for non-mutating preflight and dry-run readiness before any file-writing executor.
- Added a non-mutating apply-patch preflight/dry-run gate with metadata-only protocol events, read-only client/GUI projection, affected-path and operation summaries, blocker labels, and an explicit executor-blocked reason without file writes, shell/test execution, checkpoint restore, worktree creation, or Git mutation.
- Added the v0.7 apply-patch executor design and implementation plan for a future core-owned, worktree-first, single-file text patch executor while keeping runtime execution unimplemented in the design commit.
- Added the first narrow v0.7 apply-patch executor slice with provider-neutral execution records, explicit executor-ready gating, a pure in-memory single-file patch model, isolated-root UTF-8 create/modify file writes, and read-only client/GUI projection, while keeping shell/test execution, checkpoint restore, worktree creation, Git mutation, and GUI/TUI mutation controls blocked.
- Added the v0.7 explicit apply-patch CLI tool design and implementation plan for a gated isolated-root command over the narrow executor, while keeping worktree lifecycle, test execution, checkpoint restore, Git mutation, and GUI/TUI mutation controls blocked.
- Added an explicit `tessera apply-patch` CLI command that runs the core apply-patch gate and isolated single-file executor with trace-backed preflight/execution metadata, while keeping worktree creation, tests, checkpoint restore, Git mutation, and GUI/TUI mutation controls blocked.
- Added the v0.7 trace-driven apply-patch workflow automation design and implementation plan for resolving reviewed mutation bundles from trace and patch artifacts, while keeping worktree lifecycle, test execution, checkpoint restore, Git mutation, and GUI/TUI mutation controls blocked.
- Added trace-driven `tessera apply-patch --from-trace` mode that resolves reviewed workflow, scope, mutation request, patch proposal, checkpoint, reviewer, policy, sandbox, and clean patch artifact metadata from trace records before reusing the isolated apply-patch gate/executor, while still requiring a caller-supplied non-primary isolated root and keeping worktree lifecycle, test execution, checkpoint restore, Git mutation, and GUI/TUI mutation controls blocked.
- Added the v0.7 automatic isolated worktree lifecycle design and implementation plan for future trace-driven apply-patch `--auto-worktree` mode, while keeping actual worktree creation, test execution, checkpoint restore, branch/stage/commit/push/PR Git mutation, and GUI/TUI mutation controls blocked in this docs slice.
- Added opt-in trace-driven `tessera apply-patch --from-trace --auto-worktree` mode that creates a detached generated worktree from the source checkout, applies the reviewed patch there, records path-redacted worktree lifecycle metadata, and returns the local worktree path in CLI output while keeping test execution, checkpoint restore, branch/stage/commit/push/PR Git mutation, and GUI/TUI mutation controls blocked.
- Added `tessera worktree cleanup`, a conservative trace-backed command for removing retained Tessera-generated detached worktrees with an explicit local path, while keeping force removal, tests, checkpoint restore, branch/stage/commit/push/PR Git mutation, and GUI/TUI mutation controls blocked.
- Added read-only workflow/worktree visibility through worktree lifecycle projection, `tessera worktree list --trace <trace_id> [--json]`, TUI status summaries, and GUI metadata panels with safe aggregate fields only, while keeping executable diff/test/checkpoint/Git workflows and GUI mutation controls blocked.
- Added read-only diff/review/approval workflow inspection through safe `ClientWorkflowInspection` projection rows, `tessera workflow inspect --trace <trace_id> [--json]`, a TUI `workflow_inspection_summary` status-line field, and a GUI Review Inspection metadata panel over generated refs, counts, booleans, and status labels only, while keeping artifact body loading, patch rendering, test execution, checkpoint restore, Git mutation, provider/tool execution, app-server mutation listeners, and GUI/TUI mutation controls blocked.
- Added release identity metadata so `tessera --version` reports both the crate version and build git SHA.
- Added a provider-neutral `ToolDescriptor` schema and read-only `ToolRegistry` for tool metadata, with `parallel_safe` defaulting to false and no tool execution path.
- Added tool call request, policy decision, and approval trace metadata plus a draft `PolicyGate` that produces `allow` / `ask_user` / `deny` decisions without executing tools.
- Added tool dispatch/result trace metadata plus an `OrderedToolResultBuffer` that releases out-of-order completions in declared order without executing tools.
- Added tool-call repair telemetry metadata plus a `ToolRepairTelemetry` helper for provider-neutral flatten/scavenge/truncation/storm summaries without raw provider reasoning.
- Added approval UI projection with pending/resolved approval state, `/approve` and `/deny` client intents, and TUI approval status without executing tools.
- Added workspace guardrail and sandbox decision metadata plus a draft `WorkspaceGuardrailChecker` that records lexical path decisions without file IO, tool execution, or an OS sandbox.
- Added OS sandbox profile metadata plus an `OsSandboxPlanner` that maps tool descriptors to read-only, workspace-write, network-required, or denied profiles without starting a sandbox, opening network access, or executing tools.
- Added a `WorkspaceCheckpointPlanner` that creates checkpoint metadata only for sandbox profiles requiring checkpoints, without creating side-git state, touching files, or implementing restore/revert.
- Added a metadata-only `McpToolAdapter` that converts MCP tool specs and arguments into Tessera `ToolDescriptor` and `ToolCallRequest` values while treating MCP annotations as untrusted hints and avoiding MCP server execution.
- Added a read-only `RuntimeHttpApi` foundation that wraps `RuntimeReader` event pages as JSON and SSE frames without starting an HTTP server, binding ports, or owning runtime execution.
- Added provider-neutral diagnostics metadata with `diagnostics_reported` events and a `DiagnosticsReporter` helper for LSP-style ranges without starting LSP servers, compilers, or file reads.
- Added memory proposal UI foundations with provider-neutral memory proposal events, client pending/applied/rejected projection, `/remember` and `/forget` intents, TUI status rendering, and typed GUI bridge handling without long-term memory writes.
- Added provider-neutral `AgentProfile` metadata and a read-only `AgentRegistry` foundation without implementing an agent runtime, tool execution, or skill activation.
- Added pause/resume foundation metadata with provider-neutral `task_paused` / `task_resumed` events, shared `/pause` and `/resume-task` client intents, TUI pass-through, GUI metadata-only notices, and TypeScript bindings without implementing suspended provider execution.
- Added CLI discovery and parsing for `/pause [task_id]` and `/resume-task <task_id>` without provider stream suspension; chat-only task resume execution is now layered on the checkpoint path.
- Added a suspended/background resume design that chooses cooperative pause checkpoints and resume envelopes over provider socket freezing.
- Added `RuntimeReader::list_tasks` projection for `task_paused` / `task_resumed` trace records so read-only runtime APIs can surface paused and resumed task state without implementing provider suspension.
- Added a core-owned `RunPauseToken` control path that cooperatively records `task_paused` and ends the current trace without treating pause as cancellation, while leaving resume envelopes and provider suspension unimplemented.
- Added active `/pause` wiring in the CLI REPL and TUI runtime handler path so active chat runs can request the core pause token; TUI `/resume-task` remains metadata-only while CLI chat resume uses trace checkpoints.
- Added provider-neutral chat pause checkpoint metadata with `task_pause_checkpoint_created` trace events before `task_paused`, using `from_trace_projection` resume mode without implementing resume execution.
- Added read-only `RuntimeReader::list_pause_checkpoints` projection for the latest pause checkpoint per task, without implementing `/resume-task` execution or provider suspension.
- Added chat-only CLI `/resume-task <task_id>` execution from pause checkpoints, using trace projection to start a new core chat run and append `task_resumed` without background reattach or workspace restore.
- Added a CLI `/resume-task` guard that rejects repeat resume attempts once the original task is no longer paused, preventing duplicate chat resume runs from a stale checkpoint.
- Added a CLI `/resume-task` provider-profile preflight so missing checkpoint providers fail before trace projection mutates the visible session.
- Added CLI `/resume-task` missing-checkpoint coverage and a read-only `/resume-tasks` list for currently resumable paused trace checkpoints.
- Added `/resume-task <number|#number>` support so CLI users can resume from the `/resume-tasks` list without copying full task ids.
- Added top-level `tessera tasks [--json]` and `tessera chat --resume-task <task_id|#>` for script-friendly paused chat checkpoint discovery and chat-only trace projection resume without background reattach or workspace restore.
- Added a v0.1 manual testing guide for deterministic mock pause, checkpoint listing, numbered resume, trace inspection, and negative-path checks without live provider credentials.
- Added a real provider Chinese test-question guide covering conversation quality, structure, context continuity, safety boundaries, trace inspection, and pause/resume prompts.
- Added default config discovery so CLI commands can reuse `TESSERA_CONFIG` or the current directory `tessera.toml` without passing `--config` on every manual test run.
- Added an interactive `tessera chat` CLI REPL with `/help`, `/new`, `/profiles`, `/profile <id>`, `/status`, `/export`, and `/quit`, reusing the shared client projection and core event stream without tool or shell execution.
- Added `tessera init` for a secret-safe local config template plus interactive `/sessions` and `/resume <trace_id>` commands backed by read-only runtime trace summaries and client projection replay.
- Added provider-neutral chat history plumbing so CLI `/resume` follow-up prompts continue with restored user/assistant transcript while tracing only the new user turn.
- Added `tessera chat --resume <trace_id>` to start the interactive CLI directly from a trace-backed session.
- Added `tessera sessions` with text and JSON output for top-level trace-backed session discovery.
- Added `tessera transcript <trace_id>` with markdown and JSON output for REPL-free transcript inspection.
- Added `tessera chat --stdin` for pipe-friendly one-shot prompts.
- Added `tessera chat --file <path>` for file-backed one-shot prompts.
- Added `tessera chat --json` for script-friendly one-shot chat output containing `trace_id` and `assistant_text`.
- Added `tessera chat --continue` to start the interactive CLI from the most recent trace-backed session.
- Added `tessera replay <trace_id>` with text and JSON output for provider-free trace replay summaries.
- Added `tessera events <trace_id>` with text/JSON output and `--since` / `--limit` pagination for read-only trace event inspection.
- Added `tessera profiles` with text and JSON output for secret-safe provider profile inspection.
- Added `tessera config validate` with text/JSON output for read-only provider config checks, duplicate profile detection, and secret env presence reporting without exposing secret values.
- Added detailed text output for `tessera doctor`, including data dir, trace writability, SQLite index health, and configured provider profile IDs.
- Added `tessera chat --list-commands` to print interactive slash commands without resolving config, opening storage, or starting the REPL.
- Added REPL startup context and `/doctor` runtime health inside interactive `tessera chat`.
- Added REPL `/clear`, `/history`, and `/commands` local ergonomics without provider or storage execution.
- Added numbered session lists plus `/resume <number>` and `chat --resume <number>` support for trace-backed session recovery.
- Added REPL `/paste` multiline prompt mode with `/send` and `/cancel`.
- Added a REPL `/cancel` command that reports when no cancellable run is active, reserving the command for future async run cancellation.
- Added provider-neutral run cancellation controls in `tessera-core`, controls-aware CLI chat helpers, and shared client/TUI cancel intents so active provider streams can be interrupted without adding tool execution.
- Added active-run `/cancel` in the interactive CLI REPL by reading input concurrently with provider streaming and routing cancellation through `RunCancellationToken`.
- Added a bare `tessera` default entrypoint that launches the interactive mock REPL, keeping `tessera chat ...` for explicit and script-friendly workflows.

## [v0.1.0] - 2026-05-15

### Added

- Added provider HTTP error normalization for OpenAI-compatible and Ollama adapters, including provider-neutral error codes, retryability, safe details, and API-key/authorization/cookie redaction before trace persistence.
- Verified the final v0.1 OpenAI-compatible live smoke path against a OneAPI-compatible endpoint using `deepseek-v4-pro`, including trace review for secret-like material.

### Changed

- Changed core provider failure handling to write normalized `error`, `task_failed`, and `done` events before returning provider failures to callers.

## [v0.1.0-alpha.1] - 2026-05-15

### Added

- Added a global planning checklist covering completed v0.1 work, remaining v0.1 gates, v0.2-v0.5+ roadmap items, and mandatory update rules.
- Established the v0.1 Rust workspace with `protocol`, `client`, `core`, `providers`, `storage`, `config`, `cli`, and `tui` crates.
- Added provider-neutral protocol types for thread, turn, item, task, artifact, event frames, provider capability, reasoning delta, usage/cache/cost telemetry, and route decisions.
- Added JSONL trace writing with a rebuildable SQLite event index.
- Added a deterministic mock provider and a mock-driven core conversation loop.
- Added OpenAI-compatible and Ollama streaming provider adapters with parser tests for SSE and JSONL chunks.
- Added config-driven CLI provider routing and ignored live smoke tests for OpenAI-compatible and Ollama providers.
- Added SQLite runtime object queries, index rebuild from JSONL, and a golden trace replay gate.
- Added a core live event sink and CLI bridge so clients can consume `EventFrame`s as the run progresses while trace persistence still happens first.
- Added basic cancellation, provider event timeout, and bounded live-event backpressure semantics with `task_cancelled` trace events.
- Added a minimal TUI chat view-state reducer for input intents and streamed core event rendering.
- Added a `tessera tui` terminal loop with crossterm input, Ratatui rendering, and live channel delivery back into the TUI state.
- Added TUI profile switching through GUI-ready `ClientIntent` dispatch so prompt submission uses the currently selected provider profile.
- Added a `client` crate with UI-neutral `ClientIntent`, `ClientStatus`, `ClientProjection`, and `ClientSnapshot` for TUI and future Tauri GUI reuse.
- Added `/new`, `/save`, and `/export` basics through shared client slash-command intents, local TUI handling, and markdown projection export.
- Added a GUI-ready architecture note so future desktop/web clients reuse the same headless runtime, client intents, and UI-neutral view model instead of forking runtime behavior from CLI/TUI.
- Added ADR-001 for GUI architecture and toolkit direction, selecting a Tauri-first product GUI path with AI-ready typed IPC, permissions, fixture, and projection rules.
- Added a v0.1 release checklist and tag plan covering alpha/final tag criteria, verification gates, known limitations, release notes, and rollback.
- Added `tessera doctor --json` and `tessera chat --provider mock --prompt ...`.
- Added a minimal Ratatui status-line surface for profile, reasoning, cache, and cost placeholders.
- Added architecture, trace, protocol, crate-boundary, v0.1 planning, and DeepSeek-TUI lesson documents.
- Added crate README files and a Rust CI workflow.

### Changed

- Built `rusqlite` with bundled SQLite to reduce release/runtime dependency drift across user machines.
- Changed config-routed chat runs to use unique trace IDs so interactive sessions do not append duplicate sequence ranges to a fixed provider trace.
- Included user prompt text in `user_message_recorded` trace payloads so TUI and replay surfaces can render user turns from core events.
- Moved TUI message/status projection onto the shared `tessera-client` model while keeping terminal input and Ratatui rendering in `tessera-tui`.
- Updated the README from design-only status to the current v0.1 scaffold status.

### Notes

- This alpha is intended as the first small closed-loop runtime: mock provider, trace writing, SQLite index rebuild, replay gate, CLI doctor/chat, TUI chat loop, and shared client model.
- CLI execution still defaults to the mock provider path. Real OpenAI-compatible and Ollama adapters are present in the provider layer, but user-facing profile selection and live smoke tests are staged for a later slice.
- Tool execution, agent runtime, MCP, Auto Router, YOLO/trusted workspace mode, and long-term memory runtime remain out of v0.1 scope.
