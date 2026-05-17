import { invoke } from '@tauri-apps/api/core'

import type {
  ClientIntent,
  ClientSnapshot,
  GuiCommandOutcome,
  GuiProfile,
  GuiShellState,
} from './types'

export const allowedCommandNames = [
  'list_profiles',
  'load_client_snapshot',
  'submit_client_intent',
  'cancel_task',
  'load_trace_projection',
  'export_thread',
] as const

export const forbiddenCommandNames = [
  'call_provider',
  'read_sql',
  'write_trace',
  'execute_shell',
  'run_tool',
  'read_env_secret',
] as const

const profiles: GuiProfile[] = [
  { id: 'mock-replay', label: 'Mock Replay', mode: 'mock_replay' },
  { id: 'read-only', label: 'Read Only Runtime', mode: 'read_only' },
]

let fallbackSeq = 1
let fallbackSnapshot: ClientSnapshot = createFallbackSnapshot()

export async function listProfiles(): Promise<GuiProfile[]> {
  return callTauri('list_profiles', undefined, () => profiles)
}

export async function loadShellState(): Promise<GuiShellState> {
  if (isTauriRuntime()) {
    const [tauriProfiles, snapshot] = await Promise.all([
      invoke<GuiProfile[]>('list_profiles'),
      invoke<ClientSnapshot>('load_client_snapshot'),
    ])
    const activeProfile = tauriProfiles.find((profile) => profile.id === snapshot.status.active_profile)
    return {
      ipc_version: 1,
      mode: activeProfile?.mode ?? 'mock_replay',
      event_buffer_capacity: 64,
      profiles: tauriProfiles,
      snapshot,
    }
  }

  return {
    ipc_version: 1,
    mode: 'mock_replay',
    event_buffer_capacity: 64,
    profiles,
    snapshot: fallbackSnapshot,
  }
}

export async function loadClientSnapshot(): Promise<ClientSnapshot> {
  return callTauri('load_client_snapshot', undefined, () => fallbackSnapshot)
}

export async function submitClientIntent(intent: ClientIntent): Promise<GuiCommandOutcome> {
  return callTauri('submit_client_intent', { intent }, () => submitFallbackIntent(intent))
}

export async function cancelTask(taskId: string | null): Promise<GuiCommandOutcome> {
  return callTauri('cancel_task', { task_id: taskId }, () =>
    submitFallbackIntent({ cancel_task: { task_id: taskId } }),
  )
}

export async function exportThread(): Promise<string> {
  return callTauri('export_thread', undefined, () => {
    const lines = ['# Tessera Export', '']
    for (const message of fallbackSnapshot.projection.messages) {
      if (message.content.trim().length === 0) continue
      lines.push(`## ${message.role}`, '', message.content, '')
    }
    return lines.join('\n')
  })
}

async function callTauri<T>(
  command: string,
  args: Record<string, unknown> | undefined,
  fallback: () => T,
): Promise<T> {
  if (isTauriRuntime()) {
    return invoke<T>(command, args)
  }
  return fallback()
}

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

function submitFallbackIntent(intent: ClientIntent): GuiCommandOutcome {
  if (typeof intent === 'string') {
    if (intent === 'new_thread') {
      fallbackSnapshot = {
        ...fallbackSnapshot,
        projection: { ...fallbackSnapshot.projection, messages: [] },
        tasks: [],
        artifacts: [],
        draft_input: '',
      }
      return accepted('Started a new GUI projection thread.')
    }
    return accepted('Intent accepted by the browser fallback.')
  }

  if ('switch_profile' in intent) {
    fallbackSnapshot = {
      ...fallbackSnapshot,
      status: {
        ...fallbackSnapshot.status,
        active_profile: intent.switch_profile.profile_id,
      },
    }
    return accepted('Profile switched in client projection only.')
  }

  if ('submit_prompt' in intent) {
    const prompt = intent.submit_prompt.prompt.trim()
    if (prompt.length === 0) return accepted('Empty prompt ignored.')
    fallbackSnapshot = {
      ...fallbackSnapshot,
      status: {
        ...fallbackSnapshot.status,
        active_profile: intent.submit_prompt.profile_id,
        usage_summary: `usage in ${prompt.length} / out 88 / total ${prompt.length + 88}`,
        cache_summary: 'cache 0/0',
        context_summary: `ctx ${prompt.length} tokens`,
      },
      projection: {
        ...fallbackSnapshot.projection,
        messages: [
          ...fallbackSnapshot.projection.messages,
          {
            role: 'user',
            content: prompt,
            item_id: `item_web_user_${fallbackSeq}`,
            streaming: false,
          },
          {
            role: 'assistant',
            content:
              'mock/replay response accepted by the GUI shell. Live provider execution stays outside this spike.',
            item_id: `item_web_assistant_${fallbackSeq}`,
            streaming: false,
          },
        ],
      },
    }
    fallbackSeq += 1
    return accepted('Prompt projected with mock/replay events.')
  }

  fallbackSnapshot = {
    ...fallbackSnapshot,
    projection: {
      ...fallbackSnapshot.projection,
      messages: [
        ...fallbackSnapshot.projection.messages,
        {
          role: 'system',
          content: 'Cancel requested for mock/replay projection only.',
          item_id: null,
          streaming: false,
        },
      ],
    },
  }
  return accepted('Cancel recorded in mock/replay mode.')
}

function accepted(notice: string): GuiCommandOutcome {
  return {
    accepted: true,
    notice,
    snapshot: fallbackSnapshot,
  }
}

function createFallbackSnapshot(): ClientSnapshot {
  return {
    status: {
      active_profile: 'mock-replay',
      available_profiles: profiles.map((profile) => profile.id),
      reasoning_visible: false,
      task_summary: 'task idle',
      artifact_summary: 'artifacts 0',
      approval_summary: 'approvals 0 pending',
      memory_summary: 'memory 0 pending',
      usage_summary: 'usage in 0 / out 0 / total 0',
      cache_summary: 'cache 0/0',
      cost_summary: 'CNY 0.0000',
      context_summary: 'ctx 0 tokens',
      telemetry: {
        input_tokens: 0,
        output_tokens: 0,
        total_tokens: 0,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        cache_miss_tokens: 0,
        cache_total_tokens: 0,
        latest_context_tokens: null,
        max_context_tokens: null,
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
          content: 'Show me the GUI runtime boundary.',
          item_id: 'item_web_user_seed',
          streaming: false,
        },
        {
          role: 'assistant',
          content:
            'This mock/replay snapshot is projected through tessera-client; no provider or storage path is active.',
          item_id: 'item_web_assistant_seed',
          streaming: false,
        },
      ],
    },
    tasks: [],
    artifacts: [],
    approvals: [],
    memory_proposals: [],
    draft_input: '',
  }
}
