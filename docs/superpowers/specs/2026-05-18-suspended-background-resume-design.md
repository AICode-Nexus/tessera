# Suspended Background Resume Design

Date: 2026-05-18

## Goal

Define a conservative path from Tessera's pause/resume metadata foundation to real resumable work without adding a second runtime, provider-specific stream control, or unsafe checkpoint restore semantics.

## Current Context

Tessera now has the pieces needed for a design-level resume contract:

- `task_paused` and `task_resumed` lifecycle metadata in protocol and trace.
- UI-neutral `PauseTask` / `ResumeTask` intents across client, TUI, GUI, and CLI discovery.
- `RunCancellationToken` for interrupting active provider streams through core.
- trace-backed session projection through `RuntimeReader`, `chat --resume <trace_id>`, and `chat --continue`.
- provider-neutral chat history replay through `ConversationRequest.history`.
- checkpoint metadata schema through `WorkspaceCheckpoint`, `snapshot_created`, and `RuntimeReader::list_snapshots`, but no real restore/revert.
- read-only runtime API foundations that page trace events without owning execution.

The missing layer is a resumable run contract: what state is safe to persist, when a run can be paused, and what "resume" means when remote provider streams cannot be frozen and restarted byte-for-byte.

## Design Options

### Option A: Freeze Provider Streams

Keep the provider HTTP stream alive, stop reading from it when the user pauses, and continue reading on resume.

Trade-offs:

- It appears closest to "pause" in a media-player sense.
- It is unreliable for remote providers, proxies, timeouts, laptop sleep, process restarts, and network changes.
- It creates invisible resource ownership and makes CLI/TUI/GUI lifecycle much harder to reason about.
- It does not survive process exit, which is one of the main reasons to have resume.

This should not be Tessera's default.

### Option B: Cooperative Suspend And Restart From A Resume Envelope

Treat pause as a cooperative runtime transition. Core stops the active run at a safe checkpoint, writes a provider-neutral resume envelope into trace metadata, and marks the task paused. Resume starts a new core run from that envelope instead of trying to revive the old socket.

Trade-offs:

- It fits Tessera's trace-first architecture.
- It survives process exit because JSONL remains the source of truth.
- It is provider-neutral and can work for CLI, TUI, GUI, replay, and future runtime APIs.
- It requires clear honesty: mid-provider output cannot be continued byte-for-byte unless a future provider explicitly supports that capability.

This is the recommended path.

### Option C: Background Runtime Daemon First

Introduce a long-lived runtime process that owns tasks, then let shells detach and reattach.

Trade-offs:

- It is useful later for GUI and runtime API work.
- It risks becoming a second runtime if introduced before the trace/resume contract is stable.
- It does not solve checkpoint semantics by itself.

This should come after Option B establishes the portable contract.

## Recommended Contract

Tessera should implement suspended/background resume as cooperative suspend plus resume envelope.

Definitions:

- **Session resume** remains the existing `chat --resume <trace_id>` and `/resume <trace_id|#>` behavior: project historical trace records into a visible session and continue future prompts with restored transcript history.
- **Task pause** means core records that an active task should stop at a safe checkpoint and become `Paused`.
- **Task resume** means core starts a new run from the paused task's resume envelope and links the new task to the paused task.
- **Background resume** means the resume envelope is durable enough to survive process exit; it does not imply the original provider connection stayed alive.

The first real resume implementation should support chat tasks only. Tool runs, agent runs, swarm tasks, and workspace mutation resume remain out of scope until their policy, sandbox, checkpoint, and handoff contracts are implemented.

## Resume Envelope

A future `TaskPauseCheckpoint` should be provider-neutral and trace-safe. It should be recorded either as an optional field on a new pause-checkpoint event or as a new event adjacent to `task_paused`.

Suggested fields:

```rust
pub struct TaskPauseCheckpoint {
    pub checkpoint_id: PauseCheckpointId,
    pub task_id: TaskId,
    pub trace_id: String,
    pub last_seq: u64,
    pub thread_id: Option<ThreadId>,
    pub turn_id: Option<TurnId>,
    pub provider_id: ProviderId,
    pub profile_id: ModelProfileId,
    pub model: String,
    pub resume_mode: ResumeMode,
    pub workspace_snapshot_id: Option<SnapshotId>,
    pub transcript_event_range: Option<EventRange>,
    pub context_handle_ids: Vec<ContextId>,
    pub reason: Option<String>,
}
```

`ResumeMode` should start with conservative values:

```rust
pub enum ResumeMode {
    BeforeProviderRequest,
    AfterCompletedProviderTurn,
    FromTraceProjection,
}
```

`MidProviderStream` should not be accepted as resumable in the first implementation. If a user pauses during provider streaming, core can cancel or stop the stream and record the partial trace, but resume should be advertised as "continue from trace projection", not "continue exactly from token N".

Secrets, provider request headers, API keys, cookies, authorization values, raw socket handles, and provider-private execution IDs must never enter the envelope.

## Core Control Flow

The current `RunCancellationToken` is cancellation-specific. Suspended resume should not overload it. A future control type can generalize active run signals:

```rust
pub enum RunControlSignal {
    Cancel { reason: String },
    Pause { reason: Option<String> },
}
```

Core remains the only owner of run lifecycle transitions:

1. Shell submits `ClientIntent::PauseTask`.
2. Runtime bridge resolves it to an active task handle.
3. Core receives a `Pause` signal.
4. Core checks whether the current point is resumable.
5. Core writes a resume envelope if safe.
6. Core writes `task_paused`.
7. Core stops the active provider stream if one exists.
8. A later `ResumeTask` loads the envelope through core and starts a new run linked to the paused task.
9. Core writes `task_resumed` for the original task or a link event from original task to new task.

No CLI, TUI, GUI, provider adapter, or storage module should directly transition task state.

## Trace And Storage

JSONL remains the event truth. SQLite remains rebuildable.

Recommended trace additions for the implementation phase:

- `task_pause_checkpoint_created` with a `TaskPauseCheckpoint` payload.
- optional `resumed_from_task_id` / `resume_checkpoint_id` metadata on the new run's task-created path, or a separate `task_resume_link_recorded` event.

`RuntimeReader` should be able to reconstruct:

- paused tasks.
- latest checkpoint per paused task.
- whether a paused task is resumable.
- why a task is not resumable.

Storage should not introduce a mutable "paused task table" as the source of truth. Any SQLite index must be rebuildable from JSONL.

## Checkpoint And Workspace Policy

The first implementation should not restore workspace files. It can link to `WorkspaceCheckpoint` metadata if such metadata exists, but must treat it as advisory until real checkpoint creation and restore are implemented.

Resume modes:

- Chat-only trace resume: allowed first.
- Read-only tool or diagnostics task resume: later, after task-specific envelope design.
- Workspace-write task resume: blocked until sandbox and real checkpoint restore are implemented.
- Agent or swarm resume: blocked until structured handoff and reviewer gate exist.

This matches the existing roadmap constraint that file mutation tools require sandbox, policy, and checkpoint boundaries first.

## User-Facing Semantics

User-facing text should be explicit:

- "Paused" means Tessera recorded a pause checkpoint or metadata state.
- "Resume" means start a new run from saved trace/context state.
- "Resume" does not mean the original provider connection is still alive.

CLI naming should keep session and task resume separate:

- `/resume <trace_id|#>` remains session projection.
- `/resume-task <task_id>` targets paused task resume.
- A future `tessera tasks` or `tessera paused` command can list resumable paused tasks before exposing startup task resume flags.

## Phased Implementation

### Phase 1: Read-Only Paused Task Index

- Teach `RuntimeReader::list_tasks` to project `task_paused` and `task_resumed`.
- Add paused/resumable fields only if a test needs them.
- Add CLI/GUI read-only display for paused task state.

### Phase 2: Cooperative Pause Signal

- Add a core-owned pause signal alongside cancellation.
- Support pause before provider request and after completed provider turn.
- Record `task_pause_checkpoint_created` and `task_paused`.
- Return a clear "not resumable at this point" result for unsafe boundaries.

### Phase 3: Chat Resume From Envelope

- Load the pause checkpoint through core.
- Build provider-neutral history from trace projection.
- Start a new chat run linked to the paused task.
- Record resume linkage and `task_resumed`.

### Phase 4: Background Runtime Reattach

- Let GUI/runtime API observe active/paused tasks through core.
- Keep task ownership in the headless runtime.
- Do not create a second task state machine in GUI.

### Phase 5: Checkpoint Restore For Mutating Work

- Only after real checkpoint creation/restore exists.
- Require policy, sandbox, and workspace guardrail checks before restoring or continuing mutating tasks.

## Testing Strategy

- Protocol tests for checkpoint event shape and secret-safe serialization.
- Core tests for pause before provider request, pause after provider turn, and rejecting unsafe mid-stream resume.
- RuntimeReader tests for paused task and checkpoint reconstruction from trace.
- CLI tests that session resume and task resume remain separate commands.
- GUI bridge/bindings tests that no forbidden runtime/provider command appears.
- Storage rebuild tests proving indexes are reconstructable from JSONL.

## Acceptance Criteria For The First Implementation Slice

- Paused task state can be reconstructed from trace through `RuntimeReader`.
- Core can record a resumable pause checkpoint at safe boundaries.
- Resume starts a new core-owned chat run from a checkpoint envelope.
- Existing `chat --resume <trace_id>` semantics remain unchanged.
- No provider-private socket, API key, header, cookie, command, shell, or file mutation state is persisted.
- Workspace restore remains blocked until real checkpoint execution is designed and implemented.

## Review Note

The Superpowers brainstorming flow normally asks for explicit user approval and can dispatch spec reviewers. In this Codex desktop session, the user asked to continue and previously delegated judgment; subagents are only allowed when explicitly requested. This document is therefore a draft design PR for user review before implementation planning.
