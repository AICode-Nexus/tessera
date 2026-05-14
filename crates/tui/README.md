# tessera-tui

Ratatui view layer for Tessera.

v0.1 starts with a minimal status-line surface, chat view-state reducer, crossterm input mapping, Ratatui frame renderer, and profile-switch intent dispatch. The TUI is a view over core events and trace records, not the runtime owner.

This crate must not call provider SDKs, execute tools, write provider requests, or read SQLite internals directly.
