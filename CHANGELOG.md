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
