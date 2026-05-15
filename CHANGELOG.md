# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added

- Added a global planning checklist covering completed v0.1 work, remaining v0.1 gates, v0.2-v0.5+ roadmap items, and mandatory update rules.
- Established the v0.1 Rust workspace with `protocol`, `core`, `providers`, `storage`, `config`, `cli`, and `tui` crates.
- Added provider-neutral protocol types for thread, turn, item, task, artifact, event frames, provider capability, reasoning delta, usage/cache/cost telemetry, and route decisions.
- Added JSONL trace writing with a rebuildable SQLite event index.
- Added a deterministic mock provider and a mock-driven core conversation loop.
- Added OpenAI-compatible and Ollama streaming provider adapters with parser tests for SSE and JSONL chunks.
- Added config-driven CLI provider routing and ignored live smoke tests for OpenAI-compatible and Ollama providers.
- Added SQLite runtime object queries, index rebuild from JSONL, and a golden trace replay gate.
- Added a core live event sink and CLI bridge so clients can consume `EventFrame`s as the run progresses while trace persistence still happens first.
- Added a minimal TUI chat view-state reducer for input intents and streamed core event rendering.
- Added a `tessera tui` terminal loop with crossterm input, Ratatui rendering, and live channel delivery back into the TUI state.
- Added TUI profile switching through GUI-ready `ClientIntent` dispatch so prompt submission uses the currently selected provider profile.
- Added a GUI-ready architecture note so future desktop/web clients reuse the same headless runtime, client intents, and UI-neutral view model instead of forking runtime behavior from CLI/TUI.
- Added `tessera doctor --json` and `tessera chat --provider mock --prompt ...`.
- Added a minimal Ratatui status-line surface for profile, reasoning, cache, and cost placeholders.
- Added architecture, trace, protocol, crate-boundary, v0.1 planning, and DeepSeek-TUI lesson documents.
- Added crate README files and a Rust CI workflow.

### Changed

- Built `rusqlite` with bundled SQLite to reduce release/runtime dependency drift across user machines.
- Changed config-routed chat runs to use unique trace IDs so interactive sessions do not append duplicate sequence ranges to a fixed provider trace.
- Included user prompt text in `user_message_recorded` trace payloads so TUI and replay surfaces can render user turns from core events.
- Updated the README from design-only status to the current v0.1 scaffold status.

### Notes

- CLI execution still defaults to the mock provider path. Real OpenAI-compatible and Ollama adapters are present in the provider layer, but user-facing profile selection and live smoke tests are staged for a later slice.
- Tool execution, agent runtime, MCP, Auto Router, YOLO/trusted workspace mode, and long-term memory runtime remain out of v0.1 scope.
