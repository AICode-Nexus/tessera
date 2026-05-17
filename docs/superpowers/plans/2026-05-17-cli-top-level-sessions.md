# CLI Top-Level Sessions Implementation Plan

**Goal:** Add `tessera sessions` for REPL-free trace-backed session discovery.

**Architecture:** Reuse `RuntimeReader::list_sessions` and the REPL session formatting path through a CLI DTO.

**Tech Stack:** Rust, `clap`, existing Tessera CLI/core/storage crates.

---

## Chunk 1: Command Contract

### Task 1: Add sessions command behavior

**Files:**
- Modify: `crates/cli/tests/cli_contract.rs`
- Modify: `crates/cli/src/lib.rs`
- Modify: `crates/cli/src/main.rs`

- [x] Add RED tests for `sessions --help`, text output, and JSON output.
- [x] Add `CliSessionSummary`, `list_sessions`, and shared text formatter.
- [x] Add `tessera sessions [--json] [--config] [--data-dir]`.

## Chunk 2: Docs And Verification

### Task 2: Update docs and gates

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/global-plan.md`

- [x] Document top-level sessions command.
- [x] Run CLI smoke for `chat --prompt` followed by `sessions` text and JSON.
- [x] Run workspace fmt, clippy, tests, and `git diff --check`.
