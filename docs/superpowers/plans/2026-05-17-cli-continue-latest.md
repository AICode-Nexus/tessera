# CLI Continue Latest Implementation Plan

**Goal:** Add `tessera chat --continue` to resume the most recent trace-backed session.

**Architecture:** Resolve the latest trace ID through the existing read-only session list, then reuse the `--resume` interactive REPL startup path.

**Tech Stack:** Rust, `clap`, existing Tessera CLI/core/storage/client crates.

---

## Chunk 1: Continue Latest Contract

### Task 1: Add continue command behavior

**Files:**
- Modify: `crates/cli/tests/cli_contract.rs`
- Modify: `crates/cli/src/lib.rs`
- Modify: `crates/cli/src/main.rs`

- [x] Add RED tests for `chat --help`, latest-session continuation, missing sessions, and prompt-source conflicts.
- [x] Add `latest_session_trace_id`.
- [x] Add `chat --continue`.
- [x] Route `--continue` through the existing `run_chat_repl_with_config_and_resume` path.
- [x] Reject `--continue` with `--prompt`, `--stdin`, `--file`, or `--resume`.

## Chunk 2: Docs And Verification

### Task 2: Update docs and gates

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `crates/cli/README.md`
- Modify: `docs/global-plan.md`

- [x] Document continue-latest chat startup.
- [x] Run CLI smoke for `chat --continue`.
- [x] Run workspace fmt, clippy, tests, and `git diff --check`.
