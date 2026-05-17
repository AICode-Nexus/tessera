# CLI Doctor Text Implementation Plan

**Goal:** Make `tessera doctor` print useful runtime health details while preserving `doctor --json`.

**Architecture:** Reuse the existing `DoctorReport`; add a CLI text formatter and wire the non-JSON command path to it.

**Tech Stack:** Rust, `clap`, `serde`, existing Tessera CLI/storage/config crates.

---

## Chunk 1: Doctor Text Contract

### Task 1: Add detailed text output

**Files:**
- Modify: `crates/cli/tests/cli_contract.rs`
- Modify: `crates/cli/src/lib.rs`
- Modify: `crates/cli/src/main.rs`

- [x] Add RED test for detailed `doctor` text output.
- [x] Add `format_doctor_lines`.
- [x] Use `format_doctor_lines` for non-JSON doctor output.

## Chunk 2: Docs And Verification

### Task 2: Update docs and gates

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `crates/cli/README.md`
- Modify: `docs/global-plan.md`

- [x] Document detailed doctor text output.
- [x] Run CLI smoke for doctor text and JSON output.
- [x] Run workspace fmt, clippy, tests, and `git diff --check`.
