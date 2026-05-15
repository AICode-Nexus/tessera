# tessera-client

UI-neutral intent and projection model shared by TUI and future GUI shells.

This crate may depend on protocol and serialization primitives. It must not call provider SDKs, read storage internals, own runtime execution, or depend on terminal or GUI toolkit widgets.
