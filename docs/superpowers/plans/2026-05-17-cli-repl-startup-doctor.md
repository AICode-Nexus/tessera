# CLI REPL Startup Doctor Implementation Plan

**Goal:** Add startup runtime context and a `/doctor` command to interactive `tessera chat`.

**Architecture:** Keep the feature in `tessera-cli`; reuse existing doctor reporting and shared command-list formatting without changing core/provider/storage boundaries.

**Tech Stack:** Rust, existing Tessera CLI contract tests.

---

## Chunk 1: REPL Contracts

### Task 1: Add failing tests

**Files:**
- Modify: `crates/cli/tests/cli_contract.rs`

- [x] Add parser coverage for `/doctor`.
- [x] Add REPL command coverage for `/doctor` runtime health output.
- [x] Add REPL startup output coverage for active profile, data dir, available profiles, and command hints.
- [x] Verify RED before implementation.

## Chunk 2: CLI Implementation

### Task 2: Wire REPL startup and doctor

**Files:**
- Modify: `crates/cli/src/lib.rs`

- [x] Add `CliReplCommand::Doctor`.
- [x] Parse `/doctor`.
- [x] Handle `/doctor` through `handle_command_with_data_dir`.
- [x] Reuse `run_doctor_with_config` and `format_doctor_lines`.
- [x] Add startup context formatter and print it before the first prompt.
- [x] Add `/doctor` to shared `chat_command_lines`.

## Chunk 3: Docs And Verification

### Task 3: Update docs and gates

**Files:**
- Modify: `README.md`
- Modify: `crates/cli/README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/global-plan.md`

- [x] Document startup context and `/doctor`.
- [x] Run REPL smoke with `/doctor`.
- [x] Run workspace fmt, clippy, tests, and `git diff --check`.
