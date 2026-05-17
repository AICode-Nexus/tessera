# CLI REPL Paste Mode Implementation Plan

**Goal:** Add multiline prompt collection to interactive `tessera chat`.

**Architecture:** Keep paste collection inside the CLI REPL input loop and submit through the existing prompt writer.

**Tech Stack:** Rust, existing Tessera CLI contract tests.

---

## Chunk 1: Contracts

### Task 1: Add RED tests

**Files:**
- Modify: `crates/cli/tests/cli_contract.rs`

- [x] Add parser coverage for `/paste`.
- [x] Add command-discovery coverage for `/paste`.
- [x] Add REPL flow coverage for multiline `/paste` + `/send`.
- [x] Add REPL flow coverage for `/paste` + `/cancel`.
- [x] Verify RED before implementation.

## Chunk 2: Implementation

### Task 2: Implement paste mode

**Files:**
- Modify: `crates/cli/src/lib.rs`

- [x] Add `CliReplCommand::Paste`.
- [x] Parse `/paste`.
- [x] Add paste collection state to the interactive loop.
- [x] Submit collected multiline prompt through the existing prompt writer.
- [x] Keep `/cancel` local and non-mutating.
- [x] Add `/paste` to shared command discovery.

## Chunk 3: Docs And Verification

### Task 3: Update docs and gates

**Files:**
- Modify: `README.md`
- Modify: `crates/cli/README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/global-plan.md`

- [x] Document `/paste`, `/send`, and `/cancel`.
- [x] Run CLI smoke for paste send/cancel.
- [x] Run workspace fmt, clippy, tests, and `git diff --check`.
