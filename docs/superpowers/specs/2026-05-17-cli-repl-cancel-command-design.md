# CLI REPL Cancel Command Design

## Goal

Reserve and document the interactive CLI `/cancel` command so users get a clear response today and Tessera has a stable command surface for future async run cancellation.

## Scope

This slice adds:

- `/cancel` parsing in the CLI REPL command parser.
- `/cancel` command discovery.
- A local no-active-run response in normal REPL mode.
- Existing paste-mode `/cancel` behavior remains unchanged.

It does not add terminal Ctrl-C handling, concurrent stdin reading while a provider stream is active, tool cancellation, provider abort handles, protocol schema changes, or a new runtime branch.

## Architecture

The command remains local to `tessera-cli`:

- `CliReplCommand::Cancel` represents the user intent.
- `CliReplSession::handle_command` returns a clear no-active-run message.
- Paste mode continues to intercept `/cancel` before normal command dispatch and drops only the local paste buffer.

Existing `tessera-core` cancellation through `EventSinkAction::Cancel` and `task_cancelled` trace events remains the runtime cancellation foundation. Wiring true running cancellation into REPL requires an async input/event layer and is deliberately left for a later slice.

## Testing

- Parser contract recognizes `/cancel`.
- Command discovery includes `/cancel`.
- Local command contract verifies `/cancel` reports no active run without touching runtime.
- Existing paste-mode contract verifies `/cancel` still discards pasted content.
