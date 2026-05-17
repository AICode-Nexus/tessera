# Tessera GUI Tauri Spike

Early Tauri 2 + React/Vite shell for v0.2 GUI validation.

The shell renders `ClientSnapshot` data and calls typed Rust commands exposed through `src-tauri`. It is intentionally limited to mock/replay and read-only projection paths. It does not call provider SDKs, read SQLite internals, execute tools, or own runtime task state.

TypeScript DTOs live in `src/generated/bindings.ts` and are generated from Rust by `tessera-gui-bindings`. Keep `src/types.ts` as the local re-export layer for frontend-only helper types.

Useful commands:

```bash
npm install
npm test
npm run build
PATH="$HOME/.cargo/bin:$PATH" cargo check --manifest-path src-tauri/Cargo.toml
```

`npm test` covers the GUI view model and a deterministic smoke path for mock/replay load,
submit, cancel, new-thread, and toolbar action accessibility names.

Regenerate DTOs from the workspace root:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo run -p tessera-gui-bindings -- apps/gui-tauri/src/generated/bindings.ts
```

The browser dev server can run with:

```bash
npm run dev
```
