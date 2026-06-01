import type {
  ClientMessage,
  ClientSnapshot,
  CodingWorkflowRow,
  ShellMetric,
  WorktreeLifecycleRow,
} from './types'

export function buildShellMetrics(snapshot: ClientSnapshot): ShellMetric[] {
  return [
    { label: 'Profile', value: snapshot.status.active_profile },
    { label: 'Task', value: snapshot.status.task_summary },
    { label: 'Usage', value: snapshot.status.usage_summary },
    { label: 'Cache', value: snapshot.status.cache_summary },
    { label: 'Context', value: snapshot.status.context_summary },
  ]
}

export function visibleMessages(snapshot: ClientSnapshot): ClientMessage[] {
  return snapshot.projection.messages.filter((message) => message.content.trim().length > 0)
}

export function buildCodingWorkflowRows(snapshot: ClientSnapshot): CodingWorkflowRow[] {
  return snapshot.coding_workflows.map((workflow) => {
    const blockedRestores = workflow.restore_plans.filter((plan) => plan.execution_blocked)
    const failedTestEvidence = workflow.test_evidence_summaries.filter(
      (summary) => summary.status === 'failed' || summary.status === 'error',
    )
    const blockedTestEvidence = workflow.test_evidence_summaries.filter(
      (summary) => summary.execution_blocked,
    )
    return {
      id: workflow.workflow_id,
      taskId: workflow.task_id,
      objective: workflow.objective ?? 'Untitled workflow',
      active: workflow.active,
      scopeRootLabel: workflow.workspace_scope?.root_label ?? 'unscoped',
      allowedPathCount: workflow.workspace_scope?.allowed_paths.length ?? 0,
      deniedPathCount: workflow.workspace_scope?.denied_paths.length ?? 0,
      mutationMode: workflow.workspace_scope?.mutation_mode ?? 'unscoped',
      worktreeRequired: workflow.workspace_scope?.worktree_required ?? false,
      patchCount: workflow.patch_proposals.length,
      reviewCount: workflow.review_bundles.length,
      testPlanCount: workflow.test_plans.length,
      testRunCount: workflow.test_runs.length,
      testEvidenceCount: workflow.test_evidence_summaries.length,
      failedTestEvidenceCount: failedTestEvidence.length,
      blockedTestEvidenceCount: blockedTestEvidence.length,
      restoreCount: workflow.restore_plans.length,
      blockedRestoreCount: blockedRestores.length,
    }
  })
}

export function buildWorktreeLifecycleRows(snapshot: ClientSnapshot): WorktreeLifecycleRow[] {
  return snapshot.worktree_lifecycles.map((lifecycle) => ({
    id: lifecycle.worktree_id,
    workflowId: lifecycle.workflow_id,
    status: lifecycle.latest_status,
    rootLabel: lifecycle.worktree_root_label,
    baseKey: lifecycle.worktree_base_key,
  }))
}
