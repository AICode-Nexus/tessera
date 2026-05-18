# Context Handle Projection Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Project existing context references into UI-neutral client handles without reading source content, building prompts, or starting agent runtime.

**Architecture:** Keep `tessera-protocol::ContextReference` as the metadata source of truth. Add a read-only `ContextWorkbench` projection in `tessera-core`, then let `tessera-client` store and summarize projected handles in `ClientSnapshot`. GUI bindings include the new DTOs so future shells can render handles without depending on core.

**Tech Stack:** Rust, serde, ts-rs, existing `tessera-protocol`, `tessera-core`, `tessera-client`, `tessera-gui-bindings` contract tests.

---

## File Structure

- `crates/core/src/lib.rs`: add `ContextProjection` and `ContextWorkbench::projection()`.
- `crates/core/tests/conversation_engine_contract.rs`: add core contract test for read-only context projection.
- `crates/client/src/lib.rs`: add `ClientContextHandle`, source/placement projection enums, `ClientSnapshot::set_context_handles`, `context_handles` storage, and status summary.
- `crates/client/tests/client_contract.rs`: add client projection contract test.
- `crates/gui-bindings/src/lib.rs`: export new client context DTO bindings.
- `crates/gui-bindings/tests/bindings_contract.rs`: assert generated bindings include context DTOs and no forbidden runtime commands.
- `apps/gui-tauri/src/generated/bindings.ts`: regenerate checked-in TypeScript bindings after Rust DTO changes.
- `CHANGELOG.md`, `docs/global-plan.md`, `crates/core/README.md`, `crates/client/README.md`: document the foundation.

## Chunk 1: Core Read-Only Projection

### Task 1: Core Projection Contract

**Files:**
- Modify: `crates/core/tests/conversation_engine_contract.rs`
- Modify: `crates/core/src/lib.rs`

- [ ] **Step 1: Write the failing core test**

Add a test named `context_workbench_projects_handles_without_loading_sources`.

The test should:

```rust
let workbench = ContextWorkbench::from_references(
    ContextBudget {
        max_tokens: 200,
        reserved_output_tokens: 40,
    },
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
);
let projection = workbench.projection();
assert_eq!(projection.references.len(), 2);
assert_eq!(projection.summary.used_tokens, 150);
assert_eq!(projection.summary.available_tokens, 160);
assert_eq!(projection.references[0].source.label.as_deref(), Some("architecture"));
```

- [ ] **Step 2: Run the core test and verify RED**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-core --test conversation_engine_contract context_workbench_projects_handles_without_loading_sources -- --nocapture
```

Expected: FAIL because `ContextWorkbench::projection` or `ContextProjection` does not exist.

- [ ] **Step 3: Implement minimal core projection**

In `crates/core/src/lib.rs`, add:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextProjection {
    pub references: Vec<ContextReference>,
    pub summary: ContextBudgetSummary,
}
```

Add:

```rust
pub fn projection(&self) -> ContextProjection {
    ContextProjection {
        references: self.references.clone(),
        summary: self.summary(),
    }
}
```

Do not add file IO, URI canonicalization, prompt building, trace writing, or provider interaction.

- [ ] **Step 4: Re-run the core test and verify GREEN**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-core --test conversation_engine_contract context_workbench_projects_handles_without_loading_sources -- --nocapture
```

Expected: PASS.

## Chunk 2: Client Projection And Bindings

### Task 2: Client Context Handles

**Files:**
- Modify: `crates/client/tests/client_contract.rs`
- Modify: `crates/client/src/lib.rs`

- [ ] **Step 1: Write the failing client test**

Add a test named `client_snapshot_projects_context_handles_and_summary`.

The test should create a `ClientSnapshot`, call `set_context_handles` with two `ContextReference` values and a budget summary-like token count, and assert:

- `snapshot.context_handles.len() == 2`
- first handle keeps `context_id`, `source_kind`, label, placement, token estimate, pinned, and summary
- `snapshot.status.context_handles_summary == "context 2 handles / 150/160 tokens"`

Use over-budget coverage if simple:

```rust
assert!(!snapshot.status.context_handles_summary.contains("over budget"));
```

- [ ] **Step 2: Run the client test and verify RED**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-client --test client_contract client_snapshot_projects_context_handles_and_summary -- --nocapture
```

Expected: FAIL because client context handle projection does not exist.

- [ ] **Step 3: Implement minimal client DTOs and snapshot support**

In `crates/client/src/lib.rs`:

- Import `ContextId`, `ContextPlacement`, `ContextReference`, `ContextSourceKind`, and `ContextBudgetSummary` if needed.
- Add TS-derivable `ClientContextSourceKind` and `ClientContextPlacement` enums with snake_case serde.
- Add:

```rust
pub struct ClientContextHandle {
    pub context_id: ContextId,
    pub source_kind: ClientContextSourceKind,
    pub source_uri: Option<String>,
    pub label: Option<String>,
    pub placement: ClientContextPlacement,
    pub estimated_tokens: u64,
    pub pinned: bool,
    pub summary: Option<String>,
}
```

- Add `context_handles: Vec<ClientContextHandle>` to `ClientSnapshot`.
- Add `context_handles_summary: String` to `ClientStatus`.
- Add `ClientSnapshot::set_context_handles(...)` using `ContextReference` input and a summary input. If importing core's `ContextBudgetSummary` would violate crate boundaries, define a small client-side `ClientContextBudgetSummary` and let callers map to it.
- Reset context handles in `start_new_thread`.

Keep this UI-neutral. Do not depend on `tessera-core`, provider, storage, CLI, or TUI.

- [ ] **Step 4: Re-run the client test and verify GREEN**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-client --test client_contract client_snapshot_projects_context_handles_and_summary -- --nocapture
```

Expected: PASS.

### Task 3: GUI Binding DTOs

**Files:**
- Modify: `crates/gui-bindings/src/lib.rs`
- Modify: `crates/gui-bindings/tests/bindings_contract.rs`
- Modify: `apps/gui-tauri/src/generated/bindings.ts`

- [ ] **Step 1: Write the failing bindings assertion**

Update `generated_bindings_include_gui_dtos_without_forbidden_runtime_commands` to assert the generated output contains:

```rust
assert!(bindings.contains("export type ClientContextHandle"));
assert!(bindings.contains("export type ClientContextPlacement"));
assert!(bindings.contains("export type ClientContextSourceKind"));
assert!(bindings.contains("context_handles"));
```

- [ ] **Step 2: Run the bindings test and verify RED**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-gui-bindings --test bindings_contract generated_bindings_include_gui_dtos_without_forbidden_runtime_commands -- --nocapture
```

Expected: FAIL until `generate_bindings` exports the new DTOs and checked-in bindings are regenerated.

- [ ] **Step 3: Export and regenerate bindings**

Update `crates/gui-bindings/src/lib.rs` imports and `generate_bindings()` to push the new client context DTO declarations.

Regenerate checked-in bindings:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo run -p tessera-gui-bindings -- apps/gui-tauri/src/generated/bindings.ts
```

- [ ] **Step 4: Re-run bindings tests and verify GREEN**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-gui-bindings --test bindings_contract -- --nocapture
```

Expected: PASS.

## Chunk 3: Docs And Verification

### Task 4: Documentation And Checklist

**Files:**
- Modify: `CHANGELOG.md`
- Modify: `docs/global-plan.md`
- Modify: `crates/core/README.md`
- Modify: `crates/client/README.md`
- Optional Modify: `docs/protocol-v0.md`

- [ ] **Step 1: Update docs**

Document:

- Core now has a read-only context projection helper.
- Client now projects context handles and summary.
- No source content is read, no prompt is built, no context trace event is written.
- `docs/global-plan.md` marks `Context handle projection` complete only after implementation and verification.

- [ ] **Step 2: Run targeted tests**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-core --test conversation_engine_contract context_workbench_projects_handles_without_loading_sources -- --nocapture
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-client --test client_contract client_snapshot_projects_context_handles_and_summary -- --nocapture
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-gui-bindings --test bindings_contract -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Run full gates**

Run:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check
PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings
PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace
git diff --check
```

Expected: all PASS.

- [ ] **Step 4: Review and commit**

Review the diff for boundary violations:

- No file reads or URI canonicalization in projection code.
- No prompt construction.
- No provider/storage/CLI/TUI dependency added to `tessera-client`.
- No context trace event added.

Commit:

```bash
git add CHANGELOG.md docs/global-plan.md crates/core/README.md crates/client/README.md crates/core/src/lib.rs crates/core/tests/conversation_engine_contract.rs crates/client/src/lib.rs crates/client/tests/client_contract.rs crates/gui-bindings/src/lib.rs crates/gui-bindings/tests/bindings_contract.rs apps/gui-tauri/src/generated/bindings.ts
git commit -m "feat(client): project context handles"
```
