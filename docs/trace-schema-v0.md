# Tessera Trace Schema v0

日期：2026-05-14

## 1. 目标

Trace 是 Tessera 的调试、审计、replay 和 AI 辅助修复底座。v0.1 从第一天写 trace，避免后续 provider、tool、agent 行为只能靠人工复现。

Trace v0 的目标：

- 每次运行都有 append-only JSONL。
- 每条 JSONL 都能映射到一个 `EventFrame`。
- SQLite 只做索引和查询，不替代 JSONL 的事件真相。
- replay 不依赖真实 API key。
- 大输出通过 artifact 引用，不直接塞进 transcript。

## 2. 文件布局

建议默认路径：

```text
~/.local/share/tessera/
  traces/
    <trace_id>.jsonl
  artifacts/
    <artifact_id>/
  tessera.sqlite3
```

项目级配置后续可以覆盖 data dir，但 v0.1 不需要支持复杂多 store。

## 3. JSONL Record

每一行是一条完整 JSON object。

```json
{
  "schema_version": 1,
  "trace_id": "trace_01",
  "seq": 1,
  "event_id": "evt_01",
  "timestamp": "2026-05-14T09:00:00.000Z",
  "thread_id": "thread_01",
  "turn_id": "turn_01",
  "item_id": "item_01",
  "task_id": "task_01",
  "event_kind": "assistant_delta",
  "payload": {
    "text": "Hello"
  },
  "extension": null,
  "artifact_refs": []
}
```

字段规则：

- `schema_version`：v0.1 固定为 `1`。
- `trace_id`：一次运行的 trace 文件 ID。
- `seq`：同一 trace 内单调递增，从 `1` 开始。
- `event_id`：事件唯一 ID。
- `timestamp`：UTC RFC3339。
- `thread_id`：可为空，但 chat run 必填。
- `turn_id`：非 turn 事件可为空。
- `item_id`：与具体消息或可观察单元相关时填写。
- `task_id`：与可运行任务相关时填写。
- `event_kind`：稳定 snake_case 字符串。
- `payload`：事件载荷。
- `extension`：安全的 provider 或系统扩展 metadata。
- `artifact_refs`：大输出引用列表。

Provider capability、reasoning、cache、cost、route decision 等体验数据必须进入标准 event 或安全 extension，不能只存在于 TUI footer 或 provider adapter 内存里。

Context workbench v0.2 的 `ContextReference` / `ContextBudget` 目前是 runtime schema，不是 trace event。后续 context add/remove、loader、compaction 或 handle read 进入 runtime 时必须新增标准 event，并避免把文件内容直接塞进 trace。

Agent profile v0.5 foundation 的 `AgentProfile` 目前也是 runtime metadata schema，不是 trace event。后续 agent loop 真正启动时必须使用保留的 `agent_started` / `agent_completed` 等标准事件记录生命周期，不能把 agent runtime 状态藏在 profile metadata 中。

## 4. Event Kind

当前必须支持（v0.1 基线 + v0.2/v0.3 草案信号）：

```text
thread_created
turn_started
user_message_recorded
provider_request_started
provider_capability_reported
route_decision_recorded
assistant_message_started
assistant_delta
assistant_reasoning_delta
assistant_message_completed
usage_reported
provider_request_completed
turn_completed
task_created
task_started
task_completed
task_failed
task_cancelled
task_paused
task_resumed
no_progress_loop_detected
diagnostics_reported
memory_write_proposed
memory_write_applied
memory_write_rejected
artifact_created
snapshot_created
tool_call_requested
tool_policy_decision_recorded
sandbox_decision_recorded
os_sandbox_profile_selected
tool_dispatch_started
tool_dispatch_completed
tool_result
tool_repair_reported
tool_call_approved
tool_call_denied
error
done
```

`task_paused` 和 `task_resumed` payload 必须包含 `task_id`，可选包含 `reason`。它们只表示 provider-neutral lifecycle metadata，供 replay、TUI、GUI 和 future runtime API 投影使用；不得被解释为真实 provider stream 已挂起、后台任务已持久化或 checkpoint 已恢复。

仍只保留命名，不触发功能：

```text
route_escalation_recorded
skill_activated
skill_step_started
memory_recall
agent_started
agent_handoff
agent_completed
swarm_task_started
swarm_agent_event
swarm_task_completed
learning_observation
learning_proposal_created
learning_proposal_applied
window_opened
window_focused
window_closed
window_layout_changed
```

## 5. Example Trace

```jsonl
{"schema_version":1,"trace_id":"trace_01","seq":1,"event_id":"evt_01","timestamp":"2026-05-14T09:00:00.000Z","thread_id":"thread_01","turn_id":null,"item_id":null,"task_id":"task_01","event_kind":"task_created","payload":{"task_id":"task_01","kind":"chat"},"extension":null,"artifact_refs":[]}
{"schema_version":1,"trace_id":"trace_01","seq":2,"event_id":"evt_02","timestamp":"2026-05-14T09:00:00.010Z","thread_id":"thread_01","turn_id":null,"item_id":null,"task_id":"task_01","event_kind":"task_started","payload":{"task_id":"task_01"},"extension":null,"artifact_refs":[]}
{"schema_version":1,"trace_id":"trace_01","seq":3,"event_id":"evt_03","timestamp":"2026-05-14T09:00:00.020Z","thread_id":"thread_01","turn_id":null,"item_id":null,"task_id":"task_01","event_kind":"thread_created","payload":{"thread_id":"thread_01"},"extension":null,"artifact_refs":[]}
{"schema_version":1,"trace_id":"trace_01","seq":4,"event_id":"evt_04","timestamp":"2026-05-14T09:00:00.030Z","thread_id":"thread_01","turn_id":"turn_01","item_id":null,"task_id":"task_01","event_kind":"turn_started","payload":{"turn_id":"turn_01"},"extension":null,"artifact_refs":[]}
{"schema_version":1,"trace_id":"trace_01","seq":5,"event_id":"evt_05","timestamp":"2026-05-14T09:00:00.040Z","thread_id":"thread_01","turn_id":"turn_01","item_id":"item_user_01","task_id":"task_01","event_kind":"user_message_recorded","payload":{"item_id":"item_user_01","text":"hello"},"extension":null,"artifact_refs":[]}
{"schema_version":1,"trace_id":"trace_01","seq":6,"event_id":"evt_06","timestamp":"2026-05-14T09:00:00.050Z","thread_id":"thread_01","turn_id":"turn_01","item_id":null,"task_id":"task_01","event_kind":"provider_capability_reported","payload":{"provider_id":"mock","capability":{"provider_id":"mock","supports_streaming":true,"supports_reasoning_delta":true,"supports_cache_telemetry":true,"supports_cost_estimate":true,"supports_tool_calling":false,"max_context_tokens":128000,"extension":null}},"extension":null,"artifact_refs":[]}
{"schema_version":1,"trace_id":"trace_01","seq":7,"event_id":"evt_07","timestamp":"2026-05-14T09:00:00.060Z","thread_id":"thread_01","turn_id":"turn_01","item_id":null,"task_id":"task_01","event_kind":"route_decision_recorded","payload":{"decision_id":"route_01","decision":{"requested_profile":"mock-default","selected_profile":"mock-default","selected_model":"mock-chat","reasoning_level":null,"strategy":"manual","decision_reason":"manual_profile_selected_auto_routing_disabled","fallback_reason":null}},"extension":null,"artifact_refs":[]}
{"schema_version":1,"trace_id":"trace_01","seq":8,"event_id":"evt_08","timestamp":"2026-05-14T09:00:00.070Z","thread_id":"thread_01","turn_id":"turn_01","item_id":null,"task_id":"task_01","event_kind":"provider_request_started","payload":{"provider_id":"mock","profile_id":"mock-default","model":"mock-chat"},"extension":null,"artifact_refs":[]}
{"schema_version":1,"trace_id":"trace_01","seq":9,"event_id":"evt_09","timestamp":"2026-05-14T09:00:00.080Z","thread_id":"thread_01","turn_id":"turn_01","item_id":"item_assistant_01","task_id":"task_01","event_kind":"assistant_delta","payload":{"item_id":"item_assistant_01","text":"Hello"},"extension":null,"artifact_refs":[]}
{"schema_version":1,"trace_id":"trace_01","seq":10,"event_id":"evt_10","timestamp":"2026-05-14T09:00:00.090Z","thread_id":"thread_01","turn_id":"turn_01","item_id":null,"task_id":"task_01","event_kind":"usage_reported","payload":{"input_tokens":10,"output_tokens":1,"total_tokens":11,"cache_read_tokens":0,"cache_write_tokens":0,"cache_miss_tokens":10,"latency_ms":15,"estimated_cost":null},"extension":null,"artifact_refs":[]}
{"schema_version":1,"trace_id":"trace_01","seq":11,"event_id":"evt_11","timestamp":"2026-05-14T09:00:00.100Z","thread_id":"thread_01","turn_id":"turn_01","item_id":null,"task_id":"task_01","event_kind":"done","payload":{},"extension":null,"artifact_refs":[]}
```

## 6. SQLite Index

SQLite 用于快速查询，不是事件来源。

建议表：

```text
threads
turns
items
tasks
artifacts
traces
event_index
schema_migrations
```

`event_index` 只保存轻量字段：

```text
trace_id
seq
event_id
timestamp
thread_id
turn_id
item_id
task_id
event_kind
jsonl_offset
```

如果 SQLite 损坏，应能从 JSONL 重建索引。

## 7. No-Progress Signal

`no_progress_loop_detected` 是 v0.2 草案事件，用于记录连续只读、重复 repair 或 assistant 无输出循环。payload 必须包含：

- `task_id`。
- `signal.kind`：`repeated_read_only`、`repeated_repair` 或 `no_output`。
- `signal.consecutive_count` 和 `signal.threshold`。
- `signal.action`：`stop`、`ask_user` 或 `summarize`。
- `signal.reason`。
- `signal.route_escalation_allowed`，当前默认为 `false`。

该事件只记录 provider-neutral 控制信号，不记录 provider 原始 reasoning，也不自动触发模型升档。

## 8. Snapshot / Checkpoint

`snapshot_created` 是 checkpoint metadata event，用于记录未来 side-git 或等价 checkpoint 的句柄。v0.3 foundation 可以由 core checkpoint planner 为 `requires_checkpoint` sandbox profile 生成该 metadata，但不创建真实 checkpoint。payload 必须包含：

- `checkpoint.id`。
- `checkpoint.kind`：`side_git`、`file_archive` 或 `external`。
- `checkpoint.storage_uri`。
- 可选 `checkpoint.workspace_root`、`checkpoint.parent_snapshot_id`、`checkpoint.summary`。

该事件不包含 restore command、revert command、shell command 或文件内容。后续真实 create/restore/revert 必须通过 policy/sandbox，并写入独立 trace event。

## 9. Tool Descriptor

`ToolDescriptor` 是 v0.3 草案 metadata，不是 trace event，也不表示工具已执行。它只描述工具 ID、输入/输出 JSON schema、所需权限、side effects 和 `parallel_safe`。

当前 trace schema 可以写入 `tool_call_requested`、`tool_policy_decision_recorded`、`sandbox_decision_recorded`、`os_sandbox_profile_selected`、`tool_dispatch_started`、`tool_dispatch_completed`、`tool_result`、`tool_call_approved` 和 `tool_call_denied`，用于记录工具请求、policy、workspace guardrail、sandbox decision、OS sandbox profile、调度、结果和 approval metadata。

`parallel_safe` 缺省为 `false`。第三方/MCP tool 必须显式 opt in，未来并发 dispatcher 才能把它视作可并行候选；即便并发执行，trace append 和模型可见结果仍必须保持声明顺序。

MCP adapter foundation 只把 MCP tool metadata 和 call arguments 转成 `ToolDescriptor` / `ToolCallRequest`。MCP annotations 只能作为不可信 hint，metadata 不得包含 server URL、command、executable、transport handle 或 secret。

`tool_dispatch_*` 和 `tool_result` 事件可以记录未来 dispatcher 的结果顺序合同，但当前 core 只提供排序 buffer，不提供真实工具 executor。后续真正执行工具时，必须先有 sandbox/checkpoint 边界，并继续保证 trace append 和模型可见结果按声明顺序输出。

Tool policy events payload 要求：

- `tool_call_requested.payload.request.call_id`。
- `tool_call_requested.payload.request.tool_id`。
- `tool_policy_decision_recorded.payload.decision.outcome`：`allow`、`deny` 或 `ask_user`。
- `tool_policy_decision_recorded.payload.decision.required_permissions`。
- `tool_policy_decision_recorded.payload.decision.side_effects`。
- `tool_call_approved.payload.approval.status` 或 `tool_call_denied.payload.approval.status`。

这些 payload 不得包含 command、executable、shell、env secret 或 provider-private execution handle。

## 10. Tool Dispatch / Ordered Result

`tool_dispatch_started`、`tool_dispatch_completed` 和 `tool_result` 是 v0.3 草案 metadata events，用于记录未来安全并发工具的调度和结果排序。payload 必须包含：

- `tool_dispatch_started.payload.dispatch.dispatch_id`。
- `tool_dispatch_started.payload.dispatch.call_id`。
- `tool_dispatch_started.payload.dispatch.tool_id`。
- `tool_dispatch_started.payload.dispatch.declared_index`。
- `tool_dispatch_started.payload.dispatch.parallel_safe`。
- `tool_dispatch_completed.payload.result.result_id`。
- `tool_dispatch_completed.payload.result.call_id`。
- `tool_dispatch_completed.payload.result.declared_index`。
- `tool_dispatch_completed.payload.result.status`：`succeeded`、`failed` 或 `skipped`。
- `tool_result.payload.result.output` 或 `tool_result.payload.result.error`。
- 可选 `tool_result.payload.result.artifact_refs`，用于外部化大输出。

即便未来底层工具并发完成，`tool_dispatch_completed` 和 `tool_result` 也必须按 `declared_index` append 到 trace，并按同一顺序暴露给模型。当前 schema 和 core buffer 不表示工具已经执行。

这些 payload 不得包含 command、executable、shell、env secret 或 provider-private execution handle。

## 11. Tool-Call Repair Telemetry

`tool_repair_reported` 是 v0.3 草案 metadata event，用于记录 provider tool-call 输出进入标准 `ToolCallRequest` 前后的修复摘要。payload 必须包含：

- `report.repair_id`。
- `report.kind`：`flattened_nested_calls`、`scavenged_json`、`truncated_arguments` 或 `call_storm_detected`。
- `report.reason`。
- 可选 `report.call_id` 和 `report.tool_id`。
- 可选 `report.original_call_count` 和 `report.repaired_call_count`。
- 可选 `report.truncated_bytes`。

该事件只能记录 provider-neutral 摘要，不得写入 provider 原始 reasoning、hidden content、raw text、command、executable、shell、env secret 或 provider-private execution handle。

## 12. Workspace Guardrail / Sandbox Decision

`sandbox_decision_recorded` 是 v0.3 草案 metadata event，用于记录 tool request 对 workspace scope 和 sandbox policy 的判定。payload 必须包含：

- `decision.decision_id`。
- `decision.kind`：`allow`、`deny` 或 `ask_user`。
- `decision.reason`。
- 可选 `decision.call_id` 和 `decision.tool_id`。
- `decision.guardrail.scope.workspace_root`。
- 可选 `decision.guardrail.requested_path` 和 `decision.guardrail.resolved_path`。
- `decision.guardrail.access`：`read`、`write` 或 `execute`。
- `decision.guardrail.within_workspace`。
- `decision.guardrail.required_permissions` 和 `decision.guardrail.side_effects`。

`resolved_path` 是词法解析结果，不表示已经 `canonicalize`、读取、写入或执行。该事件不得包含 command、executable、shell、env secret 或 provider-private execution handle。

## 13. OS Sandbox Profile

`os_sandbox_profile_selected` 是 v0.3 草案 metadata event，用于记录未来 tool runtime 应选择的 OS sandbox profile。payload 必须包含：

- `profile.profile_id`。
- `profile.mode`：`read_only`、`workspace_write`、`network_required` 或 `denied`。
- 可选 `profile.workspace_root`。
- `profile.filesystem`：`read_only`、`workspace_write` 或 `denied`。
- `profile.network`：`disabled` 或 `requested`。
- `profile.shell`：当前只能是 `denied`。
- `profile.requires_checkpoint`。
- `profile.reason`。

该事件只表示隔离 profile 被规划出来，不表示 OS sandbox 已启动、网络已打开、checkpoint 已创建或工具已执行。payload 不得包含 command、executable、shell command、env secret 或 provider-private execution handle。

## 14. Diagnostics / LSP Event

`diagnostics_reported` 是 v0.4 foundation metadata event，用于记录 LSP-style diagnostics。payload 必须包含：

- `report.report_id`。
- `report.source`，例如 `rust-analyzer`、`rustc` 或 future diagnostics adapter name。
- `report.diagnostics[]`。
- `diagnostics[].severity`：`error`、`warning`、`information` 或 `hint`。
- `diagnostics[].message`。
- 可选 `diagnostics[].code`、`diagnostics[].uri` 和 `diagnostics[].range`。

该事件不表示 LSP server、compiler 或 test runner 已经启动。payload 不得包含 command、executable、process id、authorization、cookie、env secret 或 provider-private execution handle。

## 15. Memory Proposal

`memory_write_proposed`、`memory_write_applied` 和 `memory_write_rejected` 是 v0.4 foundation metadata events，用于 UI review，不表示长期 memory runtime 已经写入。payload 必须包含：

- `proposal.proposal_id`。
- `proposal.status`：`pending`、`applied` 或 `rejected`。
- `proposal.title`。
- `proposal.summary`。
- 可选 `proposal.source_item_id` 和 `proposal.reason`。

这些事件不得包含 memory store path、database URI、embedding vector、secret、command、executable 或 provider-private execution handle。future memory runtime 真实写入前必须先有 scope schema、policy 和 trace 边界。

## 16. Artifact

大输出必须外部化成 artifact。

进入 artifact 的典型内容：

- 完整 provider raw metadata 的安全子集。
- 大段日志。
- 导出文件。
- 后续 tool output。
- 后续 patch/test report/agent transcript。

Artifact record 示例：

```json
{
  "artifact_id": "artifact_01",
  "kind": "provider_raw_metadata",
  "uri": "file://artifacts/artifact_01/metadata.json",
  "media_type": "application/json",
  "size_bytes": 2048
}
```

Trace 中只写 artifact reference，不写大内容。

## 17. Redaction

禁止写入 trace：

- API key。
- Authorization header。
- Cookie。
- `.env` 原文。
- secret provider 返回值。
- 系统 keychain 原文。
- 完整 HTTP request header。
- 含凭证的 URL。

允许写入：

- provider 名称。
- model 名称。
- base URL 的 origin，去掉 path/query 中的敏感信息。
- token usage。
- cache read/write/miss tokens。
- estimated cost 和 currency。
- latency。
- retry count。
- route decision strategy、selected profile、fallback reason。
- tool dispatch policy、parallel safety decision、repair summary 和 no-progress loop reason。
- provider capability。
- provider error code。
- 已脱敏 error message。

所有 redaction 必须发生在写入 JSONL 之前。

## 18. Replay Contract

Replay runner 读取 JSONL 时必须能够：

- 校验 `schema_version`。
- 按 `seq` 排序。
- 重建 Thread/Turn/Item/Task 的基本生命周期。
- 重放 assistant delta。
- 重放 reasoning delta、usage、cache telemetry 和 error。
- 跳过未知 extension。
- 在遇到未知 required event 时失败并给出明确错误。

v0.1 replay 可以先只支持 mock provider trace，不要求重放真实 provider 的网络行为。

## 19. Failure Handling

Trace writer 失败时：

- CLI/TUI 必须收到明确错误。
- 不应静默继续一次不可追踪的 run。
- 如果 JSONL 写入成功但 SQLite index 失败，应记录 index rebuild required。
- 如果 SQLite 写入成功但 JSONL 写入失败，该事件不应被视为 durable。

## 20. 验收

Trace schema v0 可进入实现前，必须满足：

- 一条 assistant delta 能从 provider stream 写入 JSONL。
- 一条 reasoning delta 能作为可选事件写入 JSONL。
- JSONL 可以重建一次最小 chat transcript。
- SQLite 可以从 JSONL 重建。
- secret 不会出现在 trace。
- usage/cache/cost/latency 能进入 trace。
- 大输出有 artifact 路径，而不是塞进 `payload`。
