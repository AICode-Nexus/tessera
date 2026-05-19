# Tessera Manual Test Readiness Handoff

Date: 2026-05-19

## Branch

`codex/manual-test-readiness`

## Completed

- Started from `main` after PR #16.
- Ran a local mock-only smoke for pause, `/resume-tasks`, `/resume-task 1`, and `/sessions` using a temporary `mock-slow` config.
- Added `docs/manual-testing-v0.1.md` with deterministic manual test steps, expected markers, trace inspection, negative checks, cleanup, and scope notes.
- Linked the manual testing guide from `docs/v0.1-release-checklist.md`.
- Updated `CHANGELOG.md` and `docs/global-plan.md`.

## Smoke Notes

The local smoke command used a temporary config with:

```toml
[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-slow"
```

The scripted REPL input was:

```bash
printf 'pause this slow run\n/pause\n/resume-tasks\n/resume-task 1\n/sessions\n/quit\n' \
  | ./target/debug/tessera chat --provider offline --config "$TEST_ROOT/tessera.toml"
```

Observed markers included `pause requested`, a numbered resumable checkpoint, `resuming task ...`, an assistant resume response with `(history messages: 2)`, and two trace-backed sessions.

## Boundary Review

This stage only prepares manual validation:

- No provider socket freezing was implemented.
- No background runtime daemon or reattach service was added.
- No workspace checkpoint restore/revert behavior was implemented.
- No tool, MCP, agent, sub-agent, swarm, or non-chat task resume runtime was added.
- The manual path uses a temporary mock config and does not require provider secrets.

## Verification

Manual smoke passed:

```bash
printf 'pause this slow run\n/pause\n/resume-tasks\n/resume-task 1\n/sessions\n/quit\n' \
  | ./target/debug/tessera chat --provider offline --config "$TEST_ROOT/tessera.toml"
```

Documentation gate passed:

```bash
git diff --check
```

Full verification passed:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check
PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings
PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace
```

## Next Recommended Stage

Use `docs/manual-testing-v0.1.md` for hands-on validation. After manual testing, move into the first medium-line background reattach design slice if the current chat-only resume behavior is acceptable.

## Notes For Next Session

- `output/` remains a pre-existing untracked directory and should be ignored unless explicitly requested.
- Manual testing should use temporary data dirs first, then move to normal data dirs only after the mock path behaves as expected.
