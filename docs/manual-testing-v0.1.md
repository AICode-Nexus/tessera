# Tessera v0.1 Manual Testing Guide

Date: 2026-05-19

This guide is for local manual checks before trying a broader interactive session. It uses the deterministic mock provider only, so it must not require API keys, tokens, cookies, or live provider credentials.

For real provider Chinese test prompts and expected observations, see [real-provider-test-questions-zh.md](real-provider-test-questions-zh.md).

For repeated real-provider testing, keep your existing config as `./tessera.toml` in the working directory, or set `TESSERA_CONFIG=/path/to/your/tessera.toml` once in the shell. Then you can run `./target/debug/tessera chat --provider <profile-id>` without passing `--config` every time.

## 1. Build

Run from the repository root:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo build -p tessera-cli
```

## 2. Create A Temporary Mock Config

Use a temporary directory so manual smoke traces do not mix with your normal Tessera data:

```bash
TEST_ROOT="$(mktemp -d)"

cat > "$TEST_ROOT/tessera.toml" <<EOF
data_dir = "$TEST_ROOT/data"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-slow"
EOF
```

The `mock-slow` model is intentionally delayed so `/pause` can interrupt the active run.

## 3. Pause, List, And Resume By Number

This scripted REPL input is the recommended first smoke test:

```bash
printf 'pause this slow run\n/pause\n/resume-tasks\n/resume-task 1\n/sessions\n/quit\n' \
  | ./target/debug/tessera chat --provider offline --config "$TEST_ROOT/tessera.toml"
```

Expected output markers:

- Startup shows `active_profile: offline`.
- The first run prints `pause requested`.
- `/resume-tasks` prints one entry like `1. task_... | trace trace_... | provider offline | checkpoint task_pause_checkpoint_... | reason cli repl pause requested`.
- `/resume-task 1` prints `resuming task task_...`.
- The resumed run prints `assistant> mock response to: Continue the paused task ... (history messages: 2)`.
- `/sessions` prints two traces: the paused source trace and the new resume chat trace.

## 4. Inspect Trace Events

Copy the paused source `trace_...` value from `/sessions` or `/resume-tasks`, then inspect events:

```bash
./target/debug/tessera events <paused_trace_id> --config "$TEST_ROOT/tessera.toml"
```

Expected event markers on the paused source trace:

- `provider_request_started`
- `task_pause_checkpoint_created`
- `task_paused`
- `task_resumed`

The source trace should remain the event truth. SQLite indexes are rebuildable and should not be treated as the source of truth.

## 5. Negative Checks

Missing checkpoint:

```bash
printf '/resume-task task_missing_checkpoint\n/quit\n' \
  | ./target/debug/tessera chat --provider offline --config "$TEST_ROOT/tessera.toml"
```

Expected marker:

- `error: pause checkpoint not found for task: task_missing_checkpoint`

Out-of-range numbered selector after the first resume:

```bash
printf '/resume-task #1\n/quit\n' \
  | ./target/debug/tessera chat --provider offline --config "$TEST_ROOT/tessera.toml"
```

Expected marker:

- `error: resume task index out of range: 1 (available tasks: 0)`

Direct repeat resume with the old task id should also fail because the source task is no longer paused:

```bash
printf '/resume-task <old_task_id>\n/quit\n' \
  | ./target/debug/tessera chat --provider offline --config "$TEST_ROOT/tessera.toml"
```

Expected marker:

- `task <old_task_id> is not paused (current status: running)`

## 6. Optional Interactive Check

You can also run the REPL manually:

```bash
./target/debug/tessera chat --provider offline --config "$TEST_ROOT/tessera.toml"
```

Useful commands:

- `/help`
- `/doctor`
- `/pause`
- `/resume-tasks`
- `/resume-task 1`
- `/sessions`
- `/events` is not a REPL command; use the top-level `tessera events <trace_id>` command.

## 7. Cleanup

```bash
rm -rf "$TEST_ROOT"
```

## Scope Notes

This manual path covers the current v0.1 chat-only resume behavior. It does not test or imply support for provider socket freezing, background runtime reattach, workspace checkpoint restore/revert, tool execution, MCP, agents, sub-agents, swarms, or long-term memory runtime.
