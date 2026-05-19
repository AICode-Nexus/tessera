# Top-Level Paused Task CLI Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add script-friendly top-level CLI access for paused chat task discovery and chat-only checkpoint resume without requiring the interactive REPL.

**Architecture:** Reuse the existing trace-first pause/resume path already implemented for `/resume-tasks` and `/resume-task <task_id|#>`. The CLI remains an entry point: it resolves config/data-dir, calls `tessera-cli` helpers, and does not bypass core, storage, or provider boundaries. The new surface must not implement provider socket freezing, background reattach, workspace restore, tools, agents, MCP, or swarm behavior.

**Tech Stack:** Rust workspace, `clap` CLI parsing, `tessera-core::RuntimeReader`, `tessera-storage::TraceStore`, existing `tessera-cli` REPL resume helpers, contract tests in `crates/cli/tests/cli_contract.rs`.

---

## File Structure

- Modify `crates/cli/src/main.rs`
  - Add top-level `Tasks` command with `--json`, `--config`, and `--data-dir`.
  - Add `--resume-task <task_id|#>` to `chat`.
  - Enforce argument compatibility with existing `--prompt`, `--stdin`, `--file`, `--json`, `--continue`, and `--resume`.
  - Route top-level task resume through `tessera_cli::resume_task_with_config`.

- Modify `crates/cli/src/lib.rs`
  - Expose a serializable paused task summary DTO for top-level output.
  - Expose `list_resumable_tasks` and `format_resumable_task_lines` wrappers around the existing checkpoint projection.
  - Extract REPL resume execution into a reusable `resume_task_with_config` helper that writes progress to any `Write`.
  - Keep the existing REPL behavior on the same helper.

- Modify `crates/cli/tests/cli_contract.rs`
  - Add unit/contract tests for formatter and JSON shape.
  - Add async tests for top-level `chat --resume-task <task_id>` and `chat --resume-task 1`.
  - Add negative tests for incompatible args and out-of-range selectors.

- Modify `README.md`
  - Document `tessera tasks` and `chat --resume-task`.

- Modify `docs/manual-testing-v0.1.md`
  - Add mock-only top-level task list/resume smoke path.

- Modify `docs/global-plan.md`
  - Mark the new top-level paused task CLI as completed in the pause/resume sequence.

- Modify `CHANGELOG.md`
  - Add an `Unreleased` bullet for top-level paused task discovery and chat-only resume.

---

## Chunk 1: Top-Level Paused Task Listing

### Task 1: Add CLI DTO and formatter

**Files:**
- Modify: `crates/cli/src/lib.rs`
- Test: `crates/cli/tests/cli_contract.rs`

- [ ] **Step 1: Write failing tests**
  - Add a test that creates a paused mock chat task and calls `list_resumable_tasks`.
  - Assert the returned DTO contains `task_id`, `trace_id`, `provider_id`, `checkpoint_id`, `resume_mode`, and `reason`.
  - Assert `format_resumable_task_lines` matches the existing `/resume-tasks` numbered text format.

- [ ] **Step 2: Run targeted test and verify RED**
  - Run: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-cli --test cli_contract top_level_tasks_command_lists_resumable_paused_checkpoints_without_runtime_work -- --nocapture`
  - Expected: FAIL because the public helper/DTO does not exist yet.

- [ ] **Step 3: Implement minimal helper**
  - Add `CliResumableTaskSummary`.
  - Convert from `RuntimePauseCheckpointSummary`.
  - Rename or wrap internal `list_resumable_pause_checkpoints` / `format_resume_task_lines` as public CLI-facing helpers while preserving existing REPL behavior.

- [ ] **Step 4: Run targeted test and verify GREEN**
  - Run the same targeted test.
  - Expected: PASS.

### Task 2: Add top-level `tessera tasks`

**Files:**
- Modify: `crates/cli/src/main.rs`
- Test: `crates/cli/tests/cli_contract.rs`

- [ ] **Step 1: Write failing help/command tests**
  - Add a help test asserting `tessera tasks --help` lists `--json`, `--config`, and `--data-dir`.
  - Add command-level tests for text output and JSON output over a mock paused trace.

- [ ] **Step 2: Run targeted tests and verify RED**
  - Run: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-cli --test cli_contract tasks_help_lists_json_config_and_data_options top_level_tasks_command_lists_resumable_paused_checkpoints_without_runtime_work top_level_tasks_command_emits_json_for_resumable_paused_checkpoints -- --nocapture`
  - Expected: FAIL because the `tasks` subcommand does not exist.

- [ ] **Step 3: Implement minimal command**
  - Add `Commands::Tasks`.
  - Resolve config/data-dir using existing helpers.
  - Print text via `format_resumable_task_lines`.
  - Print JSON via `serde_json::to_string_pretty`.

- [ ] **Step 4: Run targeted tests and verify GREEN**
  - Run the same targeted tests.
  - Expected: PASS.

---

## Chunk 2: Top-Level Chat Resume Task

### Task 3: Extract reusable resume helper

**Files:**
- Modify: `crates/cli/src/lib.rs`
- Test: `crates/cli/tests/cli_contract.rs`

- [ ] **Step 1: Write failing library-level resume test if needed**
  - Prefer command-level coverage if it exercises the public helper end to end.

- [ ] **Step 2: Extract implementation**
  - Add `pub async fn resume_task_with_config<W>(data_dir, config, provider, task_selector, output)`.
  - Reuse `resolve_resume_task_checkpoint`, provider preflight, paused guard, trace projection, `run_chat_with_config`, and `RuntimeTaskResumer`.
  - Keep REPL-specific session projection inside REPL path; top-level one-shot resume prints progress and assistant text but does not need interactive pending input support.

- [ ] **Step 3: Keep REPL on same checkpoint/guard path**
  - Update `resume_repl_task_and_write` to call shared resolution/guard helpers or the reusable helper where possible.
  - Preserve existing REPL test expectations.

### Task 4: Add `chat --resume-task <task_id|#>`

**Files:**
- Modify: `crates/cli/src/main.rs`
- Test: `crates/cli/tests/cli_contract.rs`

- [ ] **Step 1: Write failing command tests**
  - Add a test that pauses a mock task, runs `run_chat_command` or an equivalent CLI helper with `--resume-task <task_id>`, and asserts it writes `task_resumed` to the source trace.
  - Add a numbered selector test with `--resume-task 1`.
  - Add negative compatibility tests for `--resume-task` combined with prompt sources, `--resume`, `--continue`, or `--json`.

- [ ] **Step 2: Run targeted tests and verify RED**
  - Run the new targeted tests.
  - Expected: FAIL because `--resume-task` is not parsed/routed.

- [ ] **Step 3: Implement minimal command route**
  - Add `resume_task: Option<String>` to `Chat`.
  - Reject incompatible combinations with clear errors.
  - Call `tessera_cli::resume_task_with_config`.
  - Keep normal one-shot chat and interactive session behavior unchanged.

- [ ] **Step 4: Run targeted tests and verify GREEN**
  - Run the new targeted tests.
  - Expected: PASS.

---

## Chunk 3: Docs And Roadmap

### Task 5: Update user-facing docs and roadmap

**Files:**
- Modify: `README.md`
- Modify: `docs/manual-testing-v0.1.md`
- Modify: `docs/global-plan.md`
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Update docs**
  - Document `tessera tasks [--json]`.
  - Document `tessera chat --resume-task <task_id|#>`.
  - State that this is chat-only trace projection resume, not provider socket freezing or workspace restore.

- [ ] **Step 2: Run doc diff check**
  - Run: `git diff --check`
  - Expected: PASS.

---

## Chunk 4: Full Verification

### Task 6: Run quality gates

**Files:**
- No new files beyond implementation/docs.

- [ ] **Step 1: Format**
  - Run: `PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check`
  - Expected: PASS.

- [ ] **Step 2: Clippy**
  - Run: `PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings`
  - Expected: PASS.

- [ ] **Step 3: Workspace tests**
  - Run: `PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace`
  - Expected: PASS.

- [ ] **Step 4: Diff check**
  - Run: `git diff --check`
  - Expected: PASS.

- [ ] **Step 5: Review final diff**
  - Run: `git status --short` and `git diff --stat`
  - Expected: only intended implementation, docs, changelog, and plan files changed.
