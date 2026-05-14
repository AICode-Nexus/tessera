# tessera-protocol

Public runtime schema for Tessera.

This crate owns provider-neutral IDs, runtime objects, `RunEvent`, `EventFrame`, trace records, normalized errors, provider capability, route decisions, and usage/cost telemetry.

It must not depend on provider SDKs, storage, CLI, TUI, HTTP clients, filesystem IO, or async runtime details.
