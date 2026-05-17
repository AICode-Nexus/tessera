# CLI Runtime v2 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add config initialization, session listing, and trace resume to the interactive CLI.

**Architecture:** Keep user-facing orchestration in `crates/cli`, expose trace session summaries through `tessera-core::RuntimeReader`, and reuse `tessera-client::ClientSnapshot` for resume projection.

**Tech Stack:** Rust, Clap, Tokio, existing Tessera config/core/storage/client crates.

---

## Chunk 1: Safe Config Init

### Task 1: Add `tessera init`

**Files:**
- Modify: `crates/cli/src/lib.rs`
- Modify: `crates/cli/src/main.rs`
- Modify: `crates/cli/tests/cli_contract.rs`

- [x] Add RED tests for writing a config template and refusing overwrite without `force`.
- [x] Implement `default_config_template` and `write_config_template`.
- [x] Add the `init` Clap command with `--config` and `--force`.
- [x] Run focused CLI tests.

## Chunk 2: Runtime Session Summaries

### Task 2: Add read-only session listing

**Files:**
- Modify: `crates/storage/src/lib.rs`
- Modify: `crates/storage/tests/trace_store_contract.rs`
- Modify: `crates/core/src/lib.rs`
- Modify: `crates/core/tests/conversation_engine_contract.rs`

- [x] Add RED storage/core tests for listing trace IDs and runtime session summaries.
- [x] Implement public storage trace ID listing by scanning the trace directory.
- [x] Implement `RuntimeSessionSummary` and `RuntimeReader::list_sessions`.
- [x] Run focused storage/core tests.

## Chunk 3: REPL `/sessions` And `/resume`

### Task 3: Connect CLI REPL commands

**Files:**
- Modify: `crates/cli/src/lib.rs`
- Modify: `crates/cli/tests/cli_contract.rs`
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/global-plan.md`

- [x] Add RED parser/session tests for `/sessions` and `/resume <trace_id>`.
- [x] Implement REPL command handling through `RuntimeReader`.
- [x] Update help text and docs.
- [x] Run CLI smoke with `/sessions`, prompt, `/resume`, `/export`, `/quit`.
- [x] Run workspace fmt, clippy, tests, and `git diff --check`.
