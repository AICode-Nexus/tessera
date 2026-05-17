# CLI Events Command Design

## Goal

Let users and scripts inspect raw trace events without entering the REPL, replaying provider behavior, or opening storage internals directly.

## Scope

This slice adds `tessera events <trace_id>`:

- Text output shows trace ID, event sequence, timestamp, event kind, and next page cursor.
- `--json` output returns the read-only event page with trace records and `next_since_seq`.
- `--since <seq>` returns records after a sequence number.
- `--limit <n>` limits the page size.
- The command resolves config/data-dir like other trace inspection commands.

This does not add mutation, provider re-execution, tool execution, shell execution, MCP runtime, agent runtime, or HTTP server startup.

## Architecture

`tessera-cli` exposes a `CliEventPage` DTO derived from `tessera-core::RuntimeEventPage`. The command opens storage through `TraceStore`, invokes `RuntimeReader::list_events`, and formats either text lines or JSON. Event pagination behavior stays in core.

## Testing

- `tessera events --help` exposes `<trace_id>`, `--json`, `--since`, and `--limit`.
- A child-process contract test creates a chat trace and verifies text pagination output.
- JSON output is parseable and includes paged trace records plus `next_since_seq`.
- Existing one-shot, JSON, resume/continue, sessions, transcript, and replay tests continue to pass.
