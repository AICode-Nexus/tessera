import { describe, expect, it } from 'vitest'

import { allowedCommandNames, forbiddenCommandNames } from './ipc'
import {
  buildCodingWorkflowRows,
  buildShellMetrics,
  buildWorkflowInspectionRows,
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
    workflow_inspection_summary:
      'inspection workflows 0 / review gates 0 accepted / approvals 0 pending / diff refs 0',
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
  workflow_inspections: [],
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
    const sensitiveObjective =
      'Fix /Users/admin/work/tessera/.env with token sk-secret-workflow-objective'
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
          objective: sensitiveObjective,
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
        taskId: 'task_workflow',
        active: true,
        scopeRootLabel: 'repo:tessera',
        allowedPathCount: 2,
        deniedPathCount: 1,
        mutationMode: 'read_only_proposal',
        worktreeRequired: true,
        patchCount: 1,
        reviewCount: 1,
        testPlanCount: 0,
        testRunCount: 0,
        testEvidenceCount: 1,
        failedTestEvidenceCount: 1,
        blockedTestEvidenceCount: 1,
        restoreCount: 1,
        blockedRestoreCount: 1,
      },
    ])
    expect(JSON.stringify(buildCodingWorkflowRows(workflowSnapshot))).not.toContain(
      'apps/gui-tauri/src/App.tsx',
    )
    expect(JSON.stringify(buildCodingWorkflowRows(workflowSnapshot))).not.toContain(
      '/Users/admin/work/tessera/.env',
    )
    expect(JSON.stringify(buildCodingWorkflowRows(workflowSnapshot))).not.toContain(
      'sk-secret-workflow-objective',
    )
    expect(JSON.stringify(buildCodingWorkflowRows(workflowSnapshot))).not.toContain(
      'Needs helper implementation',
    )
    expect(JSON.stringify(buildCodingWorkflowRows(workflowSnapshot))).not.toContain(
      'GUI test failed before helper exists',
    )
    expect(JSON.stringify(buildCodingWorkflowRows(workflowSnapshot))).not.toContain(
      'blocked by read-only gate',
    )
  })

  it('builds safe workflow inspection rows from projected inspection metadata', () => {
    const inspectionSnapshot: ClientSnapshot = {
      ...snapshot,
      status: {
        ...snapshot.status,
        workflow_inspection_summary:
          'inspection workflows 1 / review gates 2 accepted / approvals 1 pending / diff refs 3',
      },
      workflow_inspections: [
        {
          workflow_ref: 'workflow:1',
          task_ref: 'task:1',
          active: true,
          mutation_mode: 'read_only_proposal',
          worktree_required: true,
          patch_count: 2,
          diff_artifact_ref_count: 3,
          patches_requiring_review_count: 1,
          review_bundle_count: 4,
          review_evidence_ref_count: 5,
          reviewer_gate_count: 6,
          accepted_reviewer_gate_count: 2,
          rejected_reviewer_gate_count: 1,
          revision_requested_reviewer_gate_count: 1,
          pending_reviewer_gate_count: 2,
          approval_count: 3,
          pending_approval_count: 1,
          resolved_approval_count: 2,
          apply_patch_preflight_count: 4,
          executor_ready_preflight_count: 1,
          executor_blocked_preflight_count: 3,
          apply_patch_execution_count: 2,
          successful_apply_patch_execution_count: 1,
          failed_apply_patch_execution_count: 1,
        },
      ],
      approvals: [
        {
          approval_id: 'approval_raw_secret_id',
          call_id: 'call_raw_secret_id',
          tool_id: 'tool_raw_secret_id',
          status: 'pending',
          reason: 'approval reason mentions /Users/admin/work/tessera/.env and sk-secret-approval',
          required_permissions: ['permission-label-secret'],
          side_effects: ['would touch private path'],
        },
      ],
      reviewer_gates: [
        {
          gate_id: 'gate_raw_secret_id',
          handoff_id: 'handoff_raw_secret_id',
          parent_task_id: 'task_sensitive_raw',
          status: 'pending',
          requested_decisions: ['accept', 'reject'],
          decision: null,
          reviewer: 'reviewer-private-label',
          reason_code: 'reason-code-secret',
          comment: 'reviewer comment leaks /tmp/private-review and sk-secret-comment',
          evidence: [
            {
              kind: 'diagnostic_artifact',
              artifact_id: 'artifact_raw_secret_id',
              trace_id: 'trace_raw_secret_id',
              event_range: null,
              label: 'diagnostic-label-secret',
              summary: 'diagnostic summary secret',
            },
          ],
        },
      ],
      coding_workflows: [
        {
          workflow_id: 'workflow_sensitive_raw',
          task_id: 'task_sensitive_raw',
          objective: 'objective mentions /Users/admin/work/tessera/.env and sk-secret-objective',
          active: true,
          workspace_scope: {
            workflow_id: 'workflow_sensitive_raw',
            task_id: 'task_sensitive_raw',
            root_label: 'repo-secret-label',
            allowed_paths: ['apps/gui-tauri/src/App.tsx'],
            denied_paths: ['/Users/admin/work/tessera/.env'],
            mutation_mode: 'read_only_proposal',
            worktree_required: true,
            reason: 'scope reason secret',
          },
          mutation_requests: [
            {
              request_id: 'request_raw_secret_id',
              workflow_id: 'workflow_sensitive_raw',
              task_id: 'task_sensitive_raw',
              operation: 'patch_application',
              status: 'reviewer_pending',
              summary: 'mutation request summary secret',
              requested_paths: ['/tmp/requested-private-path'],
              required_checkpoint_id: null,
              reviewer_gate_id: 'gate_raw_secret_id',
              policy_decision_id: null,
              sandbox_profile_label: 'sandbox-label-secret',
              worktree_required: true,
              evidence: [],
            },
          ],
          apply_patch_preflights: [
            {
              preflight_id: 'preflight_raw_secret_id',
              workflow_id: 'workflow_sensitive_raw',
              task_id: 'task_sensitive_raw',
              request_id: 'request_raw_secret_id',
              patch_id: 'patch_raw_secret_id',
              status: 'blocked',
              blockers: ['unsafe_patch_path'],
              affected_paths: ['/tmp/preflight-private-path'],
              operations: [{ path: '/tmp/preflight-operation-path', operation: 'modify' }],
              executor_blocked: true,
              executor_block_reason: 'executor reason secret',
              evidence: [],
            },
          ],
          apply_patch_executions: [
            {
              execution_id: 'execution_raw_secret_id',
              preflight_id: 'preflight_raw_secret_id',
              workflow_id: 'workflow_sensitive_raw',
              task_id: 'task_sensitive_raw',
              request_id: 'request_raw_secret_id',
              patch_id: 'patch_raw_secret_id',
              checkpoint_id: null,
              reviewer_gate_id: 'gate_raw_secret_id',
              policy_decision_id: null,
              sandbox_profile_label: 'execution-sandbox-label',
              isolated_root_label: 'isolated-root-secret',
              executor_label: 'executor-label-secret',
              status: 'failed',
              blockers: ['unsafe_path'],
              affected_paths: ['/tmp/execution-private-path'],
              conflict_paths: ['/tmp/conflict-private-path'],
              artifact_refs: ['artifact_raw_secret_id'],
              evidence: [],
            },
          ],
          patch_proposals: [
            {
              patch_id: 'patch_raw_secret_id',
              workflow_id: 'workflow_sensitive_raw',
              task_id: 'task_sensitive_raw',
              summary: 'patch summary secret',
              touched_paths: ['apps/gui-tauri/src/App.tsx'],
              diff_artifacts: [
                {
                  kind: 'diff_artifact',
                  artifact_id: 'artifact_raw_secret_id',
                  trace_id: 'trace_raw_secret_id',
                  event_range: null,
                  label: 'diff-label-secret',
                  summary: 'diff summary secret',
                },
              ],
              risk_labels: ['risk-label-secret'],
              required_checkpoint_id: null,
              reviewer_gate_id: 'gate_raw_secret_id',
            },
          ],
          patch_applications: [
            {
              patch_id: 'patch_raw_secret_id',
              workflow_id: 'workflow_sensitive_raw',
              task_id: 'task_sensitive_raw',
              checkpoint_id: 'checkpoint_raw_secret_id',
              outcome: 'conflict',
              conflict_paths: ['/tmp/conflict-private-path'],
              applied_paths: ['/tmp/applied-private-path'],
              artifact_refs: ['artifact_raw_secret_id'],
            },
          ],
          test_plans: [
            {
              test_plan_id: 'test_plan_raw_secret_id',
              workflow_id: 'workflow_sensitive_raw',
              task_id: 'task_sensitive_raw',
              command_labels: ['npm test secret command label'],
              affected_paths: ['apps/gui-tauri/src/view-model.ts'],
              required_artifact_kinds: ['test_report'],
            },
          ],
          test_runs: [
            {
              test_run_id: 'test_run_raw_secret_id',
              workflow_id: 'workflow_sensitive_raw',
              task_id: 'task_sensitive_raw',
              test_plan_id: 'test_plan_raw_secret_id',
              command_label: 'npm test raw command secret',
              status: 'failed',
              exit_code: 1,
              duration_ms: 42,
              stdout_artifact_id: 'stdout_artifact_raw_secret_id',
              stderr_artifact_id: 'stderr_artifact_raw_secret_id',
              diagnostics: [
                {
                  kind: 'diagnostic_artifact',
                  artifact_id: 'artifact_raw_secret_id',
                  trace_id: null,
                  event_range: null,
                  label: 'test-run-diagnostic-label-secret',
                  summary: 'test run diagnostic summary secret',
                },
              ],
              redaction_status: 'clean',
            },
          ],
          test_evidence_summaries: [
            {
              summary_id: 'summary_raw_secret_id',
              workflow_id: 'workflow_sensitive_raw',
              task_id: 'task_sensitive_raw',
              test_plan_ids: ['test_plan_raw_secret_id'],
              test_run_ids: ['test_run_raw_secret_id'],
              status: 'failed',
              total_runs: 1,
              passed_runs: 0,
              failed_runs: 1,
              cancelled_runs: 0,
              error_runs: 0,
              required_artifact_kinds: ['test_report'],
              artifact_refs: ['artifact_raw_secret_id'],
              diagnostics: [
                {
                  kind: 'diagnostic_artifact',
                  artifact_id: 'artifact_raw_secret_id',
                  trace_id: null,
                  event_range: null,
                  label: 'evidence-diagnostic-label-secret',
                  summary: 'evidence diagnostic summary secret',
                },
              ],
              redaction_status: 'clean',
              summary: 'test evidence summary secret',
              execution_blocked: true,
            },
          ],
          review_bundles: [
            {
              review_bundle_id: 'review_bundle_raw_secret_id',
              workflow_id: 'workflow_sensitive_raw',
              task_id: 'task_sensitive_raw',
              reviewer_gate_id: 'gate_raw_secret_id',
              patch_ids: ['patch_raw_secret_id'],
              test_run_ids: ['test_run_raw_secret_id'],
              evidence: [
                {
                  kind: 'summary_artifact',
                  artifact_id: 'artifact_raw_secret_id',
                  trace_id: null,
                  event_range: null,
                  label: 'review-evidence-label-secret',
                  summary: 'review evidence summary secret',
                },
              ],
              summary: 'review bundle summary secret',
            },
          ],
          restore_plans: [
            {
              restore_plan_id: 'restore_plan_raw_secret_id',
              workflow_id: 'workflow_sensitive_raw',
              task_id: 'task_sensitive_raw',
              checkpoint_id: 'checkpoint_raw_secret_id',
              target_paths: ['/tmp/restore-private-path'],
              reason: 'restore reason secret',
              execution_blocked: true,
            },
          ],
        },
      ],
    }

    const rows = buildWorkflowInspectionRows(inspectionSnapshot)
    const rowJson = JSON.stringify(rows)

    expect(rows).toEqual([
      {
        workflowRef: 'workflow:1',
        taskRef: 'task:1',
        active: true,
        mutationMode: 'read_only_proposal',
        worktreeRequired: true,
        patchCount: 2,
        diffArtifactRefCount: 3,
        patchesRequiringReviewCount: 1,
        reviewBundleCount: 4,
        reviewEvidenceRefCount: 5,
        reviewerGateCount: 6,
        acceptedReviewerGateCount: 2,
        rejectedReviewerGateCount: 1,
        revisionRequestedReviewerGateCount: 1,
        pendingReviewerGateCount: 2,
        approvalCount: 3,
        pendingApprovalCount: 1,
        resolvedApprovalCount: 2,
        applyPatchPreflightCount: 4,
        executorReadyPreflightCount: 1,
        executorBlockedPreflightCount: 3,
        applyPatchExecutionCount: 2,
        successfulApplyPatchExecutionCount: 1,
        failedApplyPatchExecutionCount: 1,
      },
    ])
    expect(rowJson).not.toContain('workflow_sensitive_raw')
    expect(rowJson).not.toContain('task_sensitive_raw')
    expect(rowJson).not.toContain('gate_raw_secret_id')
    expect(rowJson).not.toContain('approval_raw_secret_id')
    expect(rowJson).not.toContain('artifact_raw_secret_id')
    expect(rowJson).not.toContain('repo-secret-label')
    expect(rowJson).not.toContain('diff-label-secret')
    expect(rowJson).not.toContain('diagnostic-label-secret')
    expect(rowJson).not.toContain('reviewer-private-label')
    expect(rowJson).not.toContain('apps/gui-tauri/src/App.tsx')
    expect(rowJson).not.toContain('/Users/admin/work/tessera/.env')
    expect(rowJson).not.toContain('/tmp/private-review')
    expect(rowJson).not.toContain('objective mentions')
    expect(rowJson).not.toContain('patch summary secret')
    expect(rowJson).not.toContain('review bundle summary secret')
    expect(rowJson).not.toContain('reviewer comment leaks')
    expect(rowJson).not.toContain('diagnostic summary secret')
    expect(rowJson).not.toContain('scope reason secret')
    expect(rowJson).not.toContain('npm test raw command secret')
    expect(rowJson).not.toContain('sk-secret')
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
          latest_reason: 'retained at /Users/admin/work/tessera/.env with token sk-secret',
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
          latest_reason: 'manual cleanup required for /tmp/private-worktree',
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
      },
      {
        id: 'worktree_cleanup_failed',
        workflowId: 'workflow_read_only',
        status: 'cleanup_failed',
        rootLabel: 'cleanup candidate',
        baseKey: 'base-cleanup',
      },
    ])
    expect(rows.every((row) => !('path' in row))).toBe(true)
    expect(rows.every((row) => !('reason' in row))).toBe(true)
    expect(JSON.stringify(rows)).not.toContain('/Users/admin/work/tessera/.env')
    expect(JSON.stringify(rows)).not.toContain('sk-secret')
    expect(JSON.stringify(rows)).not.toContain('/tmp/private-worktree')
  })
})
