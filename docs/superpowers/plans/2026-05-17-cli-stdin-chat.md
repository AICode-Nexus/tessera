# CLI Stdin Chat Implementation Plan

**Goal:** Add `tessera chat --stdin` for pipe-friendly one-shot prompts.

**Architecture:** Read stdin only in the CLI entrypoint, then reuse the existing one-shot chat path.

**Tech Stack:** Rust, `clap`, existing Tessera CLI/core/provider/storage crates.

---

## Chunk 1: Stdin Prompt Entry

### Task 1: Add stdin command behavior

**Files:**
- Modify: `crates/cli/tests/cli_contract.rs`
- Modify: `crates/cli/src/main.rs`

- [x] Add RED tests for `chat --help` and piped stdin execution.
- [x] Add `chat --stdin`.
- [x] Reject `--stdin` combined with `--prompt`; keep `--resume` interactive-only.

## Chunk 2: Docs And Verification

### Task 2: Update docs and gates

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/global-plan.md`

- [x] Document stdin chat.
- [x] Run CLI smoke for piped stdin prompt.
- [x] Run workspace fmt, clippy, tests, and `git diff --check`.
