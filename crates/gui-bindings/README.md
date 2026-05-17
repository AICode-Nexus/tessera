# tessera-gui-bindings

Generates TypeScript DTO bindings for the Tauri GUI shell from Rust types.

The generator is deliberately narrow: it exports GUI IPC and view-model DTOs from `tessera-protocol`, `tessera-client`, and `tessera-gui-bridge` into `apps/gui-tauri/src/generated/bindings.ts`. It does not generate runtime code, provider adapters, storage access, or tool execution paths.

Regenerate bindings from the workspace root:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo run -p tessera-gui-bindings -- apps/gui-tauri/src/generated/bindings.ts
```

Verify the checked-in file matches Rust generation:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-gui-bindings --test bindings_contract
```
