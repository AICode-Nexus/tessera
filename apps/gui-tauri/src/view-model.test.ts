import { describe, expect, it } from 'vitest'

import { allowedCommandNames, forbiddenCommandNames } from './ipc'
import { buildShellMetrics, visibleMessages } from './view-model'
import type { ClientSnapshot } from './types'

const snapshot: ClientSnapshot = {
  status: {
    active_profile: 'mock-replay',
    available_profiles: ['mock-replay', 'read-only'],
    reasoning_visible: false,
    task_summary: 'task idle',
    artifact_summary: 'artifacts 0',
    approval_summary: 'approvals 0 pending',
    memory_summary: 'memory 0 pending',
    usage_summary: 'usage in 12 / out 8 / total 20',
    cache_summary: 'cache 8/12 (66%)',
    cost_summary: 'CNY 0.0000',
    context_summary: 'ctx 12/4000 (0%)',
    context_handles_summary: 'context 1 handles / 42/1024 tokens',
    telemetry: {
      input_tokens: 12,
      output_tokens: 8,
      total_tokens: 20,
      cache_read_tokens: 8,
      cache_write_tokens: 0,
      cache_miss_tokens: 4,
      cache_total_tokens: 12,
      latest_context_tokens: 12,
      max_context_tokens: 4000,
      estimated_cost: null,
      cost_currency: null,
      cost_currency_mixed: false,
    },
  },
  projection: {
    reasoning_visible: false,
    messages: [
      {
        role: 'user',
        content: 'hello gui',
        item_id: 'item_user',
        streaming: false,
      },
      {
        role: 'assistant',
        content: 'mock/replay response',
        item_id: 'item_assistant',
        streaming: false,
      },
    ],
  },
  tasks: [],
  artifacts: [],
  approvals: [],
  memory_proposals: [],
  context_handles: [
    {
      context_id: 'context_architecture',
      source_kind: 'file',
      source_uri: 'docs/technical-architecture.md',
      label: 'architecture',
      placement: 'stable_prefix',
      estimated_tokens: 42,
      pinned: true,
      summary: 'architecture contract',
    },
  ],
  draft_input: '',
}

describe('GUI shell view model', () => {
  it('keeps IPC command names on the allowed read-only/mock surface', () => {
    expect(allowedCommandNames).toEqual([
      'list_profiles',
      'load_client_snapshot',
      'submit_client_intent',
      'cancel_task',
      'load_trace_projection',
      'export_thread',
    ])
    expect(allowedCommandNames).not.toContain('call_provider')
    expect(allowedCommandNames).not.toContain('read_sql')
    expect(allowedCommandNames).not.toContain('execute_shell')
    expect(forbiddenCommandNames).toContain('call_provider')
  })

  it('builds compact status metrics from a client snapshot', () => {
    expect(buildShellMetrics(snapshot)).toEqual([
      { label: 'Profile', value: 'mock-replay' },
      { label: 'Task', value: 'task idle' },
      { label: 'Usage', value: 'usage in 12 / out 8 / total 20' },
      { label: 'Cache', value: 'cache 8/12 (66%)' },
      { label: 'Context', value: 'ctx 12/4000 (0%)' },
    ])
  })

  it('filters empty projected messages before rendering', () => {
    const withEmpty: ClientSnapshot = {
      ...snapshot,
      projection: {
        ...snapshot.projection,
        messages: [
          ...snapshot.projection.messages,
          { role: 'assistant', content: ' ', item_id: null, streaming: true },
        ],
      },
    }

    expect(visibleMessages(withEmpty)).toHaveLength(2)
  })
})
