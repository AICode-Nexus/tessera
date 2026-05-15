# tessera-core

Headless runtime orchestration for Tessera.

The core owns the conversation lifecycle, task/turn sequencing, provider stream routing, and trace persistence coordination. CLI and TUI should both use this crate instead of talking to provider or storage internals directly.

It also exposes the first read-only runtime API surface: `RuntimeReader` can page trace events and query indexed runtime object IDs through the core boundary. This is a local Rust API, not the future HTTP/SSE server.

This crate must not render Ratatui widgets, expose provider-private response structures, execute tools, or persist API key material.
