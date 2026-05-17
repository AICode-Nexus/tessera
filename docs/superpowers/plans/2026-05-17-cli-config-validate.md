# CLI Config Validate Implementation Plan

**Goal:** Add `tessera config validate [--json] [--config] [--data-dir]` for read-only startup configuration checks.

**Architecture:** Reuse existing CLI config/data-dir resolution; keep validation in `tessera-cli` as a pure DTO/report layer that does not open storage or execute providers.

**Tech Stack:** Rust, `clap`, `serde`, existing Tessera CLI/config crates.

---

## Chunk 1: Config Validate Contract

### Task 1: Add validate command behavior

**Files:**
- Modify: `crates/cli/tests/cli_contract.rs`
- Modify: `crates/cli/src/lib.rs`
- Modify: `crates/cli/src/main.rs`

- [x] Add RED tests for `config validate --help`, ok text/JSON output, missing secret env, and provider shape errors.
- [x] Add validation report DTOs.
- [x] Add `validate_config` and `format_config_validation_lines`.
- [x] Add nested `tessera config validate [--json] [--config] [--data-dir]`.

## Chunk 2: Docs And Verification

### Task 2: Update docs and gates

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `crates/cli/README.md`
- Modify: `docs/global-plan.md`

- [x] Document config validate command.
- [x] Run CLI smoke for valid and invalid config validation.
- [x] Run workspace fmt, clippy, tests, and `git diff --check`.
