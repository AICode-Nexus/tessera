# CLI REPL Design

## Goal

Make `tessera chat` usable as a Claude Code / Codex CLI style interactive shell while preserving Tessera's single headless runtime contract.

## Scope

The first slice adds a pure CLI REPL around existing chat runs. It supports prompt entry, visible streaming output, profile inspection/switching, status, markdown export, new-thread reset, help, and quit commands.

This slice deliberately does not add tool execution, shell commands, MCP runtime, agent loops, file modification, YOLO mode, long-term memory writes, or a second runtime.

## User Experience

`tessera chat --provider mock --prompt "hello"` remains a one-shot command.

`tessera chat --provider mock` starts interactive mode:

```text
Tessera CLI interactive chat
type /help for commands, /quit to exit

tessera(mock)> hello
assistant> mock response...

tessera(mock)> /status
profile mock | task completed | usage in 0 / out 0 / total 0
```

Supported commands:

- `/help`: show available commands.
- `/new`: clear client projection and start a fresh visible session.
- `/profiles`: list configured provider profiles.
- `/profile <id>`: switch active profile after validating it exists.
- `/status`: print compact profile/task/usage/cache/cost/context status.
- `/export`: print current markdown transcript to stdout.
- `/quit` and `/exit`: leave the REPL cleanly.

Unknown slash commands should return a clear CLI-only error without sending them to the provider.

## Architecture

The REPL lives in `tessera-cli` because it is a command-line shell, not a Ratatui view. Runtime work still flows through `run_chat_with_config_and_events`, which uses `core`, `providers`, and `storage`.

The REPL keeps a `tessera-client::ClientSnapshot` as its local projection. Every live `EventFrame` from core is applied to that snapshot before the CLI prints assistant deltas or status. Profile switching updates the snapshot through the shared client model rather than introducing CLI-only state.

## Boundaries

Allowed:

- CLI parsing and stdio loop.
- `ClientSnapshot` projection updates.
- Core chat runs through the existing provider routing function.
- Markdown export from client projection.

Forbidden:

- Direct provider SDK calls from the REPL.
- Direct trace or SQLite writes from the REPL.
- Shell command execution.
- File modification tools.
- Long-term memory writes.

## Testing

Tests should cover pure parsing/session behavior first:

- empty `--prompt` enters interactive mode.
- slash command parsing for help/new/profiles/profile/status/export/quit.
- unknown slash commands are not treated as model prompts.
- `/profile <id>` validates configured profiles.
- live assistant deltas are applied to the shared client snapshot.

Then smoke the one-shot command path to ensure existing behavior remains intact.
