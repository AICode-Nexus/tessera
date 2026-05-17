# tessera-cli

Headless command entrypoint for Tessera.

v0.1 exposes `tessera doctor` / `doctor --json`, `tessera init`, read-only `tessera config validate`, secret-safe `tessera profiles`, one-shot `tessera chat --provider mock --prompt ...`, script-friendly `chat --json`, `chat --list-commands`, trace inspection with `transcript` / `replay` / `events`, and interactive `tessera chat --provider mock` / `chat --continue` on top of `tessera-core`.

Interactive `chat` mode prints startup context for the active profile, data dir, and configured profiles. It supports `/help`, `/commands`, `/new`, `/clear`, `/paste`, `/profiles`, `/profile <id>`, numbered `/sessions`, `/resume <trace_id|#>`, `/doctor`, `/history`, `/status`, `/export`, and `/quit` while keeping provider execution behind core and client projection in `tessera-client`. In paste mode, `/send` submits the collected multiline prompt and `/cancel` discards it. `chat --list-commands` prints the same command list without resolving config or starting the REPL.

`tessera init` writes a starter TOML config with env var names only. It must not store provider secrets, bearer tokens, cookies, or `.env` values.

`tessera --version` reports both the crate version and the build git SHA so release assets, package wrappers, and `doctor --json` checks can be tied back to a source revision.

The CLI must not bypass core to call provider internals or write storage internals directly.
