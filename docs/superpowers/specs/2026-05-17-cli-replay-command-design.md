# CLI Replay Command Design

## Goal

Let users and scripts replay a trace-backed run summary without entering the REPL and without contacting a provider.

## Scope

This slice adds `tessera replay <trace_id>`:

- Text output includes the trace ID, event count, and reconstructed assistant text.
- `--json` output includes `trace_id`, `event_count`, `event_kinds`, and `assistant_text`.
- The command resolves config/data-dir like other trace inspection commands.
- Replay is read-only and provider-free.

This does not add full event timeline rendering, network replay, provider re-execution, tool execution, shell execution, MCP runtime, or agent runtime.

## Architecture

`tessera-cli` exposes a `CliReplaySummary` DTO derived from `tessera-core`'s `ReplaySummary`. The command opens `TraceStore`, invokes core `ReplayRunner`, and formats the result. Storage access remains behind `TraceStore`, and replay logic remains in core.

## Testing

- `tessera replay --help` exposes `<trace_id>` and `--json`.
- A child-process contract test creates a normal chat trace, runs `replay`, and observes reconstructed assistant text.
- JSON output is parseable and includes event kinds from the trace.
- Existing one-shot, JSON, resume/continue, sessions, and transcript tests continue to pass.
