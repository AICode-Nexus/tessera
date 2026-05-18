# Tessera Chat Pause Checkpoint Envelope Handoff

Date: 2026-05-18

## Branch

`codex/chat-resume-envelope`

## Completed

- Merged PR #9 into `main`.
- Added provider-neutral pause checkpoint metadata:
  - `TaskPauseCheckpointId`
  - `EventRange`
  - `ResumeMode`
  - `TaskPauseCheckpoint`
- Added `RunEvent::TaskPauseCheckpointCreated` with stable event kind `task_pause_checkpoint_created`.
- Updated core pause handling so `finish_paused` writes `task_pause_checkpoint_created` before `task_paused`.
- Kept cancellation separate from pause checkpoints.
- Updated `CHANGELOG.md`.
- Updated `docs/global-plan.md`.
- Updated `docs/protocol-v0.md` and `docs/trace-schema-v0.md`.

## TDD Notes

- Protocol RED: `task_pause_checkpoint_event_records_trace_resume_envelope_without_secrets` first failed because the checkpoint types and event did not exist.
- Protocol GREEN: the event serializes a trace-safe envelope with `resume_mode: from_trace_projection` and no secret/socket fields.
- Core RED: `conversation_engine_pause_token_records_paused_task_without_cancelling` first failed because pauses did not write `task_pause_checkpoint_created`.
- Core GREEN: core now writes the checkpoint before `task_paused`.

## Boundary Review

This stage records resume envelope metadata only:

- No `/resume-task` runtime execution was added.
- No provider socket freezing was implemented.
- No background runtime daemon or reattach service was added.
- No checkpoint restore/revert behavior was implemented.
- No tool, MCP, agent, sub-agent, or swarm runtime was added.

## Verification

Passed:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-protocol --test protocol_contract task_pause_checkpoint_event_records_trace_resume_envelope_without_secrets -- --nocapture
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-core --test conversation_engine_contract conversation_engine_pause_token_records_paused_task_without_cancelling -- --nocapture
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-core --test conversation_engine_contract conversation_engine_cancellation_token_interrupts_stalled_provider_stream -- --nocapture
PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check
PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings
PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace
git diff --check
```

## Next Recommended Stage

After this PR is merged, implement read-only reconstruction for latest pause checkpoints in `RuntimeReader`, then add chat-only `/resume-task` execution as a later slice.

Keep background reattach, workspace checkpoint restore, tools, MCP, and agent runtime staged until their contracts are ready.

## Notes For Next Session

- `output/` remains a pre-existing untracked directory and should be ignored unless explicitly requested.
- `ResumeMode::FromTraceProjection` means continue from JSONL trace projection, not from a frozen provider socket.
