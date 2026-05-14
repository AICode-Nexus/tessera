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

## 4. Event Kind

v0.1 必须支持：

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
artifact_created
error
done
```

v0.1 只保留命名，不触发功能：

```text
tool_call_requested
tool_call_approved
tool_call_denied
tool_result
skill_activated
skill_step_started
memory_recall
memory_write_proposed
memory_write_applied
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
snapshot_created
sandbox_decision_recorded
diagnostics_reported
```

## 5. Example Trace

```jsonl
{"schema_version":1,"trace_id":"trace_01","seq":1,"event_id":"evt_01","timestamp":"2026-05-14T09:00:00.000Z","thread_id":"thread_01","turn_id":null,"item_id":null,"task_id":"task_01","event_kind":"task_created","payload":{"kind":"chat"},"extension":null,"artifact_refs":[]}
{"schema_version":1,"trace_id":"trace_01","seq":2,"event_id":"evt_02","timestamp":"2026-05-14T09:00:00.010Z","thread_id":"thread_01","turn_id":"turn_01","item_id":null,"task_id":"task_01","event_kind":"turn_started","payload":{},"extension":null,"artifact_refs":[]}
{"schema_version":1,"trace_id":"trace_01","seq":3,"event_id":"evt_03","timestamp":"2026-05-14T09:00:00.020Z","thread_id":"thread_01","turn_id":"turn_01","item_id":"item_user_01","task_id":"task_01","event_kind":"user_message_recorded","payload":{"role":"user","content":"hello"},"extension":null,"artifact_refs":[]}
{"schema_version":1,"trace_id":"trace_01","seq":4,"event_id":"evt_04","timestamp":"2026-05-14T09:00:00.030Z","thread_id":"thread_01","turn_id":"turn_01","item_id":null,"task_id":"task_01","event_kind":"provider_request_started","payload":{"provider_id":"mock","profile_id":"mock-default","model":"mock-chat"},"extension":null,"artifact_refs":[]}
{"schema_version":1,"trace_id":"trace_01","seq":5,"event_id":"evt_05","timestamp":"2026-05-14T09:00:00.035Z","thread_id":"thread_01","turn_id":"turn_01","item_id":null,"task_id":"task_01","event_kind":"route_decision_recorded","payload":{"strategy":"manual","selected_profile":"mock-default","selected_model":"mock-chat"},"extension":null,"artifact_refs":[]}
{"schema_version":1,"trace_id":"trace_01","seq":6,"event_id":"evt_06","timestamp":"2026-05-14T09:00:00.040Z","thread_id":"thread_01","turn_id":"turn_01","item_id":"item_assistant_01","task_id":"task_01","event_kind":"assistant_delta","payload":{"text":"Hello"},"extension":null,"artifact_refs":[]}
{"schema_version":1,"trace_id":"trace_01","seq":7,"event_id":"evt_07","timestamp":"2026-05-14T09:00:00.045Z","thread_id":"thread_01","turn_id":"turn_01","item_id":null,"task_id":"task_01","event_kind":"usage_reported","payload":{"input_tokens":10,"output_tokens":1,"cache_read_tokens":0,"cache_write_tokens":0,"latency_ms":15,"estimated_cost":null},"extension":null,"artifact_refs":[]}
{"schema_version":1,"trace_id":"trace_01","seq":8,"event_id":"evt_08","timestamp":"2026-05-14T09:00:00.050Z","thread_id":"thread_01","turn_id":"turn_01","item_id":null,"task_id":"task_01","event_kind":"done","payload":{},"extension":null,"artifact_refs":[]}
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

## 7. Artifact

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

## 8. Redaction

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
- provider capability。
- provider error code。
- 已脱敏 error message。

所有 redaction 必须发生在写入 JSONL 之前。

## 9. Replay Contract

Replay runner 读取 JSONL 时必须能够：

- 校验 `schema_version`。
- 按 `seq` 排序。
- 重建 Thread/Turn/Item/Task 的基本生命周期。
- 重放 assistant delta。
- 重放 reasoning delta、usage、cache telemetry 和 error。
- 跳过未知 extension。
- 在遇到未知 required event 时失败并给出明确错误。

v0.1 replay 可以先只支持 mock provider trace，不要求重放真实 provider 的网络行为。

## 10. Failure Handling

Trace writer 失败时：

- CLI/TUI 必须收到明确错误。
- 不应静默继续一次不可追踪的 run。
- 如果 JSONL 写入成功但 SQLite index 失败，应记录 index rebuild required。
- 如果 SQLite 写入成功但 JSONL 写入失败，该事件不应被视为 durable。

## 11. 验收

Trace schema v0 可进入实现前，必须满足：

- 一条 assistant delta 能从 provider stream 写入 JSONL。
- 一条 reasoning delta 能作为可选事件写入 JSONL。
- JSONL 可以重建一次最小 chat transcript。
- SQLite 可以从 JSONL 重建。
- secret 不会出现在 trace。
- usage/cache/cost/latency 能进入 trace。
- 大输出有 artifact 路径，而不是塞进 `payload`。
