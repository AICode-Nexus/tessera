# Tessera Default Config Discovery Handoff

Date: 2026-05-19

## Branch

`codex/real-provider-test-questions`

## Completed

- Started from `main` after PR #18.
- Added default config discovery so CLI commands without `--config` reuse `TESSERA_CONFIG` first, then current directory `tessera.toml`, before falling back to the built-in mock config.
- Added CLI contract coverage for both current-directory `tessera.toml` and `TESSERA_CONFIG`.
- Updated real-provider manual test docs so repeated testing can run `./target/debug/tessera chat --provider <profile-id>` without passing `--config` each time.
- Updated `CHANGELOG.md` and `docs/global-plan.md`.

## TDD Notes

- RED: `chat_command_uses_default_tessera_toml_from_current_directory` failed because missing `--config` always used the mock default config.
- GREEN: `resolve_config(None)` now checks `TESSERA_CONFIG`, then `./tessera.toml`, then mock fallback.

## Boundary Review

This stage only improves config discovery:

- No provider credentials are stored or printed.
- Explicit `--config` still wins.
- No provider adapter/runtime behavior changed.
- No tool, MCP, agent, sub-agent, swarm, workspace restore, or background reattach behavior was added.

## Verification

Targeted gates passed:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-cli --test cli_contract chat_command_uses_default_tessera_toml_from_current_directory -- --nocapture
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-cli --test cli_contract chat_command_uses_tessera_config_env_when_no_config_flag_is_passed -- --nocapture
PATH="$HOME/.cargo/bin:$PATH" cargo test -p tessera-cli --test cli_contract config_resolution_loads_explicit_path_and_data_dir_prefers_config -- --nocapture
```

Full verification passed:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check
PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings
PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace
git diff --check
```

## Next Recommended Stage

Use the real provider prompt guide without repeated config flags. If real provider hand testing reveals stream or trace issues, prioritize adapter fixes before starting background reattach design.

## Notes For Next Session

- `output/` remains a pre-existing untracked directory and should be ignored unless explicitly requested.
- Users can set `TESSERA_CONFIG=/path/to/tessera.toml` once per shell for repeated manual testing.
