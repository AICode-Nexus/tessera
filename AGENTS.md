# Tessera Agent Instructions

This repository has shipped v0.1 and is now roadmap-driven. Architecture documents remain the contract, but source changes are expected when they follow the scoped checklist and version gates.

## Current Contract

- Keep Tessera Rust-first and quality-first.
- Preserve a single headless runtime shared by CLI, TUI, replay, future GUI, and future runtime APIs.
- Treat `docs/technical-architecture.md`, `docs/version-plan.md`, `docs/global-plan.md`, `docs/coding-agent-direction.md`, `docs/deepseek-tui-lessons.md`, `docs/reasonix-lessons.md`, `docs/protocol-v0.md`, `docs/trace-schema-v0.md`, and `docs/crate-boundaries.md` as the current implementation contract.
- Treat `docs/v0.1-plan.md` and `docs/v0.1-release-checklist.md` as historical v0.1 release contract documents.
- Update `docs/version-plan.md` when a version scope, dependency, exit criterion, or cross-version gate changes.
- Update `docs/global-plan.md` whenever a staged checklist item is completed, added, removed, or deliberately deferred.
- Update `docs/coding-agent-direction.md` when aligning or changing DeepSeek-TUI, Reasonix, Codex CLI/GUI/App Server, Claude Code CLI/Desktop/Web, skills, hooks, subagents, automations, or modern coding-agent UX direction.

## Architecture Rules

- TUI is a view. It must not call provider SDKs or own runtime execution.
- CLI is an entry point. It must not bypass core.
- Providers convert provider-specific streams into protocol events. They must not execute tools, write storage, or make policy decisions.
- Storage writes JSONL trace and SQLite indexes. JSONL is the event truth; SQLite is rebuildable.
- Core owns run lifecycle, event routing, and provider/storage coordination.
- Protocol must stay provider-neutral and UI-neutral.
- DeepSeek-TUI lessons may inform Tessera design, but DeepSeek-specific capabilities must remain provider extensions.
- Auto routing, YOLO mode, tool execution, sub-agents, MCP, ACP, sandbox, snapshots, diagnostics, memory, project instruction ingestion, hooks, automations, app-server listeners, GUI Git mutation, swarm, and learning are staged roadmap items unless the user explicitly changes scope.

## Version Scope

v0.1 has shipped. Future work must follow `docs/version-plan.md` and `docs/global-plan.md`.

Always allowed when scoped by the current plan:

- Protocol, client, core, providers, storage, config, cli, and tui crates.
- Architecture, protocol, trace schema, and crate boundary documents.
- GUI bridge/bindings and `apps/gui-tauri` only when they remain thin clients over shared runtime semantics.
- Mock runtime, provider adapter skeletons, trace, doctor, CLI/TUI work, and roadmap-listed foundation work.

Not allowed unless the active version gate explicitly permits it:

- Tool execution.
- Automatic shell commands from the model.
- MCP runtime.
- Agent runtime.
- Provider socket freezing.
- Project instruction ingestion without precedence, byte limits, source reporting, redaction, and trace references.
- Hook runtime.
- Automation runtime.
- App-server listener.
- GUI Git mutation.
- Workspace restore/revert.
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
