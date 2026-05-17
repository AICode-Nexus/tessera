# CLI Doctor Text Design

## Goal

Make `tessera doctor` useful as a human-readable runtime health check, not only a one-line status wrapper around `doctor --json`.

## Scope

This slice updates `tessera doctor` text output:

- Show overall status.
- Show resolved data dir.
- Show trace writability.
- Show SQLite index health.
- Show configured provider profile IDs.
- Keep `doctor --json` unchanged for scripts.

This does not add provider execution, live connectivity checks, config validation expansion, trace inspection, tool execution, MCP runtime, or agent runtime.

## Architecture

`tessera-cli` keeps `DoctorReport` as the single report shape. Text output is produced by `format_doctor_lines`, while JSON output continues to serialize the same DTO. The command still resolves config/data-dir through existing CLI helpers and delegates health probing to `run_doctor_with_config`.

## Testing

- A child-process contract test runs `tessera doctor --config <path>`.
- Text output includes status, data dir, trace writability, SQLite index health, and provider profile IDs.
- Existing `doctor --json` behavior remains covered by the existing `DoctorReport` contract test.
