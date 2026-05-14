# tessera-tui

Ratatui view layer for Tessera.

v0.1 starts with a minimal status-line surface plus a chat view-state reducer that can turn input into user intents and render core `EventFrame` messages. The TUI is a view over core events, not the runtime owner.

This crate must not call provider SDKs, execute tools, write provider requests, or read SQLite internals directly.
