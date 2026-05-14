# tessera-config

Local configuration loading for Tessera.

The config layer owns provider profiles, model defaults, data directory resolution, UI preference placeholders, and future guardrail config. Secret values are referenced by environment variable name, not stored directly.

This crate must not call providers, write traces, render UI, or persist API keys.
