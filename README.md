# Tessera

Tessera is a model-agnostic, agent-ready terminal workbench built on typed events, auditable tools, replayable runs, and composable skills.

## Current Status

This repository is in the requirements and architecture design phase. No implementation code has been added yet.

## Design Goals

- Model-agnostic provider architecture.
- Rust-first, quality-focused local runtime.
- Headless core with replaceable TUI, CLI, and runtime API surfaces.
- Auditable tool execution through policy gates.
- Replayable runs with durable thread, turn, item, task, and artifact records.
- Agent-ready architecture with skills, memory, multi-agent workflows, swarm scheduling, and learning proposals.
- Multi-task and multi-window TUI model without coupling UI state to runtime execution.

## Documents

- [Requirements](docs/requirements.md)
- [Architecture](docs/architecture.md)

## Non-Goals For This Phase

- No Rust workspace scaffold.
- No TUI implementation.
- No provider integration.
- No tool execution.
- No agent runtime.

The first milestone is to stabilize the product requirements and architecture before writing implementation code.
