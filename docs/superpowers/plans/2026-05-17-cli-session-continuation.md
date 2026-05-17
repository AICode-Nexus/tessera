# CLI Session Continuation Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make resumed CLI sessions continue with restored chat history.

**Architecture:** Add provider-neutral message history to provider/core request DTOs, then teach the CLI REPL to build history from `ClientSnapshot`.

**Tech Stack:** Rust, existing Tessera CLI/core/providers/client crates.

---

## Chunk 1: Provider And Core History Plumbing

### Task 1: Add message history to provider/core requests

**Files:**
- Modify: `crates/providers/src/lib.rs`
- Modify: `crates/providers/src/openai_compatible.rs`
- Modify: `crates/providers/src/ollama.rs`
- Modify: `crates/providers/tests/mock_provider_contract.rs`
- Modify: `crates/core/src/lib.rs`
- Modify: `crates/core/tests/conversation_engine_contract.rs`

- [x] Add RED tests proving provider/core request messages include prior turns and current prompt.
- [x] Implement provider-neutral `ProviderMessage` and role serialization.
- [x] Add provider-neutral history to `ConversationRequest`.
- [x] Pass history through core while tracing only the current user prompt.

## Chunk 2: CLI REPL History Builder

### Task 2: Build history from resumed/current CLI projection

**Files:**
- Modify: `crates/cli/src/lib.rs`
- Modify: `crates/cli/tests/cli_contract.rs`

- [x] Add RED tests proving resumed transcript is used for the next prompt.
- [x] Convert user/assistant `ClientSnapshot` messages into core conversation history.
- [x] Keep system/reasoning messages out of provider-visible history.

## Chunk 3: Docs And Gates

### Task 3: Update docs and verify

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/global-plan.md`

- [x] Document that `/resume` now continues with restored transcript history.
- [x] Run CLI smoke for prompt -> `/sessions` -> `/resume` -> follow-up prompt.
- [x] Run workspace fmt, clippy, tests, and `git diff --check`.
