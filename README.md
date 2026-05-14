# Tessera

Tessera is a model-agnostic, agent-ready terminal workbench built on typed events, auditable tools, replayable runs, and composable skills.

## Current Status

This repository now has the v0.1 Rust workspace scaffold and a mock-driven runtime slice. The current implementation is intentionally narrow: protocol types, trace storage, provider adapters, core conversation loop, `doctor --json`, mock `chat`, and a minimal Ratatui chat view-state surface.

## Design Goals

- Model-agnostic provider architecture.
- Rust-first, quality-focused local runtime.
- Headless core with replaceable TUI, CLI, and runtime API surfaces.
- Auditable tool execution through policy gates.
- Replayable runs with durable thread, turn, item, task, and artifact records.
- Agent-ready architecture with skills, memory, multi-agent workflows, swarm scheduling, and learning proposals.
- Multi-task and multi-window TUI model without coupling UI state to runtime execution.

## Documents

- [Requirements](docs/requirements.md)
- [Architecture](docs/architecture.md)
- [Technical Architecture](docs/technical-architecture.md)
- [DeepSeek-TUI Lessons](docs/deepseek-tui-lessons.md)
- [v0.1 Plan](docs/v0.1-plan.md)
- [Global Plan](docs/global-plan.md)
- [Protocol v0](docs/protocol-v0.md)
- [Trace Schema v0](docs/trace-schema-v0.md)
- [Crate Boundaries](docs/crate-boundaries.md)
- [Changelog](CHANGELOG.md)

## Architecture Contract

The current implementation contract is still architecture-led:

- Keep the first implementation limited to `protocol`, `core`, `providers`, `storage`, `config`, `cli`, and `tui`.
- Keep future tools, agents, memory, skills, learning, and swarm support as protocol-ready extensions, not v0.1 runtime features.
- Keep CLI and TUI on top of the same headless runtime.
- Treat JSONL trace as the durable event truth and SQLite as a rebuildable index.
- Access SQLite through `rusqlite`; build it with bundled SQLite for local release portability.

## Development

Use the native Rust toolchain first in `PATH`:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace
PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings
PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check
```

Run the current mock path:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo run -p tessera-cli -- doctor --json
PATH="$HOME/.cargo/bin:$PATH" cargo run -p tessera-cli -- chat --provider mock --prompt "hello"
```

Run with an explicit config profile:

```toml
data_dir = "/tmp/tessera-dev"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
```

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo run -p tessera-cli -- chat --config ./tessera.toml --provider offline --prompt "hello"
```

## Non-Goals For This Phase

- No live provider smoke tests by default; they must be explicitly enabled with environment variables and reachable services.
- No guarantee yet that real provider paths have been manually smoke-tested in this workspace.
- No tool execution.
- No agent runtime.

The next milestone is to wire the TUI chat view-state into a real terminal event loop, then verify real provider smoke paths when OpenAI-compatible or Ollama endpoints are reachable.
