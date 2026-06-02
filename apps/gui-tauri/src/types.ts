export type {
  ClientArtifact,
  ClientCodingWorkflow,
  ClientIntent,
  ClientMessage,
  ClientMessageRole,
  ClientProjection,
  ClientSnapshot,
  ClientStatus,
  ClientTask,
  ClientTelemetrySummary,
  ClientWorkflowInspection,
  ClientWorktreeLifecycle,
  GuiCommandOutcome,
  GuiEvent,
  GuiProfile,
  GuiRuntimeMode,
  GuiShellState,
  JsonValue,
  TraceEventKind,
  TraceRecord,
} from './generated/bindings'

export interface ShellMetric {
  label: string
  value: string
}

export interface CodingWorkflowRow {
  id: string
  taskId: string
  active: boolean
  scopeRootLabel: string
  allowedPathCount: number
  deniedPathCount: number
  mutationMode: string
  worktreeRequired: boolean
  patchCount: number
  reviewCount: number
  testPlanCount: number
  testRunCount: number
  testEvidenceCount: number
  failedTestEvidenceCount: number
  blockedTestEvidenceCount: number
  restoreCount: number
  blockedRestoreCount: number
}

export interface WorktreeLifecycleRow {
  id: string
  workflowId: string
  status: string
  rootLabel: string
  baseKey: string
}

export interface WorkflowInspectionRow {
  workflowRef: string
  taskRef: string
  active: boolean
  mutationMode: string
  worktreeRequired: boolean
  patchCount: number
  diffArtifactRefCount: number
  patchesRequiringReviewCount: number
  reviewBundleCount: number
  reviewEvidenceRefCount: number
  reviewerGateCount: number
  acceptedReviewerGateCount: number
  rejectedReviewerGateCount: number
  revisionRequestedReviewerGateCount: number
  pendingReviewerGateCount: number
  approvalCount: number
  pendingApprovalCount: number
  resolvedApprovalCount: number
  applyPatchPreflightCount: number
  executorReadyPreflightCount: number
  executorBlockedPreflightCount: number
  applyPatchExecutionCount: number
  successfulApplyPatchExecutionCount: number
  failedApplyPatchExecutionCount: number
}
