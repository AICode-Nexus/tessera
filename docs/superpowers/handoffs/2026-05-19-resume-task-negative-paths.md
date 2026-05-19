# Tessera Resume Task Negative Paths Handoff

Date: 2026-05-19

## Branch

`codex/resume-task-negative-paths`

## Completed

- Started from `main` after PR #12.
- Added a CLI contract test proving a paused chat task cannot be resumed twice from the same checkpoint.
- Added a `/resume-task` preflight guard that reads the original trace task projection and requires the task status to still be `Paused`.
- Added explicit user-facing errors for non-paused checkpoint tasks, including the current status label.
- Updated `CHANGELOG.md` and `docs/global-plan.md`.

## TDD Notes

- RED: `repl_resume_task_rejects_task_that_is_no_longer_paused` first failed because the same checkpoint started two resume chat runs.
- GREEN: the second `/resume-task` now returns an error such as `task <id> is not paused (current status: running)` and does not start another chat run.

## Boundary Review

This stage only hardens chat task resume:

- No provider socket freezing was implemented.
- No background runtime daemon or reattach service was added.
- No workspace checkpoint restore/revert behavior was implemented.
- No tool, MCP, agent, sub-agent, swarm, or non-chat task resume runtime was added.
- The guard is trace-backed: JSONL remains the event truth and SQLite remains rebuildable index state.

## Verification

Targeted gate passed:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-cli --test cli_contract repl_resume_task_rejects_task_that_is_no_longer_paused -- --nocapture
```

Full verification passed:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check
PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings
PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace
git diff --check
```

## Next Recommended Stage

Continue short-line hardening with explicit missing-checkpoint and provider-profile negative-path tests, then add a read-only list of resumable paused tasks for CLI users.

## Notes For Next Session

- `output/` remains a pre-existing untracked directory and should be ignored unless explicitly requested.
- `/resume <trace_id|#>` remains session projection; `/resume-task <task_id>` remains chat-only task resume from pause checkpoint.
