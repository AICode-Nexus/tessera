# CLI Top-Level Sessions Design

## Goal

Let users discover trace-backed sessions without entering the interactive REPL.

## Scope

This slice adds `tessera sessions`:

- Text output for human CLI use.
- `--json` output for scripts and future wrappers.
- `--config` and `--data-dir` resolution consistent with existing CLI commands.
- Read-only session discovery through `RuntimeReader`.

This does not add session deletion, mutation, trace editing, provider calls, tool execution, or runtime ownership outside core.

## Architecture

`tessera-cli` exposes a small `CliSessionSummary` DTO derived from `tessera-core`'s `RuntimeSessionSummary`. Both the top-level command and REPL `/sessions` use the same formatter, so session list behavior stays consistent while storage access remains behind `TraceStore` and `RuntimeReader`.

## Testing

- `tessera sessions --help` exposes `--json` and `--data-dir`.
- `tessera sessions --config <path>` lists a trace created through the normal chat path.
- `tessera sessions --json` emits parseable session summaries.
