# CLI Events Command Implementation Plan

**Goal:** Add `tessera events <trace_id> [--json] [--since <seq>] [--limit <n>]` for read-only trace event inspection.

**Architecture:** Reuse `tessera-core::RuntimeReader::list_events`; expose a small CLI DTO and text formatter in `tessera-cli`.

**Tech Stack:** Rust, `clap`, `serde`, existing Tessera CLI/core/storage/protocol crates.

---

## Chunk 1: Events Command Contract

### Task 1: Add events command behavior

**Files:**
- Modify: `crates/cli/tests/cli_contract.rs`
- Modify: `crates/cli/src/lib.rs`
- Modify: `crates/cli/src/main.rs`

- [x] Add RED tests for `events --help`, text output, JSON output, and pagination.
- [x] Add `CliEventPage`.
- [x] Add `list_events` and `format_event_lines`.
- [x] Add `tessera events <trace_id> [--json] [--since <seq>] [--limit <n>] [--config] [--data-dir]`.

## Chunk 2: Docs And Verification

### Task 2: Update docs and gates

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `crates/cli/README.md`
- Modify: `docs/global-plan.md`

- [x] Document events command.
- [x] Run CLI smoke for events text and JSON pagination.
- [x] Run workspace fmt, clippy, tests, and `git diff --check`.
