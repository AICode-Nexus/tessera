# Tessera

Tessera is a model-agnostic, agent-ready terminal workbench built on typed events, auditable tools, replayable runs, and composable skills.

## Current Status

This repository now has the v0.1 Rust workspace scaffold and a mock-driven runtime slice. The current implementation is intentionally narrow: protocol types, shared client projection, trace storage, provider adapters, core conversation loop, `doctor --json`, safe config initialization, one-shot and interactive CLI `chat` with trace-backed session listing/resume that continues with restored user/assistant history, a minimal Ratatui terminal chat loop with profile switching and live core event delivery, early v0.2 GUI work over mock/replay projection and generated TypeScript DTOs, plus v0.3/v0.4 foundation metadata/projection for tool policy, approvals, memory proposal review, diagnostics events, MCP tool metadata adaptation, ordered tool results, repair telemetry, workspace guardrail decisions, OS sandbox profile planning, checkpoint metadata planning, and read-only runtime API JSON/SSE shaping without tool execution.

## Design Goals

- Model-agnostic provider architecture.
- Rust-first, quality-focused local runtime.
- Headless core with replaceable CLI, TUI, future GUI, and runtime API surfaces.
- Auditable tool execution through policy gates.
- Replayable runs with durable thread, turn, item, task, and artifact records.
- Agent-ready architecture with skills, memory, multi-agent workflows, swarm scheduling, and learning proposals.
- Multi-task and multi-window client model without coupling UI state to runtime execution.

## Documents

- [Requirements](docs/requirements.md)
- [Architecture](docs/architecture.md)
- [Technical Architecture](docs/technical-architecture.md)
- [DeepSeek-TUI Lessons](docs/deepseek-tui-lessons.md)
- [Reasonix Lessons](docs/reasonix-lessons.md)
- [GUI-Ready Architecture](docs/gui-ready-architecture.md)
- [ADR-001: GUI Architecture and Toolkit Direction](docs/adr/ADR-001-gui-architecture-and-toolkit.md)
- [Distribution Plan](docs/distribution-plan.md)
- [v0.1 Plan](docs/v0.1-plan.md)
- [v0.1 Release Checklist](docs/v0.1-release-checklist.md)
- [Global Plan](docs/global-plan.md)
- [Protocol v0](docs/protocol-v0.md)
- [Trace Schema v0](docs/trace-schema-v0.md)
- [Crate Boundaries](docs/crate-boundaries.md)
- [Changelog](CHANGELOG.md)

## Architecture Contract

The current implementation contract is still architecture-led:

- Keep the headless runtime limited to `protocol`, `client`, `core`, `providers`, `storage`, `config`, `cli`, and `tui`, with GUI work entering through `gui-bridge`, `gui-bindings`, and `apps/gui-tauri` shell code only.
- Keep future tools, agents, memory, skills, learning, and swarm support as protocol-ready extensions, not v0.1 runtime features.
- Keep CLI, TUI, and future GUI on top of the same headless runtime.
- Keep client UI state in UI-neutral reducers and view models before implementing the Tauri-first GUI path.
- Treat Tauri 2 + TypeScript/React/Vite as the default product GUI direction, with egui limited to possible internal inspector work and GPUI kept as a watch item.
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
PATH="$HOME/.cargo/bin:$PATH" cargo run -p tessera-cli -- init --config ./tessera.toml
PATH="$HOME/.cargo/bin:$PATH" cargo run -p tessera-cli -- chat --provider mock --prompt "hello"
PATH="$HOME/.cargo/bin:$PATH" cargo run -p tessera-cli -- chat --provider mock
PATH="$HOME/.cargo/bin:$PATH" cargo run -p tessera-cli -- tui --provider mock
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
PATH="$HOME/.cargo/bin:$PATH" cargo run -p tessera-cli -- chat --config ./tessera.toml --provider offline
```

In interactive `chat` mode, use `/help`, `/new`, `/profiles`, `/profile <id>`, `/sessions`, `/resume <trace_id>`, `/status`, `/export`, and `/quit`. After `/resume`, the next prompt uses the restored user/assistant transcript as provider-visible chat history while writing only the new turn to trace.

Run the GUI shell spike:

```bash
cd apps/gui-tauri
npm install
npm test
npm run build
PATH="$HOME/.cargo/bin:$PATH" cargo check --manifest-path src-tauri/Cargo.toml
```

Regenerate and verify GUI DTO bindings:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo run -p tessera-gui-bindings -- apps/gui-tauri/src/generated/bindings.ts
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-gui-bindings --test bindings_contract
```

## Non-Goals For This Phase

- No live provider smoke tests by default; they must be explicitly enabled with environment variables and reachable services.
- No guarantee yet that real provider paths have been manually smoke-tested in this workspace.
- No tool execution.
- No agent runtime.

The next CLI milestone is to evolve the interactive shell toward coding-agent ergonomics after sandbox, checkpoint, policy, and tool execution gates are ready.
