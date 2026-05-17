# CLI Transcript Command Design

## Goal

Let users inspect a trace-backed session transcript without entering the interactive REPL.

## Scope

This slice adds `tessera transcript <trace_id>`:

- Markdown output matching the existing REPL `/export` shape.
- `--json` output for scripts and future wrappers.
- `--config` and `--data-dir` resolution consistent with existing CLI commands.
- Read-only trace projection through `RuntimeReader` and `ClientSnapshot`.

This does not add trace mutation, provider calls, tool execution, hidden context loading, or a second runtime path.

## Architecture

`tessera-cli` loads trace records through `RuntimeReader::list_events`, applies them to a UI-neutral `ClientSnapshot`, and then exports either markdown or a small JSON transcript DTO. The command reuses existing projection logic rather than interpreting provider-specific events directly.

## Testing

- `tessera transcript --help` exposes `<trace_id>` and `--json`.
- Markdown output includes user and assistant turns from a trace created through the normal chat path.
- JSON output emits parseable transcript messages.
