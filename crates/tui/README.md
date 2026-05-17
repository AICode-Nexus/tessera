# tessera-tui

Ratatui view layer for Tessera.

v0.1 starts with a minimal status-line surface, crossterm input mapping, Ratatui frame renderer, profile-switch and `/new` `/save` `/export` intent dispatch, Ctrl-C cancellation intent for running tasks, and bounded live core-event channel delivery. The v0.3/v0.4 review surfaces add pending approval and memory proposal status plus `/approve` `/deny` `/remember` `/forget` intents, still without executing tools or writing long-term memory. Message/status projection lives in `tessera-client` so the future GUI can reuse the same UI-neutral model. The TUI is a view over core events and trace records, not the runtime owner.

This crate must not call provider SDKs, execute tools, write provider requests, or read SQLite internals directly.
