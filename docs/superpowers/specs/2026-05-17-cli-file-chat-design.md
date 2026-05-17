# CLI File Chat Design

## Goal

Let scripts and document-driven workflows send a one-shot prompt into Tessera with `tessera chat --file <path>`.

## Scope

This slice adds a non-interactive file prompt path:

- `chat --file <path>` reads a UTF-8 prompt file.
- The prompt still executes through config resolution, provider routing, core, storage, and trace writing.
- `--file`, `--stdin`, and `--prompt` are mutually exclusive prompt sources.
- `--resume` remains interactive-only.

This does not add batch multi-turn scripts, tool execution, shell execution, MCP runtime, or agent runtime.

## Architecture

`tessera-cli` owns file reading at the entrypoint. After reading the file into a prompt string, it calls the existing one-shot `run_chat_with_config` path. Provider/core/storage behavior is unchanged.

## Testing

- `tessera chat --help` exposes `--file <FILE>`.
- A child-process contract test passes `chat --file <path>` and observes the mock provider response.
- A contract test rejects multiple prompt sources.
- Existing `--prompt`, `--stdin`, interactive REPL, resume, sessions, and transcript tests continue to pass.
