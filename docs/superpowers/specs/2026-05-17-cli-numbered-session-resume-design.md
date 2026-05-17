# CLI Numbered Session Resume Design

## Goal

Make session recovery ergonomic in the interactive CLI by allowing users to resume from the numbered list they just saw instead of copying a full trace ID.

## Scope

This slice adds:

- 1-based numbering to human-readable `sessions` and `/sessions` output.
- `/resume <number>` support in the REPL.
- `chat --resume <number>` support for startup resume from the same sorted session list.

It does not change JSON output, trace IDs, trace storage, session sorting, provider execution, tool execution, MCP runtime, or agent runtime.

## Architecture

The implementation stays in `tessera-cli`:

- `format_session_lines` prefixes each human-readable row with its 1-based index.
- `CliReplSession::resume_session` treats numeric selectors as session indexes and resolves them through `list_sessions`.
- Non-numeric selectors continue to be treated as trace IDs.
- The same REPL resume path is used by `chat --resume`, so startup resume accepts either trace ID or numbered selector.

Core still owns the read-only session ordering through `RuntimeReader::list_sessions`; CLI only maps a user-facing selector onto the resulting trace ID.

## Testing

- REPL contract verifies `/sessions` output starts with `1. `.
- REPL contract verifies `/resume 1` resumes the first session returned by `list_sessions`.
- REPL contract verifies out-of-range indexes return a clear error.
- Top-level `sessions` contract verifies human-readable output includes numbering while JSON remains unchanged.
