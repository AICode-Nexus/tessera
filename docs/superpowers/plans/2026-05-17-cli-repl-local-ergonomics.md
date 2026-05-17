# CLI REPL Local Ergonomics Implementation Plan

**Goal:** Add `/commands`, `/history`, and `/clear` to the interactive CLI REPL.

**Architecture:** Keep the commands local to `tessera-cli` and `tessera-client` projection state; do not touch provider, core, protocol, or storage boundaries.

**Tech Stack:** Rust, existing Tessera CLI contract tests.

---

## Chunk 1: Contracts

### Task 1: Add RED tests

**Files:**
- Modify: `crates/cli/tests/cli_contract.rs`

- [x] Add parser coverage for `/commands`, `/clear`, and `/history`.
- [x] Add command-discovery coverage for the new commands.
- [x] Add session behavior coverage for empty history, non-empty history, and clear.
- [x] Verify RED before implementation.

## Chunk 2: Implementation

### Task 2: Implement local commands

**Files:**
- Modify: `crates/cli/src/lib.rs`

- [x] Add `CliReplCommand::Clear` and `CliReplCommand::History`.
- [x] Parse `/commands` as help.
- [x] Parse `/clear` and `/history`.
- [x] Implement `/clear` by resetting the visible `ClientSnapshot`.
- [x] Implement `/history` as compact read-only formatting over projected messages.
- [x] Add the commands to shared command discovery.

## Chunk 3: Docs And Verification

### Task 3: Update docs and gates

**Files:**
- Modify: `README.md`
- Modify: `crates/cli/README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/global-plan.md`

- [x] Document the new REPL commands.
- [x] Run CLI smoke for `/history` and `/clear`.
- [x] Run workspace fmt, clippy, tests, and `git diff --check`.
