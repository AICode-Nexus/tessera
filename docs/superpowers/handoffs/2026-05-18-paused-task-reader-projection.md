# Tessera Paused Task Reader Projection Handoff

Date: 2026-05-18

## Branch

`codex/paused-task-reader-projection`

## Completed

- Merged PR #6 into `main` after the suspended/background resume design was ready.
- Added a `RuntimeReader::list_tasks` contract test for replaying `task_paused` and `task_resumed` records from JSONL trace.
- Implemented core task projection so:
  - `task_paused` projects to `TaskStatus::Paused`;
  - `task_resumed` projects to `TaskStatus::Running`;
  - resumed tasks keep or populate `started_at` and clear `finished_at`.
- Updated `CHANGELOG.md`.
- Updated `docs/global-plan.md` to mark the paused task reader projection complete while leaving runtime pause/resume execution staged.

## TDD Notes

The new test failed before implementation with the paused task still projected as `Running`, then passed after adding the two projection branches in `crates/core/src/lib.rs`.

## Boundary Review

This stage is a read-only runtime projection change:

- No provider stream suspension was implemented.
- No core pause signal was added.
- No chat resume envelope was added.
- No background reattach daemon or runtime service was added.
- No checkpoint restore/revert behavior was implemented.
- No tool, MCP, agent, sub-agent, or swarm runtime was added.

## Verification

Passed:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-core --test conversation_engine_contract runtime_reader_projects_paused_and_resumed_task_records -- --nocapture
PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check
PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings
PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace
git diff --check
```

## Next Recommended Stage

After this PR is merged, continue with the next short-to-medium slice:

- add a core pause signal path that can record a cooperative `task_paused` event for active runs;
- keep provider suspension, background reattach, resume envelopes, and checkpoint restore as later phases unless scope changes.

## Notes For Next Session

- `output/` remains a pre-existing untracked directory and should be ignored unless explicitly requested.
- The implementation intentionally keeps JSONL trace as the source of truth and SQLite indexes rebuildable.
