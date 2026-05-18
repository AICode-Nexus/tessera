# tessera-client

UI-neutral intent and projection model shared by TUI and future GUI shells. It owns slash-command intent parsing for `/new`, `/save`, `/export`, `/approve`, `/deny`, `/remember`, and `/forget`, task/artifact/approval/memory proposal projection from live/replayed events, context handle projection from protocol metadata, usage telemetry summaries, and markdown projection export.

Context handles expose IDs, source labels/URIs, placement, token estimates, pinned state, summaries, and an aggregate budget summary for rendering. They do not read source content, build prompts, write traces, or depend on `tessera-core`.

This crate may depend on protocol and serialization primitives. It must not call provider SDKs, read storage internals, own runtime execution, or depend on terminal or GUI toolkit widgets.
