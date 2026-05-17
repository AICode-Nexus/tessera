# CLI Config Validate Design

## Goal

Let users and scripts run a startup-style configuration check before launching chat or TUI, without executing providers, opening storage, creating traces, or exposing secret values.

## Scope

This slice adds `tessera config validate`:

- Text output reports status, resolved data dir, provider validation summaries, and issues.
- `--json` output returns the same validation report for scripts.
- `--config <path>` resolves provider profiles from an explicit config file.
- `--data-dir <path>` can override the resolved data dir for validation output.
- Validation checks missing provider profiles, duplicate provider IDs, unsupported provider kinds, required OpenAI-compatible `base_url`, and configured API key env var presence.

This does not add live provider connectivity checks, model requests, storage writes, trace creation, tool execution, MCP runtime, or agent runtime.

## Architecture

`tessera-cli` exposes `CliConfigValidationReport`, `CliConfigProfileValidation`, and `CliConfigValidationIssue` DTOs. The command reuses existing config and data-dir resolution, then performs pure validation over `TesseraConfig` plus environment variable presence checks. Secret environment variable values are never stored in the report or printed.

## Testing

- `tessera config validate --help` exposes `--json`, `--config`, and `--data-dir`.
- A child-process contract test validates a mock plus OpenAI-compatible profile with the secret env set, covering text and JSON output.
- A missing secret env var fails validation, emits JSON, and does not create the resolved data dir.
- Structural provider errors cover missing profile lists, duplicate profile IDs, missing OpenAI-compatible `base_url`, and unsupported provider kind.
