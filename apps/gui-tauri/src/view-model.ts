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
    return {
      id: workflow.workflow_id,
      objective: workflow.objective ?? 'Untitled workflow',
      active: workflow.active,
      scopeRootLabel: workflow.workspace_scope?.root_label ?? 'unscoped',
      allowedPathCount: workflow.workspace_scope?.allowed_paths.length ?? 0,
      patchCount: workflow.patch_proposals.length,
      reviewCount: workflow.review_bundles.length,
      testEvidenceCount: workflow.test_evidence_summaries.length,
      blockedRestoreCount: blockedRestores.length,
      patchLabels: workflow.patch_proposals.map((patch) => {
        const pathLabel = patch.touched_paths.length > 0 ? ` (${patch.touched_paths.join(', ')})` : ''
        return `${patch.patch_id}: ${patch.summary}${pathLabel}`
      }),
      reviewLabels: workflow.review_bundles.map(
        (bundle) => `${bundle.review_bundle_id}: ${bundle.summary}`,
      ),
      testEvidenceLabels: workflow.test_evidence_summaries.map((summary) => {
        const blocked = summary.execution_blocked ? ', blocked' : ''
        return `${summary.summary_id}: ${summary.status}${blocked} - ${summary.summary}`
      }),
      restoreLabels: blockedRestores.map((plan) => `${plan.restore_plan_id}: ${plan.reason}`),
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
    reason: lifecycle.latest_reason ?? 'No reason recorded',
  }))
}
