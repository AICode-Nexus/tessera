# tessera-protocol

Public runtime schema for Tessera.

This crate owns provider-neutral IDs, runtime objects, `RunEvent`, `EventFrame`, trace records, normalized errors, provider capability, route decisions, context reference metadata, no-progress loop signals, diagnostics metadata, memory proposal metadata, skill manifest metadata, tool descriptor metadata, tool policy/approval metadata, tool dispatch/result metadata, tool-call repair metadata, workspace guardrail/sandbox decision metadata, OS sandbox profile metadata, workspace checkpoint metadata, and usage/cost telemetry.

It must not depend on provider SDKs, storage, CLI, TUI, HTTP clients, filesystem IO, or async runtime details.
