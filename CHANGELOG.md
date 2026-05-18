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
- Added the first Tauri GUI shell spike with a tested `tessera-gui-bridge`, typed mock/replay commands, bounded GUI event backpressure, and a React/Vite shell that renders shared `ClientSnapshot` projection without provider or storage access.
- Added `tessera-gui-bindings` to generate GUI TypeScript DTOs from Rust `protocol` / `client` / `gui-bridge` types, plus a contract test that keeps `apps/gui-tauri/src/generated/bindings.ts` in sync.
- Added a deterministic GUI smoke test for the Tauri shell covering mock/replay load, prompt submission, cancellation, new-thread reset, and toolbar action accessibility names.
- Added a v0.2 distribution plan covering GitHub Releases, Cargo, Homebrew, npm wrapper, Docker, checksums, publish ordering, mirror knobs, and v0.3+ acceptance gates.
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
