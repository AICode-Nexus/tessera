# ADR-001: GUI Architecture and Toolkit Direction

**Status:** Accepted
**Date:** 2026-05-15
**Deciders:** Tessera project owner, Codex
**Owner:** Tessera maintainers

## Context

Tessera is Rust-first and headless-first. CLI, TUI, replay, future runtime API, and future GUI must share the same protocol, trace, and runtime lifecycle.

The project needs a GUI path, but starting a GUI too early can easily create a second runtime, duplicate provider access, or put session/task state into UI code. The GUI decision therefore needs to optimize for:

- one runtime source of truth;
- typed and replayable UI state;
- AI-friendly small boundaries;
- cross-platform desktop distribution;
- accessibility and rich workbench UI;
- security permissions that can be audited.

## Decision

Tessera will use **Tauri 2 + TypeScript/React/Vite** as the default product GUI direction, but only after the UI-neutral `client` boundary is extracted.

Rust remains the runtime owner. The Tauri Rust side is a bridge over `client` and core/future runtime API. The WebView frontend renders snapshots and projection patches; it does not call providers, read SQLite internals, execute tools, or own task/session lifecycle.

`egui` remains a candidate for an internal Rust-first diagnostics or inspector panel. `GPUI` remains a watch item and is not the default v0.2 GUI path.

## Alternatives Considered

### Option 1: Tauri 2 + TypeScript/React/Vite (CHOSEN)

Pros:

- Keeps Rust on the privileged backend side while using system WebView for rich UI.
- Gives a mature retained UI/component model for complex chat/workbench surfaces.
- HTML accessibility, keyboard handling, and screenshot automation are easier to validate.
- TypeScript plus generated DTOs make the IPC boundary explicit and AI-friendly.
- Tauri permissions/capabilities can keep frontend authority narrow.

Cons:

- Introduces a Web/Rust dual stack.
- Requires discipline so React state does not become a second runtime state machine.
- Requires generated bindings or schema discipline to avoid Rust/TypeScript drift.

### Option 2: egui / eframe

Pros:

- Rust-first and simple to embed.
- Good fit for internal tools, inspectors, settings, and debug panels.
- Immediate-mode model reduces callback/state wiring.

Cons:

- Product-grade retained layouts, long transcript surfaces, and accessibility need more validation.
- Complex workbench UI can become harder to structure and test.

### Option 3: GPUI

Pros:

- Rust-native and designed for high-performance desktop applications.
- Fits the long-term desire for a native-feeling workbench.

Cons:

- Still active/pre-1.0 and likely to have breaking changes.
- Documentation and ecosystem are less mature than Tauri or React.
- v0.2 should not depend on it as the default path.

### Option 4: Electron

Pros:

- Mature desktop ecosystem.
- Large component and testing ecosystem.

Cons:

- Heavier runtime and wider Node boundary.
- Less aligned with Tessera's Rust-first local security posture.

### Option 5: Native Swift/WinUI/Linux UI

Pros:

- Best per-platform fidelity.

Cons:

- Creates multiple GUI implementations and duplicated bindings.
- Higher maintenance burden and worse AI-assisted consistency.

## Tradeoffs

Optimized for:

- typed IPC;
- replay-driven GUI debugging;
- small AI-editable modules;
- accessible product UI;
- one runtime source of truth;
- cross-platform desktop delivery.

Sacrificed:

- pure Rust GUI for the product shell;
- no-JavaScript distribution;
- avoiding a frontend build system.

Mitigations:

- Do not add GUI dependencies in v0.1.
- Extract `client` first.
- Generate TypeScript DTOs from Rust/schema.
- Keep all provider, storage, secret, and tool authority on the Rust side.
- Require mock/replay fixtures before real provider GUI paths.

## Consequences

Positive:

- Future GUI can be rich without polluting core runtime.
- TUI and GUI can share `ClientIntent`, `ClientProjection`, and trace replay.
- GUI bugs can be reproduced from trace fixtures.
- Tauri command permissions give a concrete security boundary.

Negative:

- Repository will eventually become Rust + TypeScript.
- CI will need frontend checks once GUI code exists.
- The team must prevent UI state drift between Rust and TypeScript.

Risks:

- Risk: React state becomes a hidden runtime.
  - Mitigation: frontend state only stores UI chrome and latest `ClientSnapshot`/patch projection.
- Risk: Tauri permissions become too broad.
  - Mitigation: start with no shell/process/SQL/frontend HTTP authority.
- Risk: generated bindings become stale.
  - Mitigation: binding generation becomes a required GUI check.

## Implementation Notes

Future layout:

```text
crates/client/
apps/gui-tauri/
  src/
  src-tauri/
```

Allowed bridge commands:

- `list_profiles`
- `load_client_snapshot`
- `submit_client_intent`
- `cancel_task`
- `load_trace_projection`
- `export_thread`

Forbidden bridge commands:

- `call_provider`
- `read_sql`
- `write_trace`
- `execute_shell`
- `run_tool`
- `read_env_secret`

The GUI must start with mock/replay support and a bounded event channel before any live provider GUI path.

## Follow-up Actions

- [x] Extract `crates/client` or `core::client` with `ClientIntent`, `ClientProjection`, and `ClientSnapshot`.
- [x] Move TUI message/status projection onto the shared client model.
- [x] Define Rust-to-TypeScript binding generation strategy for GUI DTOs.
- [x] Build a Tauri spike using mock/replay only.
- [x] Add deterministic GUI smoke automation after the spike exists.
- [ ] Add real-browser screenshot automation once Playwright/Chromium is stable in the local/CI environment.

## References

- Tauri Architecture: https://v2.tauri.app/concept/architecture/
- Tauri Calling Rust From Frontend: https://v2.tauri.app/develop/calling-rust/
- Tauri Calling Frontend From Rust: https://v2.tauri.app/develop/calling-frontend/
- Tauri Permissions: https://v2.tauri.app/security/permissions/
- egui: https://github.com/emilk/egui
- GPUI: https://github.com/zed-industries/zed/tree/main/crates/gpui

## Revision History

- 2026-05-15: Accepted initial GUI architecture and toolkit direction.
