# tessera-tui

Ratatui view layer for Tessera.

v0.1 starts with a minimal status-line surface for profile, reasoning, cache, and cost placeholders. The TUI is a view over core events, not the runtime owner.

This crate must not call provider SDKs, execute tools, write provider requests, or read SQLite internals directly.
