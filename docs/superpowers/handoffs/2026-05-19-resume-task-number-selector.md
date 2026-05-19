# Tessera Resume Task Number Selector Handoff

Date: 2026-05-19

## Branch

`codex/resume-task-number-selector`

## Completed

- Started from `main` after PR #15.
- Added `/resume-task <number|#number>` support, resolving numbers against the same read-only ordering shown by `/resume-tasks`.
- Added CLI contract coverage for numbered resume success and out-of-range selector failure.
- Updated `/help` / `chat --list-commands` text, `CHANGELOG.md`, and `docs/global-plan.md`.

## TDD Notes

- RED: `repl_resume_task_accepts_numbered_resume_task_selector` first failed because `/resume-task 1` was treated as a literal task id and never started resume.
- GREEN: numbered selectors now resolve through `list_resumable_pause_checkpoints`; out-of-range selectors report the available count without projecting old trace records or starting runtime work.

## Boundary Review

This stage only improves CLI selection ergonomics:

- No provider socket freezing was implemented.
- No background runtime daemon or reattach service was added.
- No workspace checkpoint restore/revert behavior was implemented.
- No tool, MCP, agent, sub-agent, swarm, or non-chat task resume runtime was added.
- Number resolution is read-only and uses JSONL trace projection through `RuntimeReader`.

## Verification

Targeted gate passed:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-cli --test cli_contract repl_resume_task_accepts_numbered_resume_task_selector -- --nocapture
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-cli --test cli_contract repl_resume_task_rejects_out_of_range_numbered_selector_without_runtime_work -- --nocapture
```

Full verification passed:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check
PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings
PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace
git diff --check
```

## Next Recommended Stage

Move from short-line CLI ergonomics into the first medium-line design slice for background reattach state, while keeping real provider suspension and workspace restore behind explicit staged gates.

## Notes For Next Session

- `output/` remains a pre-existing untracked directory and should be ignored unless explicitly requested.
- `/resume-task <task_id>` still supports direct task ids.
- `/resume-task <number|#number>` only selects currently resumable checkpoints visible through `/resume-tasks`.
