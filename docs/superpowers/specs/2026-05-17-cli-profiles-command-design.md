# CLI Profiles Command Design

## Goal

Let users and scripts inspect configured provider profiles before starting chat, without entering the REPL and without exposing provider secret values.

## Scope

This slice adds `tessera profiles`:

- Text output lists provider ID, kind, default model, optional base URL, and optional API key environment variable name.
- `--json` output returns the same secret-safe DTO for scripts.
- `--config <path>` resolves provider profiles from an explicit config file.
- The command does not open storage or create traces.

This does not add provider execution, live provider validation, tool execution, shell execution, MCP runtime, agent runtime, or secret loading.

## Architecture

`tessera-cli` exposes a `CliProviderProfile` DTO derived from `tessera_config::ProviderProfile`. The command reuses existing config resolution, formats either text lines or JSON, and deliberately reports only the configured environment variable name for API keys.

## Testing

- `tessera profiles --help` exposes `--json` and `--config`.
- A child-process contract test loads a mock plus OpenAI-compatible profile from TOML.
- Text output includes provider ID/kind/model/base URL/api key env name.
- JSON output is parseable and includes the same secret-safe fields.
- Secret environment variable values are not printed in text or JSON.
