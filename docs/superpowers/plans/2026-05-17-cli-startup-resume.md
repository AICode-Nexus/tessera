# CLI Startup Resume Implementation Plan

**Goal:** Add `tessera chat --resume <trace_id>` for direct interactive session restore.

**Architecture:** Keep `/resume` and startup resume on the same `CliReplSession` projection path, then reuse provider-neutral history plumbing for follow-up prompts.

**Tech Stack:** Rust, `clap`, existing Tessera CLI/core/client crates.

---

## Chunk 1: Startup Resume Entry

### Task 1: Add CLI-facing contract

**Files:**
- Modify: `crates/cli/tests/cli_contract.rs`
- Modify: `crates/cli/src/lib.rs`
- Modify: `crates/cli/src/main.rs`

- [x] Add RED tests for `chat --help` and startup resume follow-up history.
- [x] Add `run_chat_repl_with_io_and_resume` / config wrapper.
- [x] Add `chat --resume <trace_id>` and route it to the interactive REPL startup path.

## Chunk 2: Docs And Verification

### Task 2: Update docs and gates

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/global-plan.md`

- [x] Document startup resume.
- [x] Run CLI smoke for `chat --resume <trace_id>` followed by a prompt.
- [x] Run workspace fmt, clippy, tests, and `git diff --check`.
