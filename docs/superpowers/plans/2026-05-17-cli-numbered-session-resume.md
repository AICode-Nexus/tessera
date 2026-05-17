# CLI Numbered Session Resume Implementation Plan

**Goal:** Add numbered session lists and numeric resume selectors to CLI chat.

**Architecture:** Keep numbering and selector resolution in `tessera-cli`, while relying on the existing read-only `RuntimeReader::list_sessions` ordering.

**Tech Stack:** Rust, existing Tessera CLI contract tests.

---

## Chunk 1: Contracts

### Task 1: Add RED tests

**Files:**
- Modify: `crates/cli/tests/cli_contract.rs`

- [x] Add `/sessions` numbering assertion.
- [x] Add `/resume 1` behavior test against the current sorted session list.
- [x] Add out-of-range numeric resume error coverage.
- [x] Add top-level `sessions` numbering assertion.
- [x] Verify RED before implementation.

## Chunk 2: Implementation

### Task 2: Implement numbering and selector resolution

**Files:**
- Modify: `crates/cli/src/lib.rs`

- [x] Prefix human-readable session rows with 1-based indexes.
- [x] Resolve numeric resume selectors through `list_sessions`.
- [x] Preserve non-numeric trace ID resume behavior.
- [x] Update command discovery text to `/resume <trace_id|#>`.

## Chunk 3: Docs And Verification

### Task 3: Update docs and gates

**Files:**
- Modify: `README.md`
- Modify: `crates/cli/README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/global-plan.md`

- [x] Document numbered sessions and numeric resume.
- [x] Run CLI smoke for `/sessions` and `/resume 1`.
- [x] Run workspace fmt, clippy, tests, and `git diff --check`.
