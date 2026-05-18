# Tessera Suspended Background Resume Design Handoff

Date: 2026-05-18

## Branch

`codex/suspended-resume-design`

## Completed

- Merged PR #5 into `main` after CI passed.
- Created the suspended/background resume design draft:
  - `docs/superpowers/specs/2026-05-18-suspended-background-resume-design.md`
- Updated `CHANGELOG.md`.
- Updated `docs/global-plan.md` to separate design completion from implementation.

## Design Decision

The design recommends cooperative pause checkpoints and resume envelopes instead of freezing provider sockets.

Rationale:

- Provider HTTP streams are not reliable durable pause handles.
- JSONL trace should remain the source of truth.
- Resume must survive process exit.
- CLI, TUI, GUI, replay, and future runtime APIs need one headless contract.

## Boundary Review

This stage is documentation-only:

- No provider stream suspension was implemented.
- No background runtime daemon was added.
- No checkpoint restore was implemented.
- No tool, MCP, agent, sub-agent, or swarm runtime was added.
- Existing `chat --resume <trace_id>` session projection semantics remain unchanged.

## Verification

Documentation-only gate:

```bash
git diff --check
```

## Next Recommended Stage

Review the design, then write an implementation plan for Phase 1 only:

- teach `RuntimeReader::list_tasks` to project `task_paused` / `task_resumed`;
- expose read-only paused task state cleanly;
- keep core pause signals and resume envelopes for later phases.

## Notes For Next Session

- `output/` remains a pre-existing untracked directory and should be ignored unless explicitly requested.
- The design intentionally avoids true socket freeze semantics.
