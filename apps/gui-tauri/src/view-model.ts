import type {
  ClientMessage,
  ClientSnapshot,
  CodingWorkflowRow,
  ShellMetric,
  WorkflowInspectionRow,
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

export function buildWorkflowInspectionRows(snapshot: ClientSnapshot): WorkflowInspectionRow[] {
  return snapshot.workflow_inspections.map((inspection) => ({
    workflowRef: inspection.workflow_ref,
    taskRef: inspection.task_ref,
    active: inspection.active,
    mutationMode: inspection.mutation_mode ?? 'unscoped',
    worktreeRequired: inspection.worktree_required,
    patchCount: inspection.patch_count,
    diffArtifactRefCount: inspection.diff_artifact_ref_count,
    patchesRequiringReviewCount: inspection.patches_requiring_review_count,
    reviewBundleCount: inspection.review_bundle_count,
    reviewEvidenceRefCount: inspection.review_evidence_ref_count,
    reviewerGateCount: inspection.reviewer_gate_count,
    acceptedReviewerGateCount: inspection.accepted_reviewer_gate_count,
    rejectedReviewerGateCount: inspection.rejected_reviewer_gate_count,
    revisionRequestedReviewerGateCount: inspection.revision_requested_reviewer_gate_count,
    pendingReviewerGateCount: inspection.pending_reviewer_gate_count,
    approvalCount: inspection.approval_count,
    pendingApprovalCount: inspection.pending_approval_count,
    resolvedApprovalCount: inspection.resolved_approval_count,
    applyPatchPreflightCount: inspection.apply_patch_preflight_count,
    executorReadyPreflightCount: inspection.executor_ready_preflight_count,
    executorBlockedPreflightCount: inspection.executor_blocked_preflight_count,
    applyPatchExecutionCount: inspection.apply_patch_execution_count,
    successfulApplyPatchExecutionCount: inspection.successful_apply_patch_execution_count,
    failedApplyPatchExecutionCount: inspection.failed_apply_patch_execution_count,
  }))
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
