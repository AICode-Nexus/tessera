# Pause Resume Foundation Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add provider-neutral pause/resume task lifecycle metadata and UI-neutral intents without implementing suspended provider execution.

**Architecture:** Extend protocol task lifecycle events, then project those events through `tessera-client`. TUI and GUI bridge accept typed pause/resume intents but do not own runtime execution. Existing `chat --resume <trace_id>` remains trace-backed session projection and is not renamed or repurposed.

**Tech Stack:** Rust, serde, ts-rs, tessera protocol/client/core/TUI/GUI bridge crates, TypeScript bindings.

---

## File Structure

- `crates/protocol/src/lib.rs`: add `RunEvent::TaskPaused` and `RunEvent::TaskResumed`.
- `crates/protocol/tests/protocol_contract.rs`: add protocol event contract test.
- `crates/client/src/lib.rs`: add `ClientIntent::PauseTask` / `ResumeTask`, slash commands, and live/replay task projection.
- `crates/client/tests/client_contract.rs`: add intent and projection contract tests.
- `crates/tui/src/lib.rs`: pass through pause/resume intents as non-local runtime-facing intents.
- `crates/tui/tests/tui_contract.rs`: add `/pause` dispatch test.
- `crates/gui-bridge/src/lib.rs`: accept pause/resume typed intents as metadata-only GUI notices.
- `crates/gui-bridge/tests/gui_bridge_contract.rs`: add GUI bridge no-runtime pause/resume test.
- `crates/gui-bindings/src/lib.rs`: regenerate bindings if exports require no manual change.
- `crates/gui-bindings/tests/bindings_contract.rs`: assert generated TypeScript includes `pause_task`, `resume_task`, `task_paused`, and `task_resumed`.
- `apps/gui-tauri/src/generated/bindings.ts`: regenerate.
- `apps/gui-tauri/src/ipc.ts`: keep forbidden runtime command list unchanged; no new IPC runtime command required.
- `CHANGELOG.md`, `docs/global-plan.md`, relevant README/protocol docs if boundary text changes.
- `docs/superpowers/handoffs/2026-05-18-pause-resume-foundation.md`: stage handoff.

## Chunk 1: Protocol Events

### Task 1: Add Pause/Resume Trace Metadata

**Files:**
- Modify: `crates/protocol/tests/protocol_contract.rs`
- Modify: `crates/protocol/src/lib.rs`

- [ ] **Step 1: Write failing protocol test**

Add `task_pause_resume_events_are_traceable_without_runtime_suspension`.

The test should:

```rust
let task_id = TaskId::from_static("task_pause_resume");
let paused = RunEvent::TaskPaused {
    task_id: task_id.clone(),
    reason: Some("user requested pause".to_string()),
};
let resumed = RunEvent::TaskResumed {
    task_id: task_id.clone(),
    reason: Some("user requested resume".to_string()),
};

assert_eq!(paused.kind(), "task_paused");
assert_eq!(resumed.kind(), "task_resumed");
assert_eq!(paused.task_id(), Some(task_id.clone()));
assert_eq!(resumed.task_id(), Some(task_id.clone()));
assert_eq!(paused.payload()["reason"], "user requested pause");
assert_eq!(resumed.payload()["reason"], "user requested resume");
```

- [ ] **Step 2: Run targeted protocol test and verify RED**

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-protocol --test protocol_contract task_pause_resume_events_are_traceable_without_runtime_suspension -- --nocapture
```

Expected: FAIL because variants do not exist.

- [ ] **Step 3: Implement protocol variants**

In `RunEvent`, add:

```rust
TaskPaused {
    task_id: TaskId,
    reason: Option<String>,
},
TaskResumed {
    task_id: TaskId,
    reason: Option<String>,
},
```

Update `kind()`, `task_id()`, and `payload()`.

- [ ] **Step 4: Re-run targeted protocol test and verify GREEN**

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-protocol --test protocol_contract task_pause_resume_events_are_traceable_without_runtime_suspension -- --nocapture
```

Expected: PASS.

## Chunk 2: Client Intents And Projection

### Task 2: Client Pause/Resume Intents

**Files:**
- Modify: `crates/client/tests/client_contract.rs`
- Modify: `crates/client/src/lib.rs`

- [ ] **Step 1: Write failing slash command intent test**

Add `client_snapshot_maps_pause_resume_commands_to_ui_neutral_intents`.

Create a running task with `TaskCreated` and `TaskStarted`, then assert:

```rust
snapshot.draft_input = "/pause".to_string();
assert_eq!(
    snapshot.submit_input(),
    Some(ClientIntent::PauseTask {
        task_id: Some(running_task_id.clone()),
    })
);

snapshot.draft_input = "/pause task_explicit".to_string();
assert_eq!(
    snapshot.submit_input(),
    Some(ClientIntent::PauseTask {
        task_id: Some(TaskId::from_static("task_explicit")),
    })
);

snapshot.draft_input = "/resume-task task_paused".to_string();
assert_eq!(
    snapshot.submit_input(),
    Some(ClientIntent::ResumeTask {
        task_id: TaskId::from_static("task_paused"),
    })
);
```

- [ ] **Step 2: Run targeted client test and verify RED**

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-client --test client_contract client_snapshot_maps_pause_resume_commands_to_ui_neutral_intents -- --nocapture
```

Expected: FAIL because intents/commands do not exist.

- [ ] **Step 3: Implement minimal client intents**

Add `PauseTask` and `ResumeTask` variants to `ClientIntent`.

Update `submit_input()`:

- `/pause` uses latest running task.
- `/pause <task_id>` uses explicit task.
- `/resume-task <task_id>` returns `ResumeTask`; empty id returns `None`.

Use a helper similar to `approval_intent` / `memory_intent`.

- [ ] **Step 4: Re-run targeted client intent test and verify GREEN**

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-client --test client_contract client_snapshot_maps_pause_resume_commands_to_ui_neutral_intents -- --nocapture
```

Expected: PASS.

### Task 3: Client Pause/Resume Projection

**Files:**
- Modify: `crates/client/tests/client_contract.rs`
- Modify: `crates/client/src/lib.rs`

- [ ] **Step 1: Write failing projection test**

Add `client_snapshot_projects_paused_and_resumed_tasks`.

The test should:

- create/start a task through live events
- apply `RunEvent::TaskPaused`
- assert task status is `TaskStatus::Paused`
- assert `status.task_summary == "task paused"`
- apply `RunEvent::TaskResumed`
- assert task status is `TaskStatus::Running`
- assert `status.task_summary == "task running"`
- replay a `task_paused` trace record into a fresh snapshot and assert it projects paused state

- [ ] **Step 2: Run targeted projection test and verify RED**

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-client --test client_contract client_snapshot_projects_paused_and_resumed_tasks -- --nocapture
```

Expected: FAIL because projection handling is missing.

- [ ] **Step 3: Implement live and replay projection**

In `apply_event`, handle `TaskPaused` and `TaskResumed`.

In `apply_trace_record`, handle `"task_paused"` and `"task_resumed"`.

Use existing `task_mut_or_insert` and `trace_record_task_id` helpers. Do not add runtime execution.

- [ ] **Step 4: Re-run targeted projection test and verify GREEN**

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-client --test client_contract client_snapshot_projects_paused_and_resumed_tasks -- --nocapture
```

Expected: PASS.

## Chunk 3: TUI, GUI Bridge, And Bindings

### Task 4: TUI Intent Pass-Through

**Files:**
- Modify: `crates/tui/tests/tui_contract.rs`
- Modify: `crates/tui/src/lib.rs`

- [ ] **Step 1: Write failing TUI test**

Add `pause_command_dispatches_runtime_intent_from_tui_state`.

Create/start a running task in `ChatViewState`, set input `/pause`, call `handle_terminal_input(..., TerminalInput::Submit)`, and assert:

```rust
TerminalAction::Dispatch(ClientIntent::PauseTask {
    task_id: Some(running_task_id),
})
```

- [ ] **Step 2: Run targeted TUI test and verify RED**

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-tui --test tui_contract pause_command_dispatches_runtime_intent_from_tui_state -- --nocapture
```

Expected: FAIL until client intent exists and TUI marks it non-local.

- [ ] **Step 3: Update TUI non-local intent matching**

In `apply_client_intent_locally` and terminal dispatch matching, include `PauseTask` and `ResumeTask` with runtime-facing intents.

- [ ] **Step 4: Re-run targeted TUI test and verify GREEN**

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-tui --test tui_contract pause_command_dispatches_runtime_intent_from_tui_state -- --nocapture
```

Expected: PASS.

### Task 5: GUI Bridge And Bindings

**Files:**
- Modify: `crates/gui-bridge/tests/gui_bridge_contract.rs`
- Modify: `crates/gui-bridge/src/lib.rs`
- Modify: `crates/gui-bindings/tests/bindings_contract.rs`
- Modify: `apps/gui-tauri/src/generated/bindings.ts`

- [ ] **Step 1: Write failing GUI bridge test**

Add `gui_bridge_pause_resume_task_intents_are_typed_but_do_not_execute_runtime_work`.

Assert `submit_client_intent(ClientIntent::PauseTask { ... })` and `ResumeTask { ... }` both return accepted notices mentioning mock/replay or typed/no runtime execution.

- [ ] **Step 2: Write failing bindings assertions**

In `generated_bindings_include_gui_dtos_without_forbidden_runtime_commands`, assert bindings contain:

```rust
assert!(bindings.contains("pause_task"));
assert!(bindings.contains("resume_task"));
assert!(bindings.contains("task_paused"));
assert!(bindings.contains("task_resumed"));
```

- [ ] **Step 3: Run targeted tests and verify RED**

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-gui-bridge --test gui_bridge_contract gui_bridge_pause_resume_task_intents_are_typed_but_do_not_execute_runtime_work -- --nocapture
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-gui-bindings --test bindings_contract generated_bindings_include_gui_dtos_without_forbidden_runtime_commands -- --nocapture
```

Expected: FAIL before implementation/regeneration.

- [ ] **Step 4: Implement GUI bridge acceptances and regenerate bindings**

Handle `PauseTask` and `ResumeTask` in `GuiBridge::submit_client_intent` as typed, metadata-only notices.

Regenerate bindings:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo run -p tessera-gui-bindings -- apps/gui-tauri/src/generated/bindings.ts
```

- [ ] **Step 5: Re-run GUI bridge/bindings tests and verify GREEN**

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-gui-bridge --test gui_bridge_contract gui_bridge_pause_resume_task_intents_are_typed_but_do_not_execute_runtime_work -- --nocapture
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-gui-bindings --test bindings_contract -- --nocapture
npm --prefix apps/gui-tauri run build
```

Expected: PASS.

## Chunk 4: Docs, Handoff, And Verification

### Task 6: Documentation And Final Gates

**Files:**
- Modify: `CHANGELOG.md`
- Modify: `docs/global-plan.md`
- Optional modify: `docs/protocol-v0.md`, `docs/trace-schema-v0.md`, `docs/gui-ready-architecture.md`
- Create: `docs/superpowers/handoffs/2026-05-18-pause-resume-foundation.md`

- [ ] **Step 1: Update docs**

Document:

- pause/resume lifecycle metadata exists
- UI intents exist
- no real provider suspension is implemented
- `Pause / resume` checklist is marked complete only for foundation semantics if implementation and gates pass

- [ ] **Step 2: Run full gates**

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check
PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings
PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace
npm --prefix apps/gui-tauri test
npm --prefix apps/gui-tauri run build
git diff --check
```

Expected: all PASS.

- [ ] **Step 3: Review and commit**

Review:

- no provider stream suspension
- no new tool execution
- no storage internals in client/TUI/GUI
- existing trace session resume naming unchanged

Commit:

```bash
git add CHANGELOG.md docs/global-plan.md docs/protocol-v0.md docs/trace-schema-v0.md docs/gui-ready-architecture.md crates/protocol/src/lib.rs crates/protocol/tests/protocol_contract.rs crates/client/src/lib.rs crates/client/tests/client_contract.rs crates/tui/src/lib.rs crates/tui/tests/tui_contract.rs crates/gui-bridge/src/lib.rs crates/gui-bridge/tests/gui_bridge_contract.rs crates/gui-bindings/tests/bindings_contract.rs apps/gui-tauri/src/generated/bindings.ts docs/superpowers/handoffs/2026-05-18-pause-resume-foundation.md
git commit -m "feat(client): add pause resume foundation"
```
