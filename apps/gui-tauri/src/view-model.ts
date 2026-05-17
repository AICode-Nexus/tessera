import type { ClientMessage, ClientSnapshot, ShellMetric } from './types'

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
