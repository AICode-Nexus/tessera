# Context Handle Projection Design

Date: 2026-05-18

## Goal

Add a narrow UI-neutral projection for context handles so future CLI/TUI/GUI and agent surfaces can show what context is attached without reading source content or building provider prompts.

## Scope

This slice is a v0.5 foundation on top of the existing `ContextReference` and `ContextWorkbench` work. It exposes context references as handles and budget summaries only.

In scope:

- A client-facing `ClientContextHandle` projection.
- A `ClientSnapshot` list of projected context handles.
- A short status summary for context handles and estimated token usage.
- A core read-only projection helper from `ContextWorkbench`.
- Documentation and checklist updates.
- Contract tests covering protocol neutrality, core read-only behavior, client projection, and TypeScript bindings if required.

Out of scope:

- Reading files, directories, artifacts, traces, URLs, or inline source bodies.
- Building provider prompts.
- Writing context trace events.
- Adding CLI slash commands such as `/context`.
- Implementing agent loop behavior.
- Implementing context compaction, loader, slice reads, or cache serialization.

## Architecture

`tessera-protocol` already owns `ContextReference`, `ContextSource`, `ContextPlacement`, and `ContextBudget`. Those remain the durable schema for context metadata. This design does not add `content`, `bytes`, file paths with secret expansion, or any runtime execution handle.

`tessera-core` keeps `ContextWorkbench` as the pure in-memory owner of context references and budget math. It gains a read-only projection method that returns a cloneable value containing the current references and `ContextBudgetSummary`. The method must not read any source URI, canonicalize workspace paths, or build prompt text.

`tessera-client` gets the UI-neutral projection:

- `ClientContextHandle`
- `ClientContextSourceKind`
- `ClientContextPlacement`

`ClientSnapshot` stores `context_handles: Vec<ClientContextHandle>`. `ClientStatus` stores `context_handles_summary`, for example `context 3 handles / 175 tokens` or `context 3 handles / 175/160 tokens over budget`. The snapshot exposes a method such as `set_context_handles(...)` or `apply_context_projection(...)` so CLI/TUI/GUI bridge layers can update visible context handles without depending on core internals.

The GUI TypeScript binding generator should include `ClientContextHandle` and related enums if they derive `TS`.

## Data Flow

1. A future runtime component maintains a `ContextWorkbench`.
2. The component asks core for a read-only projection of handles and budget summary.
3. A shell bridge converts or passes the projection into `tessera-client`.
4. `ClientSnapshot` updates its `context_handles` list and context handle status summary.
5. TUI/GUI render handles and summary as normal view state.

No source content flows through this path. Source metadata remains limited to `ContextSource.kind`, optional `uri`, optional `label`, placement, token estimate, pinned flag, and optional summary.

## Error Handling

The projection path should be infallible because it only clones existing metadata and summarizes known token estimates. Invalid or missing source labels are view concerns, not runtime errors.

If a handle has no label, UI consumers can fall back to URI or context ID. This fallback can live in client helper methods or tests, but the runtime does not need to mutate the reference.

## Testing

Tests should be TDD-first:

- Protocol contract: `ContextReference` remains metadata-only and does not serialize `content` or `bytes`.
- Core contract: `ContextWorkbench` read-only projection returns references and budget summary without loading sources.
- Client contract: `ClientSnapshot` accepts projected context handles, exposes them in order, and updates `context_handles_summary`.
- GUI bindings contract: checked-in TypeScript bindings include the new client context handle DTOs and remain free of forbidden runtime commands.

## Documentation

Update:

- `CHANGELOG.md`
- `docs/global-plan.md`
- `docs/protocol-v0.md` if the public shape changes.
- `docs/technical-architecture.md` or `docs/crate-boundaries.md` only if boundaries need clarification.
- `crates/client/README.md`, `crates/core/README.md`, or `crates/protocol/README.md` if public crate responsibilities change.

## Acceptance Criteria

- Context handles can be projected into `ClientSnapshot` without source content.
- Context handle summary reports count and token usage.
- Core projection is read-only and does not perform file IO or prompt building.
- Existing provider, storage, CLI, TUI, and GUI boundaries remain intact.
- Full verification gates pass:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check
PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings
PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace
git diff --check
```
