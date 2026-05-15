# tessera-client

UI-neutral intent and projection model shared by TUI and future GUI shells. It owns slash-command intent parsing for `/new`, `/save`, and `/export`, task and artifact projection from live/replayed events, usage telemetry summaries, and markdown projection export.

This crate may depend on protocol and serialization primitives. It must not call provider SDKs, read storage internals, own runtime execution, or depend on terminal or GUI toolkit widgets.
