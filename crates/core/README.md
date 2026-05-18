# tessera-core

Headless runtime orchestration for Tessera.

The core owns the conversation lifecycle, task/turn sequencing, provider stream routing, and trace persistence coordination. CLI and TUI should both use this crate instead of talking to provider or storage internals directly.

It exposes `RunControls` and `RunCancellationToken` so UI/runtime shells can request provider-neutral cancellation through the headless run loop. Cancellation records `task_cancelled` and `done` events; it does not execute tools, kill shell commands, or bypass provider/storage boundaries.

It includes a draft `ModelRouter` that records manual/default `RouteDecision` values with explicit route reasons. Auto routing remains disabled until a later policy-backed implementation.

It also includes a draft `NoProgressDetector` that turns no-output, repeated read-only, and repeated repair observations into provider-neutral no-progress signals. These signals stop/ask/summarize first and never enable silent route escalation.

It includes a `DiagnosticsReporter` helper that wraps LSP-style diagnostics into provider-neutral `diagnostics_reported` events. It does not start LSP servers, run compilers, or read workspace files.

It includes a read-only `SkillRegistry` for listing and finding `SkillManifest` metadata. It does not activate skills, execute workflows, or bypass future tool/policy boundaries.

It includes a read-only `ToolRegistry` for listing and finding `ToolDescriptor` metadata. It does not execute tools, dispatch tool calls, or bypass future policy/sandbox boundaries.

It includes a metadata-only `McpToolAdapter` for converting MCP tool specs and arguments into Tessera `ToolDescriptor` and `ToolCallRequest` values. It does not connect to MCP servers, execute tools, or treat MCP annotations as trusted permission grants.

It includes a draft `PolicyGate` that evaluates tool metadata into `allow`, `ask_user`, or `deny` decisions. It does not dispatch tools or grant shell/file/git execution by itself.

It includes an `OrderedToolResultBuffer` that can release completed tool results in declared order for trace/model visibility even when lower layers finish out of order. It does not execute tools.

It includes a `ToolRepairTelemetry` helper that records provider-neutral tool-call repair summaries without provider raw text, hidden reasoning, or secrets.

It includes a draft `WorkspaceGuardrailChecker` that lexically resolves requested paths against a `WorkspaceScope` and records sandbox decisions. It does not canonicalize paths, read files, write files, execute shell commands, or provide an OS sandbox.

It includes an `OsSandboxPlanner` that maps tool descriptors to read-only, workspace-write, network-required, or denied sandbox profiles. It does not start an OS sandbox, open network access, create checkpoints, or execute tools.

It includes a `WorkspaceCheckpointPlanner` that creates checkpoint metadata for sandbox profiles that require checkpoints. It does not create side-git state, read or write workspace files, restore snapshots, or revert changes.

It includes a pure in-memory `ContextWorkbench` for managing context references and token budget summaries across stable prefix, append-only transcript, and volatile scratch placement. Its read-only projection helper exposes context handles plus budget summary for client shells. It does not read files, canonicalize URIs, build provider prompts, or write context trace events.

It also exposes read-only runtime API foundations: `RuntimeReader` can page trace events, query indexed runtime object IDs, and rebuild task/artifact/snapshot summaries through the core boundary, while `RuntimeHttpApi` can shape those event pages as JSON and SSE frames. This is not a listening HTTP server and does not own runtime execution.

This crate must not render Ratatui widgets, expose provider-private response structures, execute tools, restore checkpoints, or persist API key material.
