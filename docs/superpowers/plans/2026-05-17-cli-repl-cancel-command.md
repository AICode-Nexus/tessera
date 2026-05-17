# CLI REPL Cancel Command Implementation Plan

**Goal:** Add a stable `/cancel` command surface to interactive `tessera chat`.

**Architecture:** Keep this as a CLI-local placeholder while preserving existing core cancellation semantics for runtime sinks.

**Tech Stack:** Rust, existing Tessera CLI contract tests.

---

## Chunk 1: Contracts

### Task 1: Add RED tests

**Files:**
- Modify: `crates/cli/tests/cli_contract.rs`

- [x] Add parser coverage for `/cancel`.
- [x] Add command-discovery coverage for `/cancel`.
- [x] Add local command behavior coverage for no active run.
- [x] Verify RED before implementation.

## Chunk 2: Implementation

### Task 2: Implement local cancel command

**Files:**
- Modify: `crates/cli/src/lib.rs`

- [x] Add `CliReplCommand::Cancel`.
- [x] Parse `/cancel`.
- [x] Return a clear no-active-run message in normal REPL mode.
- [x] Keep paste-mode `/cancel` behavior unchanged.
- [x] Add `/cancel` to shared command discovery.

## Chunk 3: Docs And Verification

### Task 3: Update docs and gates

**Files:**
- Modify: `README.md`
- Modify: `crates/cli/README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/global-plan.md`

- [x] Document `/cancel` normal and paste-mode behavior.
- [x] Run CLI smoke for `/cancel`.
- [x] Run workspace fmt, clippy, tests, and `git diff --check`.
