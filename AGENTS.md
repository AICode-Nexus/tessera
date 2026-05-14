# Tessera Agent Instructions

This repository is in the requirements and architecture phase unless the user explicitly asks to start implementation.

## Current Contract

- Do not scaffold the Rust workspace unless the user explicitly asks for implementation.
- Do not add source code while working on architecture documents.
- Keep Tessera Rust-first and quality-first.
- Preserve a single headless runtime shared by CLI, TUI, replay, and future runtime APIs.
- Treat `docs/technical-architecture.md`, `docs/deepseek-tui-lessons.md`, `docs/v0.1-plan.md`, `docs/protocol-v0.md`, `docs/trace-schema-v0.md`, and `docs/crate-boundaries.md` as the current implementation contract.

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

- Architecture documents.
- Protocol and trace schema documents.
- Crate boundary documents.
- Later, when approved: Rust workspace scaffold, protocol/core/providers/storage/config/cli/tui crates.

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
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
