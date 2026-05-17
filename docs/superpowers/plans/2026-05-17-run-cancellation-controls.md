# Run Cancellation Controls Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a provider-neutral cancellation control path that can interrupt headless runs and be surfaced by CLI/TUI/client intents without introducing tool execution.

**Architecture:** `tessera-core` owns cancellation semantics through `RunControls`; CLI passes controls through the same provider/core path; `tessera-client` and `tessera-tui` expose UI-neutral cancel intent foundations. The interactive CLI `/cancel` command remains line-oriented until a later async input loop wires it to an active run handle.

**Tech Stack:** Rust, Tokio, existing `tessera-core` run loop, existing `tessera-client` projection, existing CLI/TUI contract tests.

---

### Task 1: Core Cancellation Token

**Files:**
- Modify: `crates/core/src/lib.rs`
- Test: `crates/core/tests/conversation_engine_contract.rs`

- [x] **Step 1: Write failing tests**

Add tests proving a `RunCancellationToken` interrupts a stalled provider stream and writes `task_cancelled` / `done` without `task_completed`.

- [x] **Step 2: Run the targeted test**

Run: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-core --test conversation_engine_contract cancellation_token -- --nocapture`

- [x] **Step 3: Implement minimal token support**

Add a cloneable cancellation token to `RunControls` and check it before provider setup plus during provider stream iteration.

- [x] **Step 4: Re-run targeted test**

Run the same command and confirm it passes.

### Task 2: CLI Controls Pass-Through

**Files:**
- Modify: `crates/cli/src/lib.rs`
- Test: `crates/cli/tests/cli_contract.rs`

- [x] **Step 1: Write failing test**

Add a CLI contract test proving pre-cancelled `RunControls` reach the core path and produce a cancelled trace.

- [x] **Step 2: Run targeted CLI tests**

Run: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-cli --test cli_contract cancel -- --nocapture`

- [x] **Step 3: Implement pass-through helpers**

Add controls-aware variants of chat and REPL prompt helpers while preserving existing public helper behavior through defaults.

- [x] **Step 4: Re-run targeted CLI tests**

Run the same command and confirm it passes.

### Task 3: Shared UI Cancel Intent

**Files:**
- Modify: `crates/client/src/lib.rs`
- Modify: `crates/tui/src/lib.rs`
- Test: `crates/client/tests/client_contract.rs`
- Test: `crates/tui/tests/tui_contract.rs`

- [x] **Step 1: Write failing tests**

Add tests for `/cancel` producing `ClientIntent::CancelTask` and Ctrl-C dispatching cancel while a task is running but quitting when idle.

- [x] **Step 2: Run targeted client/TUI tests**

Run:
`PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-client --test client_contract cancel -- --nocapture`
`PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-tui --test tui_contract cancel -- --nocapture`

- [x] **Step 3: Implement minimal shared intent support**

Add active cancellable task lookup in `ClientSnapshot`; map `/cancel` and TUI interrupt input to `CancelTask`.

- [x] **Step 4: Re-run targeted tests**

Run both commands and confirm they pass.

### Task 4: Docs And Gates

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/global-plan.md`
- Modify: `crates/core/README.md`
- Modify: `crates/cli/README.md`

- [x] **Step 1: Update docs**

Document the new cancellation controls foundation and keep the CLI REPL limitation explicit.

- [x] **Step 2: Run gates**

Run:
`PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check`
`PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings`
`PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace`
`git diff --check`

- [ ] **Step 3: Review, commit, and push**

Review the diff for runtime boundary violations, then commit and push to `main`.
