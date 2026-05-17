# CLI Runtime v2 Design

## Goal

Make the CLI feel like a durable local workbench rather than a one-off chat wrapper by adding safe config initialization, session listing, and trace resume.

## Scope

This slice adds:

- `tessera init` to write a secret-safe local `tessera.toml` template.
- `/sessions` in interactive `tessera chat` to list recent trace-backed sessions.
- `/resume <trace_id>` to project a previous trace into the current CLI session.

It does not add tool execution, shell commands, file-editing agents, MCP runtime, automatic context compaction, or long-term memory writes.

## User Experience

Create a starter config:

```bash
tessera init --config ./tessera.toml
```

The generated file contains mock, Ollama, and OpenAI-compatible examples. It stores only environment variable names such as `TESSERA_OPENAI_COMPATIBLE_API_KEY`, never provider secrets.

Inside `tessera chat`:

```text
tessera(mock)> /sessions
trace_mock_... | 12 events | updated 2026-05-17T...

tessera(mock)> /resume trace_mock_...
resumed trace trace_mock_... (2 messages)
```

After resume, `/export` prints the restored transcript and future prompts continue using the selected profile.

## Architecture

`tessera init` stays in `tessera-cli` because it is local command orchestration. The template is static and uses TOML syntax explicitly to avoid serializing secret values.

Session listing and resume use `tessera-core::RuntimeReader`. `tessera-storage` may expose trace IDs as a public storage primitive, but the CLI consumes the read-only core API. The CLI does not scan SQLite internals or reconstruct runtime behavior itself.

Resume is projection only. It reads trace records, calls `ClientSnapshot::start_new_thread`, applies each trace record through `ClientSnapshot::apply_trace_record`, and leaves provider/storage authority unchanged.

## Boundaries

Allowed:

- Writing a config template requested by the user.
- Reading trace metadata through public runtime reader APIs.
- Projecting trace records into `ClientSnapshot`.

Forbidden:

- Writing provider secrets or `.env` values.
- Mutating trace history.
- Re-running provider calls during resume.
- Executing tools, shell commands, MCP calls, or file edits.

## Testing

Tests should verify:

- `init` writes a template with env var names and no secret values.
- `init` refuses to overwrite unless forced.
- `RuntimeReader::list_sessions` summarizes trace IDs without requiring provider access.
- `/sessions` renders runtime summaries.
- `/resume <trace_id>` loads transcript projection and rejects missing traces clearly.
