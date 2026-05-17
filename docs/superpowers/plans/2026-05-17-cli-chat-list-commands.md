# CLI Chat List Commands Implementation Plan

**Goal:** Add `tessera chat --list-commands` for REPL command discovery without starting runtime work.

**Architecture:** Reuse the REPL help formatter and check `--list-commands` before config/data-dir resolution in the `chat` command.

**Tech Stack:** Rust, `clap`, existing Tessera CLI crate.

---

## Chunk 1: Command Discovery Contract

### Task 1: Add list-commands behavior

**Files:**
- Modify: `crates/cli/tests/cli_contract.rs`
- Modify: `crates/cli/src/lib.rs`
- Modify: `crates/cli/src/main.rs`

- [x] Add RED tests for `chat --help` and `chat --list-commands`.
- [x] Expose shared `chat_command_lines`.
- [x] Add `--list-commands` to `tessera chat`.
- [x] Return command list before config/data-dir resolution.

## Chunk 2: Docs And Verification

### Task 2: Update docs and gates

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `crates/cli/README.md`
- Modify: `docs/global-plan.md`

- [x] Document `chat --list-commands`.
- [x] Run CLI smoke for `chat --list-commands`.
- [x] Run workspace fmt, clippy, tests, and `git diff --check`.
