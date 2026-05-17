# CLI Replay Command Implementation Plan

**Goal:** Add `tessera replay <trace_id> [--json]` for provider-free trace replay summaries.

**Architecture:** Keep replay logic in `tessera-core::ReplayRunner`; expose a small CLI DTO and formatter in `tessera-cli`.

**Tech Stack:** Rust, `clap`, `serde`, existing Tessera CLI/core/storage crates.

---

## Chunk 1: Replay Command Contract

### Task 1: Add replay command behavior

**Files:**
- Modify: `crates/cli/tests/cli_contract.rs`
- Modify: `crates/cli/src/lib.rs`
- Modify: `crates/cli/src/main.rs`

- [x] Add RED tests for `replay --help`, text output, and JSON output.
- [x] Add `CliReplaySummary`.
- [x] Add `replay_trace` and `format_replay_summary`.
- [x] Add `tessera replay <trace_id> [--json] [--config] [--data-dir]`.

## Chunk 2: Docs And Verification

### Task 2: Update docs and gates

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `crates/cli/README.md`
- Modify: `docs/global-plan.md`

- [x] Document replay command.
- [x] Run CLI smoke for replay text and JSON.
- [x] Run workspace fmt, clippy, tests, and `git diff --check`.
