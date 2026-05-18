# Pause Resume Foundation Design

Date: 2026-05-18

## Goal

Add provider-neutral pause/resume lifecycle metadata and UI intents without implementing suspended provider execution.

## Current Context

Tessera already has:

- `TaskStatus::Paused` in `tessera-protocol`.
- task lifecycle events for created, started, completed, failed, and cancelled.
- shared `ClientIntent` and `ClientSnapshot` consumed by CLI, TUI, and GUI.
- `/cancel` and `RunCancellationToken` for interrupting active provider streams.
- trace-backed `chat --resume <trace_id>`, which means "restore a historical session projection", not "resume a paused task".

The missing foundation is a provider-neutral way to say a task was paused or resumed, plus UI-neutral intents that future shells can dispatch.

## Scope

This stage adds metadata and UI plumbing only:

- `RunEvent::TaskPaused { task_id, reason }`
- `RunEvent::TaskResumed { task_id, reason }`
- client projection of paused/resumed task status
- UI-neutral client intents for pause/resume requests
- typed GUI bindings for the new intents/events

It does not pause an HTTP provider stream, keep a background run alive, restore a checkpoint, execute tools, start an agent runtime, or schedule sub-agents.

## Naming

Keep historical session resume separate from task resume:

- Existing `chat --resume <trace_id>` and `/resume <trace_id|#>` continue to mean trace-backed session projection.
- New paused-task resume uses `ResumeTask` and a slash command named `/resume-task <task_id>`.

This avoids overloading the existing CLI session command.

## Protocol

Add two task lifecycle events:

```rust
TaskPaused {
    task_id: TaskId,
    reason: Option<String>,
}

TaskResumed {
    task_id: TaskId,
    reason: Option<String>,
}
```

They serialize to `task_paused` and `task_resumed`, expose `task_id()`, and write payloads with `task_id` and `reason`.

## Client

Add intents:

```rust
PauseTask { task_id: Option<TaskId> }
ResumeTask { task_id: TaskId }
```

Slash-command behavior:

- `/pause` targets the latest running task, matching `/cancel`'s "latest running task" ergonomics.
- `/pause <task_id>` targets an explicit task.
- `/resume-task <task_id>` targets an explicit paused task.

Projection behavior:

- live `TaskPaused` sets the task status to `Paused`, stores `cancel_reason` as the pause reason only if a dedicated pause reason field is not added in this stage, and refreshes task summary.
- live `TaskResumed` sets status to `Running`, clears `finished_at`, and refreshes task summary.
- replayed `task_paused` and `task_resumed` records do the same.

The first slice should not add dedicated pause timestamps or state machines unless tests prove they are needed.

## TUI And GUI

TUI should pass through the new typed intents and continue to render `task paused` through the existing status summary.

GUI bindings should include the new `ClientIntent` variants automatically after regeneration. GUI bridge should accept the typed intent as metadata-only and return an accepted notice, not execute runtime work.

## Testing

Use TDD:

1. Protocol contract for `task_paused` / `task_resumed` kind, payload, and `task_id()`.
2. Client contract for slash command intents and live/replayed status projection.
3. TUI contract that `/pause` maps to `ClientIntent::PauseTask`.
4. GUI bridge/bindings contract that generated TypeScript contains `pause_task` and `resume_task`, and GUI bridge accepts them without execution.
5. Full Rust and GUI gates.

## Acceptance Criteria

- The trace schema can represent paused and resumed task lifecycle metadata.
- `ClientSnapshot` can project live and replayed pause/resume events.
- TUI/GUI can dispatch typed pause/resume intents without owning runtime execution.
- Existing session resume behavior is unchanged.
- No provider stream suspension, tool execution, checkpoint restore, or agent runtime is introduced.

## Review Note

The Superpowers spec review step normally uses a subagent. This Codex session can only spawn subagents when the user explicitly requests them, so this stage uses local self-review and full verification.
