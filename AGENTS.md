# Tessera Agent Instructions

This repository has entered v0.1 implementation. Architecture documents remain the contract, but source changes are now expected when they follow the scoped checklist.

## Current Contract

- Keep Tessera Rust-first and quality-first.
- Preserve a single headless runtime shared by CLI, TUI, replay, and future runtime APIs.
- Treat `docs/technical-architecture.md`, `docs/deepseek-tui-lessons.md`, `docs/global-plan.md`, `docs/v0.1-plan.md`, `docs/protocol-v0.md`, `docs/trace-schema-v0.md`, and `docs/crate-boundaries.md` as the current implementation contract.
- Update `docs/global-plan.md` whenever a staged checklist item is completed, added, removed, or deliberately deferred.

## Architecture Rules

- TUI is a view. It must not call provider SDKs or own runtime execution.
- CLI is an entry point. It must not bypass core.
- Providers convert provider-specific streams into protocol events. They must not execute tools, write storage, or make policy decisions.
- Storage writes JSONL trace and SQLite indexes. JSONL is the event truth; SQLite is rebuildable.
- Core owns run lifecycle, event routing, and provider/storage coordination.
- Protocol must stay provider-neutral and UI-neutral.
- DeepSeek-TUI lessons may inform Tessera design, but DeepSeek-specific capabilities must remain provider extensions.
- Auto routing, YOLO mode, tool execution, sub-agents, MCP, ACP, sandbox, snapshots, and diagnostics are staged roadmap items unless the user explicitly changes scope.

## v0.1 Scope

Allowed:

- Protocol, core, providers, storage, config, cli, and tui crates.
- Architecture, protocol, trace schema, and crate boundary documents.
- Mock runtime, provider adapter skeletons, trace, doctor, and CLI/TUI v0.1 work listed in `docs/global-plan.md`.

Not allowed in v0.1 unless the user changes scope:

- Tool execution.
- Automatic shell commands from the model.
- MCP runtime.
- Agent runtime.
- Swarm scheduler.
- Long-term memory runtime.
- Learning runtime.
- Complex multi-window TUI.

## Safety

- Never commit API keys, `.env` secrets, provider tokens, cookies, or authorization headers.
- Do not write secrets into trace, SQLite, session files, tests, or logs.
- Real provider tests must be opt-in and skipped when required environment variables are absent.

## Verification

For documentation-only changes, verify with:

```bash
git diff --check
```

For implementation changes, expected gates are:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check
PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings
PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace
```
