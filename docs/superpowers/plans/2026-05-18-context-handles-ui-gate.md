# Context Handles UI Gate Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Render context handle metadata in the TUI status line and make GUI TypeScript fixtures/gates cover the new context handle DTOs.

**Architecture:** Keep context handles as `tessera-client` metadata. TUI reads only `ClientStatus.context_handles_summary`; GUI TypeScript tests/build validate that snapshots include `context_handles` and `context_handles_summary`. No runtime execution, source reads, prompt building, provider calls, or storage access are added.

**Tech Stack:** Rust, Ratatui, `tessera-client`, Vitest, TypeScript, existing `apps/gui-tauri` scripts.

---

## File Structure

- `crates/tui/src/lib.rs`: add `context_handles_summary` to the existing status line.
- `crates/tui/tests/tui_contract.rs`: add a contract test proving context handle summary renders in the TUI.
- `apps/gui-tauri/src/ipc.ts`: update fallback `ClientSnapshot` shape with context handle fields.
- `apps/gui-tauri/src/view-model.test.ts`: update typed test fixture with context handle fields and assert GUI metrics keep the normal context token metric.
- `CHANGELOG.md`: document the UI/gate shortline addition.
- `docs/global-plan.md`: only update if a checklist line is added or re-scoped.
- `docs/superpowers/handoffs/2026-05-18-shortline-stage-2.md`: write stage handoff after implementation and verification.

## Chunk 1: TUI Context Handle Visibility

### Task 1: Status Line Contract

**Files:**
- Modify: `crates/tui/tests/tui_contract.rs`
- Modify: `crates/tui/src/lib.rs`

- [ ] **Step 1: Write the failing TUI test**

Add a test named `tui_status_line_renders_context_handle_summary`.

Use `ChatViewState::new("mock-default")`, then call `set_context_handles` with two `ContextReference` values and `ClientContextBudgetSummary`:

```rust
state.set_context_handles(
    [
        ContextReference {
            id: ContextId::from_static("context_architecture"),
            source: ContextSource {
                kind: ContextSourceKind::File,
                uri: Some("docs/technical-architecture.md".to_string()),
                label: Some("architecture".to_string()),
            },
            placement: ContextPlacement::StablePrefix,
            estimated_tokens: 100,
            pinned: true,
            summary: Some("architecture contract".to_string()),
            metadata: None,
        },
        ContextReference {
            id: ContextId::from_static("context_trace"),
            source: ContextSource {
                kind: ContextSourceKind::Trace,
                uri: Some("trace://trace_mock".to_string()),
                label: Some("transcript".to_string()),
            },
            placement: ContextPlacement::AppendOnlyTranscript,
            estimated_tokens: 50,
            pinned: false,
            summary: None,
            metadata: None,
        },
    ],
    ClientContextBudgetSummary {
        max_tokens: 200,
        reserved_output_tokens: 40,
        available_tokens: 160,
        used_tokens: 150,
        remaining_tokens: 10,
        stable_prefix_tokens: 100,
        append_only_transcript_tokens: 50,
        volatile_scratch_tokens: 0,
        over_budget: false,
    },
);
```

Render `status_line(&state)` to a string and assert it contains:

```text
context 2 handles / 150/160 tokens
```

- [ ] **Step 2: Run the targeted TUI test and verify RED**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-tui --test tui_contract tui_status_line_renders_context_handle_summary -- --nocapture
```

Expected: FAIL because `status_line` does not render `context_handles_summary` yet.

- [ ] **Step 3: Implement minimal status-line rendering**

In `crates/tui/src/lib.rs`, add a status segment after `memory_summary` or near `context_summary`:

```rust
Span::raw(" | "),
Span::raw(state.status.context_handles_summary.clone()),
```

Do not add a new runtime event, provider/storage dependency, or context source read.

- [ ] **Step 4: Re-run the targeted TUI test and verify GREEN**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-tui --test tui_contract tui_status_line_renders_context_handle_summary -- --nocapture
```

Expected: PASS.

## Chunk 2: GUI TypeScript Gate

### Task 2: Type-Safe GUI Fixtures

**Files:**
- Modify: `apps/gui-tauri/src/ipc.ts`
- Modify: `apps/gui-tauri/src/view-model.test.ts`

- [ ] **Step 1: Update GUI fixture test first**

In `apps/gui-tauri/src/view-model.test.ts`, update the typed `ClientSnapshot` fixture:

- add `status.context_handles_summary: 'context 1 handles / 42/1024 tokens'`
- add one `context_handles` entry using generated TypeScript fields:

```ts
context_handles: [
  {
    context_id: 'context_architecture',
    source_kind: 'file',
    source_uri: 'docs/technical-architecture.md',
    label: 'architecture',
    placement: 'stable_prefix',
    estimated_tokens: 42,
    pinned: true,
    summary: 'architecture contract',
  },
],
```

Keep the existing metrics expectation focused on `Context` using `context_summary`, not handle summary.

- [ ] **Step 2: Run GUI tests/build and verify the fixture compiles**

Run:

```bash
npm --prefix apps/gui-tauri test
npm --prefix apps/gui-tauri run build
```

Expected initially may FAIL if fallback snapshots in `ipc.ts` are missing fields.

- [ ] **Step 3: Update browser fallback snapshot**

In `apps/gui-tauri/src/ipc.ts`, update `createFallbackSnapshot()`:

- add `status.context_handles_summary: 'context 0 handles / 0/0 tokens'`
- add `context_handles: []`

In the `new_thread` fallback branch, also clear `context_handles` and preserve/reset `context_handles_summary` if needed.

Do not add any IPC command for reading context sources.

- [ ] **Step 4: Re-run GUI tests/build and verify GREEN**

Run:

```bash
npm --prefix apps/gui-tauri test
npm --prefix apps/gui-tauri run build
```

Expected: PASS.

## Chunk 3: Docs, Handoff, And Verification

### Task 3: Documentation And Final Gates

**Files:**
- Modify: `CHANGELOG.md`
- Create: `docs/superpowers/handoffs/2026-05-18-shortline-stage-2.md`

- [ ] **Step 1: Update changelog**

Add an `Unreleased` entry:

```markdown
- Surfaced projected context handle summaries in the TUI status line and strengthened GUI TypeScript fixtures/build coverage for generated context handle DTOs.
```

- [ ] **Step 2: Write the stage handoff**

Create `docs/superpowers/handoffs/2026-05-18-shortline-stage-2.md` with:

- branch name
- completed work
- verification commands and results
- next recommended medium-stage work: `Pause / resume foundation`
- reminder that `output/` is pre-existing and untouched

- [ ] **Step 3: Run full gates**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check
PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings
PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace
npm --prefix apps/gui-tauri test
npm --prefix apps/gui-tauri run build
git diff --check
```

Expected: all PASS.

- [ ] **Step 4: Review and commit**

Review the diff for boundary violations:

- TUI only reads `ClientStatus`.
- GUI only updates typed fixtures/fallbacks.
- No context source content reads.
- No provider/storage/tool/runtime commands added.

Commit:

```bash
git add CHANGELOG.md crates/tui/src/lib.rs crates/tui/tests/tui_contract.rs apps/gui-tauri/src/ipc.ts apps/gui-tauri/src/view-model.test.ts docs/superpowers/handoffs/2026-05-18-shortline-stage-2.md
git commit -m "feat(tui): surface context handle summary"
```
