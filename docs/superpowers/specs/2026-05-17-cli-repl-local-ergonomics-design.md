# CLI REPL Local Ergonomics Design

## Goal

Make the interactive CLI easier to operate in long sessions by adding local commands for command discovery, current visible history, and clearing the current visible thread.

## Scope

This slice adds:

- `/commands` as an alias for `/help`.
- `/history` to list the current visible `ClientSnapshot` messages in a compact text form.
- `/clear` to reset the current visible thread/projection.

It does not delete traces, mutate stored sessions, call providers, execute tools, start MCP or agent runtimes, or change protocol/storage schemas.

## Architecture

The feature stays inside `tessera-cli`:

- `parse_repl_command` maps new slash commands to local `CliReplCommand` variants.
- `CliReplSession::handle_command` reads or resets the existing `ClientSnapshot`.
- `/history` formats `ClientMessage` values from the UI-neutral client projection.
- `/clear` calls `ClientSnapshot::start_new_thread`, matching the current `/new` projection reset behavior while using user-facing REPL language.
- `chat_command_lines` remains the shared source for `/help` and `chat --list-commands`.

Core, providers, storage, and trace semantics remain unchanged.

## Testing

- Parser contract recognizes `/commands`, `/clear`, and `/history`.
- REPL session contract verifies `/history` on empty and non-empty projections.
- REPL session contract verifies `/clear` empties the visible projection without runtime work.
- Command-discovery contract verifies `chat --list-commands` advertises the new commands without starting the REPL.
