# CLI Stdin Chat Design

## Goal

Let scripts pipe a prompt into Tessera one-shot chat with `tessera chat --stdin`.

## Scope

This slice adds a non-interactive stdin prompt path:

- `chat --stdin` reads all stdin as the prompt.
- The prompt still executes through config resolution, provider routing, core, storage, and trace writing.
- `--stdin` is mutually exclusive with `--prompt`.
- `--resume` remains interactive-only.

This does not add batch multi-turn scripts, tool execution, shell execution, MCP runtime, or agent runtime.

## Architecture

`tessera-cli` owns stdin reading at the entrypoint. After reading stdin into a prompt string, it calls the existing one-shot `run_chat_with_config` path. Provider/core/storage behavior is unchanged.

## Testing

- `tessera chat --help` exposes `--stdin`.
- A child-process contract test pipes stdin into `chat --stdin` and observes the mock provider response.
- Existing `--prompt`, interactive REPL, resume, sessions, and transcript tests continue to pass.
