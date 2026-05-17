# CLI Transcript Command Implementation Plan

**Goal:** Add `tessera transcript <trace_id>` for REPL-free transcript inspection.

**Architecture:** Reuse `RuntimeReader` and `ClientSnapshot` projection, then export markdown or a small JSON DTO.

**Tech Stack:** Rust, `clap`, existing Tessera CLI/core/client crates.

---

## Chunk 1: Command Contract

### Task 1: Add transcript command behavior

**Files:**
- Modify: `crates/cli/tests/cli_contract.rs`
- Modify: `crates/cli/src/lib.rs`
- Modify: `crates/cli/src/main.rs`

- [x] Add RED tests for `transcript --help`, markdown output, and JSON output.
- [x] Add `CliTranscript` and trace-to-snapshot loading helpers.
- [x] Add `tessera transcript <trace_id> [--json] [--config] [--data-dir]`.

## Chunk 2: Docs And Verification

### Task 2: Update docs and gates

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/global-plan.md`

- [x] Document top-level transcript command.
- [x] Run CLI smoke for `chat --prompt` followed by `transcript` markdown and JSON.
- [x] Run workspace fmt, clippy, tests, and `git diff --check`.
