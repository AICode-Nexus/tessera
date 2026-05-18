# Context Handles UI Gate Design

Date: 2026-05-18

## Goal

Make context handles visible to an existing UI surface and add a stronger GUI TypeScript safety gate so generated context DTOs cannot silently drift.

## Scope

This is a shortline stage after context handle projection. It consumes existing `tessera-client` metadata only:

- `ClientSnapshot.context_handles`
- `ClientStatus.context_handles_summary`
- generated TypeScript DTOs for `ClientContextHandle`, source/placement enums, budget summary, and `ContextId`

This stage does not read context source content, build prompts, write context trace events, call providers, access storage internals, execute tools, or start agent/MCP/sandbox runtime.

## Approach

Use a conservative TUI-first implementation:

1. Render `ClientStatus.context_handles_summary` in the TUI status line.
2. Add a TUI contract test that populates context handles through `ClientSnapshot::set_context_handles` and verifies the status line includes the handle summary.
3. Strengthen the GUI TypeScript gate by ensuring fallback/test snapshots include `context_handles` and `context_handles_summary`, and by running the app's TypeScript build/test path.

This keeps the runtime boundary intact while proving the client projection is actually consumed by shells.

## Components

### `crates/tui`

`status_line` already renders profile, reasoning, task, artifact, approval, memory, context telemetry, usage, cache, and cost summaries. Add the context handle summary as another plain status segment.

Expected status fragment:

```text
context 2 handles / 150/160 tokens
```

### `apps/gui-tauri`

The generated bindings already include context handle DTOs. The browser fallback and view-model test snapshots need to satisfy the latest `ClientSnapshot` shape by including:

- `status.context_handles_summary`
- `context_handles`

The GUI gate should use existing commands:

```bash
npm --prefix apps/gui-tauri test
npm --prefix apps/gui-tauri run build
```

`npm run build` includes `tsc --noEmit`, which catches missing generated aliases or incomplete snapshot fixtures.

## Testing

Use TDD for behavior changes:

1. Add a failing TUI contract test for status-line context handle visibility.
2. Implement the minimal status-line segment.
3. Add or update GUI TypeScript fixtures so the build catches generated binding drift.
4. Run targeted tests, then full gates.

Required final verification:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check
PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings
PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace
npm --prefix apps/gui-tauri test
npm --prefix apps/gui-tauri run build
git diff --check
```

## Review Note

The Superpowers spec review step normally dispatches a reviewer subagent. This Codex environment only permits subagents when the user explicitly asks for them, so this stage uses local self-review and full verification instead.
