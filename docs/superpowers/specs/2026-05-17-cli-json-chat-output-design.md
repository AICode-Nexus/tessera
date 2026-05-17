# CLI JSON Chat Output Design

## Goal

Let scripts consume one-shot Tessera chat results without parsing human-oriented assistant text.

## Scope

This slice adds `tessera chat --json` for one-shot prompt sources:

- `chat --prompt ... --json`
- `chat --stdin --json`
- `chat --file <path> --json`

The JSON output contains:

- `trace_id`
- `assistant_text`

`--json` is rejected when no one-shot prompt source is present so interactive REPL mode does not silently ignore it.

This does not add streaming JSON lines, batch mode, tool execution, shell execution, MCP runtime, or agent runtime.

## Architecture

`tessera-cli` converts the existing `ConversationOutcome` into a small serializable `CliChatOutput` DTO. Runtime behavior stays unchanged: config resolution, provider routing, core execution, trace writing, and SQLite indexing still happen through the existing one-shot chat path.

## Testing

- `tessera chat --help` exposes `--json`.
- A child-process contract test runs `chat --prompt ... --json` and parses stdout as JSON.
- A contract test rejects `chat --json` without `--prompt`, `--stdin`, or `--file`.
- Existing text output, stdin/file prompt, REPL, resume, sessions, and transcript tests continue to pass.
