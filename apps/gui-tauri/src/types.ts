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
  objective: string
  active: boolean
  scopeRootLabel: string
  allowedPathCount: number
  patchCount: number
  reviewCount: number
  testEvidenceCount: number
  blockedRestoreCount: number
  patchLabels: string[]
  reviewLabels: string[]
  testEvidenceLabels: string[]
  restoreLabels: string[]
}

export interface WorktreeLifecycleRow {
  id: string
  workflowId: string
  status: string
  rootLabel: string
  baseKey: string
  reason: string
}
