# tessera-cli

Headless command entrypoint for Tessera.

v0.1 exposes `tessera doctor --json` and `tessera chat --provider mock --prompt ...` on top of `tessera-core`.

`tessera --version` reports both the crate version and the build git SHA so release assets, package wrappers, and `doctor --json` checks can be tied back to a source revision.

The CLI must not bypass core to call provider internals or write storage internals directly.
