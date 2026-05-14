# tessera-core

Headless runtime orchestration for Tessera.

The core owns the conversation lifecycle, task/turn sequencing, provider stream routing, and trace persistence coordination. CLI and TUI should both use this crate instead of talking to provider or storage internals directly.

This crate must not render Ratatui widgets, expose provider-private response structures, execute tools, or persist API key material.
