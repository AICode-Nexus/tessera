# tessera-cli

Headless command entrypoint for Tessera.

v0.1 exposes `tessera doctor --json` and `tessera chat --provider mock --prompt ...` on top of `tessera-core`.

The CLI must not bypass core to call provider internals or write storage internals directly.
