# Tessera Real Provider Test Questions Handoff

Date: 2026-05-19

## Branch

`codex/real-provider-test-questions`

## Completed

- Started from `main` after PR #17.
- Added `docs/real-provider-test-questions-zh.md` for real provider manual testing in Chinese.
- Covered Chinese conversation quality, long structured output, context continuity, safety boundaries, JSON/Markdown output, provider stability, and pause/resume continuation prompts.
- Linked the new guide from `docs/manual-testing-v0.1.md` and `docs/v0.1-release-checklist.md`.
- Updated `CHANGELOG.md` and `docs/global-plan.md`.

## Boundary Review

This stage is documentation only:

- No mock-only flow is presented as the real provider path.
- No API key, token, cookie, authorization header, or `.env` secret is included.
- No provider socket freezing, background reattach, workspace restore, tool execution, MCP, or agent runtime behavior was added.

## Verification

Documentation gate passed:

```bash
git diff --check
```

## Next Recommended Stage

Run real provider manual testing from `docs/real-provider-test-questions-zh.md`, record provider/model/trace ids, and only then decide whether the next implementation stage should be real-provider robustness fixes or background reattach design.

## Notes For Next Session

- `output/` remains a pre-existing untracked directory and should be ignored unless explicitly requested.
- The new test-question guide is intended for real providers only, not mock validation.
