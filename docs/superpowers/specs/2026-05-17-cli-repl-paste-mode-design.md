# CLI REPL Paste Mode Design

## Goal

Make the interactive CLI usable for long prompts, code snippets, logs, and other multiline input without requiring a separate file or shell pipeline.

## Scope

This slice adds:

- `/paste` to enter multiline prompt collection.
- `/send` to submit the collected prompt.
- `/cancel` to discard the collected prompt.

It does not execute shell commands, read files, mutate config, change provider behavior, alter trace schemas, start tools, or add agent runtime.

## Architecture

The feature stays in the `tessera-cli` REPL input loop:

- `parse_repl_command` recognizes `/paste`.
- The interactive loop handles `CliReplCommand::Paste` before regular command dispatch.
- Paste mode accumulates raw input lines locally until `/send` or `/cancel`.
- `/send` submits the collected prompt through the same prompt writer used by single-line REPL prompts.
- `/cancel` drops the local buffer and returns to the normal prompt.

`CliReplSession`, `tessera-core`, providers, storage, protocol, and client projection keep their existing responsibilities.

## Testing

- Parser contract recognizes `/paste`.
- Command discovery includes `/paste`.
- REPL contract verifies multiline content is submitted as one prompt.
- REPL contract verifies `/cancel` discards local pasted text.
- Existing history projection confirms only submitted multiline content reaches the visible transcript.
