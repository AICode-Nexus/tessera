# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added

- Established the v0.1 Rust workspace with `protocol`, `core`, `providers`, `storage`, `config`, `cli`, and `tui` crates.
- Added provider-neutral protocol types for thread, turn, item, task, artifact, event frames, provider capability, reasoning delta, usage/cache/cost telemetry, and route decisions.
- Added JSONL trace writing with a rebuildable SQLite event index.
- Added a deterministic mock provider and a mock-driven core conversation loop.
- Added OpenAI-compatible and Ollama streaming provider adapters with parser tests for SSE and JSONL chunks.
- Added `tessera doctor --json` and `tessera chat --provider mock --prompt ...`.
- Added a minimal Ratatui status-line surface for profile, reasoning, cache, and cost placeholders.
- Added architecture, trace, protocol, crate-boundary, v0.1 planning, and DeepSeek-TUI lesson documents.
- Added crate README files and a Rust CI workflow.

### Changed

- Updated the README from design-only status to the current v0.1 scaffold status.

### Notes

- CLI execution still defaults to the mock provider path. Real OpenAI-compatible and Ollama adapters are present in the provider layer, but user-facing profile selection and live smoke tests are staged for a later slice.
- Tool execution, agent runtime, MCP, Auto Router, YOLO/trusted workspace mode, and long-term memory runtime remain out of v0.1 scope.
