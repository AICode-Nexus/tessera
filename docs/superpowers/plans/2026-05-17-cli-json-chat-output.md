# CLI JSON Chat Output Implementation Plan

**Goal:** Add `tessera chat --json` for script-friendly one-shot chat output.

**Architecture:** Keep JSON formatting in the CLI entrypoint by converting `ConversationOutcome` into a small serializable DTO.

**Tech Stack:** Rust, `clap`, `serde`, existing Tessera CLI/core/provider/storage crates.

---

## Chunk 1: JSON Output Contract

### Task 1: Add JSON command behavior

**Files:**
- Modify: `crates/cli/tests/cli_contract.rs`
- Modify: `crates/cli/src/lib.rs`
- Modify: `crates/cli/src/main.rs`

- [x] Add RED tests for `chat --help`, one-shot JSON stdout, and rejecting JSON without a prompt source.
- [x] Add `CliChatOutput`.
- [x] Add `chat --json` for one-shot prompt sources.
- [x] Reject `--json` in interactive chat mode.

## Chunk 2: Docs And Verification

### Task 2: Update docs and gates

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `crates/cli/README.md`
- Modify: `docs/global-plan.md`

- [x] Document JSON chat output.
- [x] Run CLI smoke for one-shot JSON output.
- [x] Run workspace fmt, clippy, tests, and `git diff --check`.
