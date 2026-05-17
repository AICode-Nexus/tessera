# CLI REPL Startup Doctor Design

## Goal

Make the interactive CLI feel less blind on entry by showing the runtime context that matters before the first prompt, and let users run the same health check from inside the REPL.

## Scope

This slice adds two CLI REPL affordances:

- Startup lines for `tessera chat` showing the active profile, resolved data dir, and configured provider profile IDs.
- A local `/doctor` slash command that reports runtime health for the active data dir.

It does not add provider execution paths, tool execution, MCP runtime, shell execution, config mutation, storage mutation beyond the existing doctor write probe, or GUI behavior.

## Architecture

`tessera-cli` keeps this as entrypoint/view behavior:

- `repl_startup_lines` formats startup context from the already resolved `TesseraConfig`, data dir, and selected profile.
- `CliReplCommand::Doctor` is parsed as a local command.
- `CliReplSession::handle_command_with_data_dir` handles `/doctor` by reusing `run_doctor_with_config` and `format_doctor_lines`.
- `/help` and `chat --list-commands` continue to share `chat_command_lines`, now including `/doctor`.

Core, providers, protocol, storage schemas, and client projection remain unchanged.

## Testing

- Parser contract recognizes `/doctor`.
- REPL command contract verifies `/doctor` returns status, data dir, trace writability, SQLite index health, and configured profiles.
- Startup contract verifies banner output includes active profile, data dir, available profiles, `chat --list-commands`, and `/doctor`.
- Existing REPL resume/history tests continue to cover provider history and trace projection behavior.
