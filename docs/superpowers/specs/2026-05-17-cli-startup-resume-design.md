# CLI Startup Resume Design

## Goal

Let users start the interactive CLI directly from an existing trace-backed session with `tessera chat --resume <trace_id>`.

## Scope

This slice adds a startup resume path for the existing interactive `chat` REPL:

- `chat --resume <trace_id>` loads the trace through the same read-only runtime projection as `/resume`.
- The resumed transcript is visible before the first prompt is submitted.
- Follow-up prompts reuse the existing provider-neutral history path.
- `--resume` remains scoped to interactive chat mode.

This does not add tool execution, shell execution, context compaction, long-term memory, MCP runtime, or agent runtime.

## Architecture

`tessera-cli` owns the startup convenience flag and delegates to the same `CliReplSession` command path used by `/resume`. The headless runtime behavior is unchanged: old trace events are projected into `ClientSnapshot`, and only new turns create new trace events when the user submits another prompt.

## Testing

- CLI help exposes `--resume`.
- A REPL started with `--resume` prints the resumed trace notice.
- The next prompt after startup resume sees the restored transcript as provider-visible history.
