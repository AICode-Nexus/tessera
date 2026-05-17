# CLI Continue Latest Design

## Goal

Let users resume the most recent trace-backed session without first running `tessera sessions` and copying a trace ID.

## Scope

This slice adds `tessera chat --continue`:

- Finds the most recently updated trace-backed session through the existing read-only session list.
- Starts the interactive CLI by projecting that trace through the same path as `chat --resume <trace_id>`.
- The next prompt uses restored user/assistant transcript history as provider-visible context.
- Fails clearly when there are no sessions.
- Rejects combinations with `--prompt`, `--stdin`, `--file`, or `--resume`.

This does not add automatic tool execution, shell execution, MCP runtime, agent runtime, or batch continuation.

## Architecture

`tessera-cli` exposes a small `latest_session_trace_id` helper built on top of `RuntimeReader::list_sessions`. The CLI entrypoint resolves `--continue` into a trace ID before calling the existing `run_chat_repl_with_config_and_resume` path, so runtime, provider, storage, and projection behavior remain shared with explicit resume.

## Testing

- `tessera chat --help` exposes `--continue`.
- A child-process contract test creates two traces and proves `chat --continue` resumes the latest one.
- A contract test rejects `chat --continue` when no trace-backed sessions exist.
- A contract test rejects `chat --continue` with one-shot prompt sources.
- Existing one-shot, JSON, explicit resume, sessions, and transcript tests continue to pass.
