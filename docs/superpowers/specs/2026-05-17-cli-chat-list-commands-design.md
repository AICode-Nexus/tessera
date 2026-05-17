# CLI Chat List Commands Design

## Goal

Let users discover interactive slash commands before starting the REPL, similar to coding-agent CLIs that expose available local commands without requiring runtime setup.

## Scope

This slice adds `tessera chat --list-commands`:

- Prints the same slash-command list used by REPL `/help`.
- Exits without resolving config.
- Exits without resolving data dir.
- Does not open storage, call providers, create traces, or start the REPL.

This does not add command completion, fuzzy command search, provider execution, tool execution, MCP runtime, agent runtime, or shell execution.

## Architecture

`tessera-cli` exposes `chat_command_lines` as the shared command-list formatter. REPL `/help` and `chat --list-commands` both use that formatter, keeping command discovery consistent. The CLI checks `--list-commands` before config/data-dir resolution.

## Testing

- `tessera chat --help` exposes `--list-commands`.
- A child-process contract test runs `tessera chat --list-commands --config <missing>` and verifies it succeeds, proving the path does not require config resolution.
- Output includes key commands such as `/help`, `/profiles`, `/resume <trace_id>`, and `/quit`.
- Output does not include the interactive REPL banner.
