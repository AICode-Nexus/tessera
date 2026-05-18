# Tessera Active Pause Wiring Handoff

Date: 2026-05-18

## Branch

`codex/active-pause-wiring`

## Completed

- Merged PR #8 into `main`.
- Wired interactive CLI active `/pause` to a per-run `RunPauseToken`.
- Kept idle CLI `/pause [task_id]` as metadata-only behavior through `CliReplSession::handle_command`.
- Kept `/resume-task <task_id>` metadata-only.
- Added a TUI runtime-control helper that dispatches `CancelTask` and `PauseTask` through handler callbacks and applies notice/error messages to `ChatViewState`.
- Updated the CLI TUI entrypoint to track an active `RunPauseToken` alongside the active cancellation token.
- Updated `CHANGELOG.md`.
- Updated `docs/global-plan.md` to mark CLI/TUI active pause wiring complete.

## TDD Notes

- CLI RED: `repl_pause_interrupts_active_run_and_records_paused_trace` first failed because active `/pause` still fell through to the metadata-only command path.
- CLI GREEN: active `/pause` now records `task_paused`, avoids `task_cancelled` / `task_completed`, and reports `pause requested`.
- TUI RED: `runtime_control_intent_invokes_pause_handler_from_tui_state` first failed because no runtime-control helper existed.
- TUI GREEN: `apply_runtime_control_intent` now invokes pause handlers and applies notices to the view state.

## Boundary Review

This stage wires active pause requests only:

- No `/resume-task` runtime execution was added.
- No provider socket freezing was implemented.
- No resume envelope was written.
- No background runtime daemon or reattach service was added.
- No checkpoint restore/revert behavior was implemented.
- No tool, MCP, agent, sub-agent, or swarm runtime was added.

## Verification

Passed:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-cli --test cli_contract repl_pause_interrupts_active_run_and_records_paused_trace -- --nocapture
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-tui --test tui_contract runtime_control_intent_invokes_pause_handler_from_tui_state -- --nocapture
PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check
PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings
PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace
git diff --check
```

## Next Recommended Stage

After this PR is merged, move to the first resume-envelope design/implementation slice for chat-only paused tasks. Keep background reattach, checkpoint restore, tools, MCP, and agent runtime staged until their contracts are ready.

## Notes For Next Session

- `output/` remains a pre-existing untracked directory and should be ignored unless explicitly requested.
- Paused traces still represent a lifecycle state, not a durable replay envelope.
