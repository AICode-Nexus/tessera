import { describe, expect, it } from 'vitest'

import { allowedCommandNames, forbiddenCommandNames } from './ipc'
import {
  buildCodingWorkflowRows,
  buildShellMetrics,
  buildWorktreeLifecycleRows,
  visibleMessages,
} from './view-model'
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
    handoff_summary: 'handoffs 0',
    coding_workflow_summary: 'workflows 0',
    worktree_summary: 'worktrees 0',
    subagent_summary: 'subagents 0',
    subagent_runtime_summary: 'subagent runtime 0',
    subagent_transcript_lifecycle_summary: 'subagent transcripts 0',
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
  handoffs: [],
  reviewer_gates: [],
  coding_workflows: [],
  worktree_lifecycles: [],
  subagent_sessions: [],
  subagent_runtime_decisions: [],
  subagent_transcripts: [],
  subagent_transcript_lifecycles: [],
  subagent_approval_forwarding: [],
  subagent_inactive_policies: [],
  subagent_cancellations: [],
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

  it('builds read-only coding workflow rows from snapshot metadata', () => {
    const workflowSnapshot: ClientSnapshot = {
      ...snapshot,
      artifacts: [
        {
          artifact_id: 'artifact_diff_summary',
          kind: 'patch',
          thread_id: null,
          turn_id: null,
          task_id: 'task_workflow',
          item_id: null,
          created_at: null,
          referenced_by_event_kinds: ['patch_proposal_recorded'],
        },
      ],
      coding_workflows: [
        {
          workflow_id: 'workflow_read_only',
          task_id: 'task_workflow',
          objective: 'Add read-only workflow panels',
          active: true,
          workspace_scope: {
            workflow_id: 'workflow_read_only',
            task_id: 'task_workflow',
            root_label: 'repo:tessera',
            allowed_paths: ['crates/tui/src/lib.rs', 'apps/gui-tauri/src/App.tsx'],
            denied_paths: ['/Users/admin/work/tessera/.env'],
            mutation_mode: 'read_only_proposal',
            worktree_required: true,
            reason: 'roadmap gate',
          },
          mutation_requests: [],
          apply_patch_preflights: [],
          apply_patch_executions: [],
          patch_proposals: [
            {
              patch_id: 'patch_ui_summary',
              workflow_id: 'workflow_read_only',
              task_id: 'task_workflow',
              summary: 'Expose workflow metadata',
              touched_paths: ['apps/gui-tauri/src/App.tsx'],
              diff_artifacts: [
                {
                  kind: 'diff_artifact',
                  artifact_id: 'artifact_diff_summary',
                  trace_id: null,
                  event_range: null,
                  label: 'diff summary',
                  summary: '1 UI file',
                },
              ],
              risk_labels: ['ui-only'],
              required_checkpoint_id: null,
              reviewer_gate_id: 'gate_review',
            },
          ],
          patch_applications: [],
          test_plans: [],
          test_runs: [],
          test_evidence_summaries: [
            {
              summary_id: 'test_summary_contract',
              workflow_id: 'workflow_read_only',
              task_id: 'task_workflow',
              test_plan_ids: ['test_plan_gui'],
              test_run_ids: ['test_run_gui'],
              status: 'failed',
              total_runs: 1,
              passed_runs: 0,
              failed_runs: 1,
              cancelled_runs: 0,
              error_runs: 0,
              required_artifact_kinds: ['test_report'],
              artifact_refs: [],
              diagnostics: [
                {
                  kind: 'diagnostic_artifact',
                  artifact_id: null,
                  trace_id: null,
                  event_range: null,
                  label: 'vitest',
                  summary: 'view model helper missing',
                },
              ],
              redaction_status: 'clean',
              summary: 'GUI test failed before helper exists',
              execution_blocked: true,
            },
          ],
          review_bundles: [
            {
              review_bundle_id: 'review_bundle_ui',
              workflow_id: 'workflow_read_only',
              task_id: 'task_workflow',
              reviewer_gate_id: 'gate_review',
              patch_ids: ['patch_ui_summary'],
              test_run_ids: ['test_run_gui'],
              evidence: [],
              summary: 'Needs helper implementation',
            },
          ],
          restore_plans: [
            {
              restore_plan_id: 'restore_blocked_ui',
              workflow_id: 'workflow_read_only',
              task_id: 'task_workflow',
              checkpoint_id: 'checkpoint_before_ui',
              target_paths: ['apps/gui-tauri/src/App.tsx'],
              reason: 'blocked by read-only gate',
              execution_blocked: true,
            },
          ],
        },
      ],
    }

    expect(buildCodingWorkflowRows(workflowSnapshot)).toEqual([
      {
        id: 'workflow_read_only',
        objective: 'Add read-only workflow panels',
        active: true,
        scopeRootLabel: 'repo:tessera',
        allowedPathCount: 2,
        patchCount: 1,
        reviewCount: 1,
        testEvidenceCount: 1,
        blockedRestoreCount: 1,
        patchLabels: ['patch_ui_summary: Expose workflow metadata (apps/gui-tauri/src/App.tsx)'],
        reviewLabels: ['review_bundle_ui: Needs helper implementation'],
        testEvidenceLabels: ['test_summary_contract: failed, blocked - GUI test failed before helper exists'],
        restoreLabels: ['restore_blocked_ui: blocked by read-only gate'],
      },
    ])
  })

  it('builds read-only worktree lifecycle rows from snapshot metadata', () => {
    const worktreeSnapshot: ClientSnapshot = {
      ...snapshot,
      worktree_lifecycles: [
        {
          worktree_id: 'worktree_retained',
          workflow_id: 'workflow_read_only',
          task_id: 'task_workflow',
          trace_id: 'trace_workflow',
          source_commit: 'abc123',
          source_branch_label: 'main',
          worktree_root_label: 'isolated worktree',
          worktree_base_key: 'base-main',
          latest_status: 'retained',
          latest_reason: 'awaiting review',
          created_for_request_id: 'request_patch',
          created_for_patch_id: 'patch_ui_summary',
          evidence: [],
        },
        {
          worktree_id: 'worktree_cleanup_failed',
          workflow_id: 'workflow_read_only',
          task_id: 'task_workflow',
          trace_id: 'trace_workflow',
          source_commit: 'def456',
          source_branch_label: null,
          worktree_root_label: 'cleanup candidate',
          worktree_base_key: 'base-cleanup',
          latest_status: 'cleanup_failed',
          latest_reason: 'manual cleanup required',
          created_for_request_id: null,
          created_for_patch_id: null,
          evidence: [],
        },
      ],
    }

    const rows = buildWorktreeLifecycleRows(worktreeSnapshot)

    expect(rows).toEqual([
      {
        id: 'worktree_retained',
        workflowId: 'workflow_read_only',
        status: 'retained',
        rootLabel: 'isolated worktree',
        baseKey: 'base-main',
        reason: 'awaiting review',
      },
      {
        id: 'worktree_cleanup_failed',
        workflowId: 'workflow_read_only',
        status: 'cleanup_failed',
        rootLabel: 'cleanup candidate',
        baseKey: 'base-cleanup',
        reason: 'manual cleanup required',
      },
    ])
    expect(rows.every((row) => !('path' in row))).toBe(true)
  })
})
