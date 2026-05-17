# CLI REPL Active Cancel Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let interactive `tessera chat` process `/cancel` while a provider run is still streaming.

**Architecture:** Keep core cancellation ownership in `RunCancellationToken`. The CLI REPL reads input lines on a bounded async path, races active runs against incoming slash commands, and only treats `/cancel` as an active-run control while buffering other lines for later processing.

**Tech Stack:** Rust, Tokio `mpsc`, existing `tessera-core` cancellation controls, deterministic slow mock model for contract testing.

---

### Task 1: RED Test

**Files:**
- Modify: `crates/cli/tests/cli_contract.rs`

- [x] Add a contract test where REPL starts a slow mock run, receives `/cancel`, and records `task_cancelled` without `task_completed`.
- [x] Run the targeted test and verify it fails before implementation.

### Task 2: Async REPL Input And Cancel

**Files:**
- Modify: `crates/cli/src/lib.rs`
- Modify: `crates/providers/src/lib.rs`
- Modify: `crates/providers/Cargo.toml`

- [x] Add a background line reader for blocking `BufRead` input.
- [x] Race active prompt execution against incoming lines and assistant deltas.
- [x] Cancel the active `RunCancellationToken` when `/cancel` arrives during a run.
- [x] Buffer non-cancel input typed during a run for later prompt-loop processing.
- [x] Add deterministic `mock-slow` stream behavior for cancellation tests.

### Task 3: Docs And Gates

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `crates/cli/README.md`
- Modify: `crates/providers/README.md`
- Modify: `docs/global-plan.md`

- [x] Update docs to describe active-run `/cancel`.
- [x] Run targeted REPL tests.
- [x] Run full verification gates.
- [x] Review, commit, and push.
