# Roadmap Governance Refresh Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the stale mixed global checklist with a current, rigorous roadmap system that covers v0.1-v0.9 and clearly separates version plans, current progress, and implementation contracts.

**Architecture:** `docs/version-plan.md` becomes the durable roadmap source of truth for v0.1-v0.9. `docs/global-plan.md` becomes a current status dashboard and execution control document, not a historical dumping ground. Existing architecture documents remain authoritative for invariants and boundaries; `docs/v0.1-plan.md` remains the detailed historical v0.1 plan.

**Tech Stack:** Markdown documentation, existing Rust workspace verification gates, git branch `codex/top-level-paused-task-cli`.

---

## File Structure

- Create `docs/version-plan.md`
  - Owns v0.1-v0.9 version themes, scope, exits, non-goals, and dependencies.

- Replace `docs/global-plan.md`
  - Current state dashboard.
  - Version status matrix.
  - Active queue.
  - Blocked/not-started work.
  - Mandatory update protocol.

- Modify `README.md`
  - Point readers to the new roadmap hierarchy.
  - Update current status wording so it no longer sounds v0.1-only.

- Modify `AGENTS.md`
  - Treat `docs/version-plan.md` as part of the current contract.
  - Clarify that `docs/global-plan.md` and `CHANGELOG.md` must be updated when staged work changes.

- Modify `CHANGELOG.md`
  - Add an Unreleased entry for the roadmap governance refresh.

---

## Chunk 1: Roadmap Source Of Truth

- [ ] **Step 1: Create `docs/version-plan.md`**
  - Include v0.1-v0.9 sections.
  - For each version include status, goal, included scope, excluded scope, exit criteria, and dependencies.
  - Explicitly distinguish "foundation complete" from "runtime user capability complete".

- [ ] **Step 2: Rewrite `docs/global-plan.md`**
  - Use current date.
  - Add document hierarchy.
  - Add version status matrix.
  - Add current completed work, active next work, blocked work, and update rules.
  - Preserve current facts from the previous checklist, but remove obsolete phrasing like "下一步可打 final tag" after tags already exist.

## Chunk 2: Contract And Index Updates

- [ ] **Step 3: Update `README.md`**
  - Add `docs/version-plan.md` to Documents.
  - Reword Current Status as post-v0.1 foundation progress rather than v0.1 scaffold only.

- [ ] **Step 4: Update `AGENTS.md`**
  - Add `docs/version-plan.md` to Current Contract.
  - Clarify v0.1 is released while staged roadmap items remain gated by the version plan.

- [ ] **Step 5: Update `CHANGELOG.md`**
  - Add roadmap governance refresh bullet under Unreleased.

## Chunk 3: Verification And Merge

- [ ] **Step 6: Verify docs and code gates**
  - Run `git diff --check`.
  - Because the branch includes CLI implementation from the previous slice, run full Rust gates:
    - `PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check`
    - `PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings`
    - `PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace`

- [ ] **Step 7: Commit branch**
  - Stage the top-level paused task CLI work and roadmap refresh together.
  - Commit with a conventional message.

- [ ] **Step 8: Merge to main locally**
  - Switch to `/Users/admin/work/tessera`.
  - Confirm `main` only has unrelated untracked `output/`.
  - Merge `codex/top-level-paused-task-cli` into `main`.
  - Re-run at least `git diff --check` and `cargo test --workspace` on `main`.
