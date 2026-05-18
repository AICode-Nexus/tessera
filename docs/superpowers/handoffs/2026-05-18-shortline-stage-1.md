# Tessera Shortline Stage 1 Handoff

Date: 2026-05-18

## Current State

`main` is the current integration baseline.

Two shortline PRs were merged into `main` with passing CI:

- PR #1: `feat: add agent profile foundation and bare REPL entrypoint`
- PR #2: `feat: project context handles`

Recent `main` commits:

- `4004c64 feat: project context handles`
- `47e0e7f feat: add agent profile foundation and bare REPL entrypoint`
- `e43937b feat(cli): cancel active repl runs`

There are no open GitHub PRs after this stage. The local worktree has only the pre-existing untracked `output/` directory.

## Completed In This Stage

- Merged the agent profile foundation and bare REPL entrypoint into `main`.
- Merged context handle projection into `main`.
- Confirmed the merged `main` branch stays inside the architecture contract:
  - no tool execution
  - no agent runtime
  - no MCP runtime execution
  - no sandbox runtime
  - no provider/storage dependencies in `tessera-client`
  - no context source content reads for handle projection
- Verified the integrated baseline.

## Verification Evidence

Run from `/Users/admin/work/tessera` on `main`:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check
PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings
PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace
git diff --check
```

Result: all passed.

Notable test counts after integration:

- `crates/cli/tests/cli_contract.rs`: 53 passed
- `crates/client/tests/client_contract.rs`: 16 passed
- `crates/core/tests/conversation_engine_contract.rs`: 33 passed
- `crates/protocol/tests/protocol_contract.rs`: 16 passed
- `crates/tui/tests/tui_contract.rs`: 19 passed

## What Is Now Available

Agent profile foundation:

- `tessera-protocol` has metadata-only agent profile schema.
- `tessera-core` has a read-only agent registry.
- Bare `tessera` now starts the mock interactive REPL.
- The feature remains metadata-only and does not start an agent runtime.

Context handle projection:

- `ContextWorkbench::projection()` returns context references plus budget summary.
- `tessera-client` has `ClientContextHandle`, client-side source/placement DTOs, budget summary, snapshot storage, and `context_handles_summary`.
- GUI TypeScript bindings include the new context DTOs and `ContextId`.
- The feature does not read source content, build prompts, or write context trace events.

## Recommended Next Shortline Stage

Next stage should start from `main` and create a new branch, for example:

```bash
git switch main
git pull --ff-only origin main
git switch -c codex/context-handles-ui-gate
```

Recommended scope:

1. Add an actual UI consumer for context handles.
   - Keep this metadata-only.
   - Good first target: TUI or GUI status/detail surface that renders `ClientSnapshot.context_handles`.
   - Do not load file contents or build prompts.

2. Add a stronger generated-binding safety gate.
   - Add a TS typecheck or GUI smoke assertion that catches missing generated aliases like `ContextId`.
   - Keep it CI-friendly and deterministic.

3. Update docs and gates.
   - `CHANGELOG.md`
   - `docs/global-plan.md` if a checklist item is completed, added, deferred, or re-scoped
   - relevant crate README files if crate boundary wording changes

Suggested verification for this shortline stage:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check
PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings
PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace
git diff --check
```

If the GUI package is touched, also run the existing app-level tests for `apps/gui-tauri` using the repo's package-manager commands.

## Recommended Medium Stage

After context handles are visible and bindings are guarded, plan `Pause / resume foundation`.

Suggested design direction:

- Reuse existing cancellation, sessions, resume, trace replay, task lifecycle, and client projection.
- Keep pause/resume as provider-neutral lifecycle metadata first.
- Do not implement background agents, sub-agent persistence, swarm scheduling, or tool execution as part of the first slice.
- Decide whether pause means:
  - local UI suspension only,
  - runtime task state checkpoint metadata,
  - or resumable provider request orchestration.

The conservative first slice should likely be metadata and client intent plumbing, not real suspended provider execution.

## New Session Prompt

Use this to resume cleanly in a new conversation:

```text
继续 Tessera 开发。请先阅读 /Users/admin/work/tessera/AGENTS.md 和 /Users/admin/work/tessera/docs/superpowers/handoffs/2026-05-18-shortline-stage-1.md。

当前 main 已合并 PR #1 和 PR #2，质量门禁已通过，只剩未跟踪 output/ 不要动。

下一步按 handoff 建议做 shortline stage 2：从 main 新建分支，先设计并实现 context handles 的实际 UI 消费和 TypeScript/GUI binding 安全 gate。继续遵守 Rust-first、headless runtime、client UI-neutral、TUI/GUI 不绕过 core、v0.1/v0.2 scope 约束。
```
