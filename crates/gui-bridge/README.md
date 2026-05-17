# tessera-gui-bridge

Typed bridge DTOs and mock/replay projection helpers for the Tauri GUI shell.

This crate sits on the client side of the runtime boundary. It may depend on `tessera-client` and `tessera-protocol`; it must not depend on providers, storage internals, TUI widgets, shell execution, or long-running runtime ownership.

Current scope:

- list GUI profiles for mock/replay and read-only modes;
- load a `ClientSnapshot`;
- submit `ClientIntent` values into mock/replay projection;
- accept typed approval and memory proposal review intents without executing tools or writing memory;
- expose a bounded GUI event buffer that returns backpressure instead of growing unbounded;
- project provided trace records through the shared client model.
