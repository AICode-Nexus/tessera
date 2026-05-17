# CLI Profiles Command Implementation Plan

**Goal:** Add `tessera profiles [--json] [--config]` for secret-safe provider profile inspection.

**Architecture:** Reuse existing CLI config resolution; expose a small CLI DTO and text formatter in `tessera-cli`.

**Tech Stack:** Rust, `clap`, `serde`, existing Tessera CLI/config crates.

---

## Chunk 1: Profiles Command Contract

### Task 1: Add profiles command behavior

**Files:**
- Modify: `crates/cli/tests/cli_contract.rs`
- Modify: `crates/cli/src/lib.rs`
- Modify: `crates/cli/src/main.rs`

- [x] Add RED tests for `profiles --help`, text output, JSON output, and secret redaction.
- [x] Add `CliProviderProfile`.
- [x] Add `list_profiles` and `format_profile_lines`.
- [x] Add `tessera profiles [--json] [--config]`.

## Chunk 2: Docs And Verification

### Task 2: Update docs and gates

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `crates/cli/README.md`
- Modify: `docs/global-plan.md`

- [x] Document profiles command.
- [x] Run CLI smoke for profiles text and JSON output.
- [x] Run workspace fmt, clippy, tests, and `git diff --check`.
