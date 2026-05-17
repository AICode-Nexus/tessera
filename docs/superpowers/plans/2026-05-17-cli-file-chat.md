# CLI File Chat Implementation Plan

**Goal:** Add `tessera chat --file <path>` for file-backed one-shot prompts.

**Architecture:** Read prompt files only in the CLI entrypoint, then reuse the existing one-shot chat path.

**Tech Stack:** Rust, `clap`, existing Tessera CLI/core/provider/storage crates.

---

## Chunk 1: File Prompt Entry

### Task 1: Add file command behavior

**Files:**
- Modify: `crates/cli/tests/cli_contract.rs`
- Modify: `crates/cli/src/main.rs`

- [x] Add RED tests for `chat --help`, file prompt execution, and mutually exclusive prompt sources.
- [x] Add `chat --file <path>`.
- [x] Reject `--file` combined with `--prompt` or `--stdin`; keep `--resume` interactive-only.

## Chunk 2: Docs And Verification

### Task 2: Update docs and gates

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/global-plan.md`

- [x] Document file-backed chat.
- [x] Run CLI smoke for a file prompt.
- [x] Run workspace fmt, clippy, tests, and `git diff --check`.
