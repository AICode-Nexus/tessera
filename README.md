# Tessera

Tessera is a model-agnostic, agent-ready terminal workbench built on typed events, auditable tools, replayable runs, and composable skills.

## Current Status

This repository has shipped the v0.1 trace-first local runtime and is now advancing through foundation work for later versions. The current implementation includes provider-neutral protocol types, shared client projection, trace storage, provider adapters, a core conversation loop with cancellation/pause controls, one-shot and interactive CLI chat, trace-backed session and paused-task resume, transcript/replay/event inspection commands, a no-tool single-agent loop with trace-backed run/step summaries, opt-in project instruction discovery/source reporting, a minimal Ratatui TUI, mock/replay GUI shell foundations, and v0.2-v0.4 metadata/projection foundations for runtime queries, tool policy, approvals, memory proposal review, diagnostics events, MCP tool metadata adaptation, ordered tool results, repair telemetry, workspace guardrail decisions, OS sandbox profile planning, checkpoint metadata planning, and read-only runtime API JSON/SSE shaping. Tool execution, skill runtime, full coding-agent workflow, MCP runtime, workspace restore, swarm scheduling, and learning apply paths remain gated by the roadmap.

## Design Goals

- Model-agnostic provider architecture.
- Rust-first, quality-focused local runtime.
- Headless core with replaceable CLI, TUI, future GUI, and runtime API surfaces.
- Auditable tool execution through policy gates.
- Replayable runs with durable thread, turn, item, task, and artifact records.
- Agent-ready architecture with skills, memory, multi-agent workflows, swarm scheduling, and learning proposals.
- Multi-task and multi-window client model without coupling UI state to runtime execution.

## Documents

- [Requirements](docs/requirements.md) (early product baseline; current roadmap lives below)
- [Architecture](docs/architecture.md) (early vision; current contract lives in the documents below)
- [Technical Architecture](docs/technical-architecture.md)
- [DeepSeek-TUI Lessons](docs/deepseek-tui-lessons.md)
- [Reasonix Lessons](docs/reasonix-lessons.md)
- [Coding-Agent Direction](docs/coding-agent-direction.md)
- [GUI-Ready Architecture](docs/gui-ready-architecture.md)
- [ADR-001: GUI Architecture and Toolkit Direction](docs/adr/ADR-001-gui-architecture-and-toolkit.md)
- [Distribution Plan](docs/distribution-plan.md)
- [Version Plan](docs/version-plan.md)
- [Global Plan](docs/global-plan.md)
- [v0.1 Plan](docs/v0.1-plan.md)
- [v0.1 Release Checklist](docs/v0.1-release-checklist.md)
- [Protocol v0](docs/protocol-v0.md)
- [Trace Schema v0](docs/trace-schema-v0.md)
- [Crate Boundaries](docs/crate-boundaries.md)
- [Changelog](CHANGELOG.md)

## Architecture Contract

The current implementation contract is still architecture-led:

- Keep the headless runtime limited to `protocol`, `client`, `core`, `providers`, `storage`, `config`, `cli`, and `tui`, with GUI work entering through `gui-bridge`, `gui-bindings`, and `apps/gui-tauri` shell code only.
- Follow `docs/version-plan.md` for v0.1-v0.9 scope, dependencies, and exit criteria; use `docs/global-plan.md` as the current status dashboard.
- Follow `docs/coding-agent-direction.md` when aligning with DeepSeek-TUI, Reasonix, Codex CLI/GUI/App Server, Claude Code CLI/Desktop/Web, skills, hooks, subagents, automations, and modern coding-agent UX.
- Keep future tools, agents, memory, skills, learning, and swarm support behind their version gates.
- Keep CLI, TUI, and future GUI on top of the same headless runtime.
- Keep client UI state in UI-neutral reducers and view models before implementing the Tauri-first GUI path.
- Treat Tauri 2 + TypeScript/React/Vite as the default product GUI direction, with egui limited to possible internal inspector work and GPUI kept as a watch item.
- Treat app-server, hooks, automations, default-on/global instruction loading, and GUI Git controls as policy/trace-gated roadmap items, not shortcuts around core.
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
PATH="$HOME/.cargo/bin:$PATH" cargo install --path crates/cli --force
tessera
tessera doctor
tessera doctor --json
tessera init --config ./tessera.toml
tessera profiles
tessera chat --list-commands
tessera chat --provider mock --prompt "hello"
tessera chat --provider mock --prompt "hello" --json
tessera agent run --provider mock --goal "summarize this task"
tessera agent run --provider mock --goal "summarize this task" --json
tessera instructions inspect --workspace .
tessera agent run --provider mock --goal "summarize this task" --instructions --workspace .
printf 'hello from stdin\n' | tessera chat --provider mock --stdin
tessera chat --provider mock --file prompt.md
tessera sessions
tessera transcript <trace_id>
tessera replay <trace_id>
tessera events <trace_id> --limit 20
tessera chat --provider mock
tessera tui --provider mock
```

For source-tree development without installing, prefix the same CLI arguments with `PATH="$HOME/.cargo/bin:$PATH" cargo run -p tessera-cli --`. The bare `tessera` command launches the default mock interactive REPL, while `tessera chat ...` keeps the explicit script-friendly command surface.

Run with an explicit config profile:

```toml
data_dir = "/tmp/tessera-dev"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
```

```bash
tessera doctor --config ./tessera.toml
tessera doctor --config ./tessera.toml --json
tessera chat --list-commands
tessera chat --config ./tessera.toml --provider offline --prompt "hello"
tessera chat --config ./tessera.toml --provider offline --prompt "hello" --json
tessera agent run --config ./tessera.toml --provider offline --goal "summarize this task"
tessera agent run --config ./tessera.toml --provider offline --goal "summarize this task" --json
tessera instructions inspect --workspace .
tessera agent run --config ./tessera.toml --provider offline --goal "summarize this task" --instructions --workspace .
tessera config validate --config ./tessera.toml
tessera config validate --config ./tessera.toml --json
tessera profiles --config ./tessera.toml
tessera profiles --config ./tessera.toml --json
cat prompt.md | tessera chat --config ./tessera.toml --provider offline --stdin
tessera chat --config ./tessera.toml --provider offline --file prompt.md
tessera sessions --config ./tessera.toml
tessera sessions --config ./tessera.toml --json
tessera tasks --config ./tessera.toml
tessera tasks --config ./tessera.toml --json
tessera transcript <trace_id> --config ./tessera.toml
tessera transcript <trace_id> --config ./tessera.toml --json
tessera replay <trace_id> --config ./tessera.toml
tessera replay <trace_id> --config ./tessera.toml --json
tessera events <trace_id> --config ./tessera.toml --since 0 --limit 20
tessera events <trace_id> --config ./tessera.toml --since 0 --limit 20 --json
tessera chat --config ./tessera.toml --provider offline
tessera chat --config ./tessera.toml --provider offline --resume <trace_id>
tessera chat --config ./tessera.toml --provider offline --continue
tessera chat --config ./tessera.toml --resume-task <task_id|#>
```

Use bare `tessera` to start the default interactive mock REPL. Use `chat --stdin` to pipe a prompt into one-shot chat, or `chat --file <path>` to read a prompt from a UTF-8 file. Add `--json` to a one-shot chat command to emit `trace_id` and `assistant_text` for scripts. Use `agent run --goal ...` for the first no-tool single-agent path; it records `agent_run_started`, `agent_step_started`, `agent_step_completed`, and `agent_run_completed` events and emits task/trace/status summaries for scripts. Add `--instructions --workspace <path>` to opt into project-local `AGENTS.md` / `CLAUDE.md` discovery; Tessera records `instructions_discovered` source metadata, applies byte limits and redaction, and does not write instruction text to trace. Use `tessera instructions inspect --workspace <path>` to preview that source report without starting a provider run. This agent path does not execute tools, mutate files, activate skills, run hooks, or keep background tasks alive. Use `tessera doctor` to inspect resolved runtime health, trace writability, SQLite index health, and provider profile IDs in a readable form. Use `tessera config validate` to check provider shape, duplicate IDs, resolved data dir, and configured secret env presence without touching storage or printing secret values. Use `tessera profiles` to inspect configured provider profiles without exposing secret values, `tessera sessions` to list numbered trace-backed sessions, `tessera tasks` to list currently resumable paused chat checkpoints, `tessera transcript <trace_id>` to inspect one without entering the REPL, `tessera replay <trace_id>` to reconstruct a trace summary without provider access, and `tessera events <trace_id>` to page raw trace events with `--since` / `--limit`. In interactive `chat` mode, startup output shows the active profile, data dir, and configured profiles; use `/help`, `/commands`, `/new`, `/clear`, `/cancel`, `/paste`, `/profiles`, `/profile <id>`, `/sessions`, `/resume <trace_id|#>`, `/resume-tasks`, `/resume-task <task_id|#>`, `/doctor`, `/history`, `/status`, `/export`, and `/quit`. `/cancel` discards paste buffers in paste mode, cancels the active provider run while streaming, and reports when there is no active run to cancel. Use `chat --list-commands` to print that slash-command list without resolving config or starting the REPL. You can also start directly from a prior trace with `chat --resume <trace_id>` or resume the most recent trace with `chat --continue`; after checking `/sessions`, `chat --resume <number>` and REPL `/resume <number>` restore the numbered session from the same sorted list. After either session resume path, the next prompt uses the restored user/assistant transcript as provider-visible chat history while writing only the new turn to trace. For paused chat tasks, `tessera tasks` lists resumable checkpoints and `chat --resume-task <task_id|#>` starts a new chat run from the saved trace projection, then appends `task_resumed` to the source trace. This does not freeze provider sockets, reattach background runtimes, restore workspace files, or resume non-chat tasks.

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
- No tool-using, skill-aware, or background agent runtime beyond the no-tool `agent run` slice.

The next CLI milestone is to evolve the interactive shell toward coding-agent ergonomics after sandbox, checkpoint, policy, and tool execution gates are ready.
