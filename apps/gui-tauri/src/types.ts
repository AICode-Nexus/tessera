export type {
  ClientArtifact,
  ClientIntent,
  ClientMessage,
  ClientMessageRole,
  ClientProjection,
  ClientSnapshot,
  ClientStatus,
  ClientTask,
  ClientTelemetrySummary,
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
