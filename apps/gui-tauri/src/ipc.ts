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
        status: {
          ...fallbackSnapshot.status,
          context_handles_summary: 'context 0 handles / 0/0 tokens',
          coding_workflow_summary: 'workflows 0',
          worktree_summary: 'worktrees 0',
        },
        projection: { ...fallbackSnapshot.projection, messages: [] },
        tasks: [],
        artifacts: [],
        coding_workflows: [],
        worktree_lifecycles: [],
        context_handles: [],
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

  if ('cancel_task' in intent) {
    appendSystemMessage('Cancel requested for mock/replay projection only.')
    return accepted('Cancel recorded in mock/replay mode.')
  }

  if ('pause_task' in intent) {
    const taskLabel = intent.pause_task.task_id ?? 'latest running task'
    appendSystemMessage(
      `Pause requested for ${taskLabel} as typed metadata; no runtime execution was invoked.`,
    )
    return accepted('Pause task intent accepted as typed metadata with no runtime execution.')
  }

  if ('resume_task' in intent) {
    appendSystemMessage(
      `Resume requested for ${intent.resume_task.task_id} as typed metadata; no runtime execution was invoked.`,
    )
    return accepted('Resume task intent accepted as typed metadata with no runtime execution.')
  }

  return accepted('Intent accepted by the browser fallback without runtime execution.')
}

function accepted(notice: string): GuiCommandOutcome {
  return {
    accepted: true,
    notice,
    snapshot: fallbackSnapshot,
  }
}

function appendSystemMessage(content: string): void {
  fallbackSnapshot = {
    ...fallbackSnapshot,
    projection: {
      ...fallbackSnapshot.projection,
      messages: [
        ...fallbackSnapshot.projection.messages,
        {
          role: 'system',
          content,
          item_id: null,
          streaming: false,
        },
      ],
    },
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
      handoff_summary: 'handoffs 0',
      coding_workflow_summary: 'workflows 1 active',
      worktree_summary: 'worktrees 1 retained',
      subagent_summary: 'subagents 0',
      subagent_runtime_summary: 'subagent runtime 0',
      subagent_transcript_lifecycle_summary: 'subagent transcripts 0',
      usage_summary: 'usage in 0 / out 0 / total 0',
      cache_summary: 'cache 0/0',
      cost_summary: 'CNY 0.0000',
      context_summary: 'ctx 0 tokens',
      context_handles_summary: 'context 0 handles / 0/0 tokens',
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
    artifacts: [
      {
        artifact_id: 'artifact_web_patch_summary',
        kind: 'patch',
        thread_id: null,
        turn_id: null,
        task_id: 'task_web_workflow',
        item_id: null,
        created_at: null,
        referenced_by_event_kinds: ['patch_proposal_recorded'],
      },
    ],
    approvals: [],
    memory_proposals: [],
    handoffs: [],
    reviewer_gates: [],
    coding_workflows: [
      {
        workflow_id: 'workflow_web_read_only',
        task_id: 'task_web_workflow',
        objective: 'Project read-only workflow metadata',
        active: true,
        workspace_scope: {
          workflow_id: 'workflow_web_read_only',
          task_id: 'task_web_workflow',
          root_label: 'repo:tessera',
          allowed_paths: ['apps/gui-tauri/src/App.tsx', 'apps/gui-tauri/src/view-model.ts'],
          denied_paths: [],
          mutation_mode: 'read_only_proposal',
          worktree_required: true,
          reason: 'GUI projection only',
        },
        mutation_requests: [],
        apply_patch_preflights: [],
        apply_patch_executions: [],
        patch_proposals: [
          {
            patch_id: 'patch_web_projection',
            workflow_id: 'workflow_web_read_only',
            task_id: 'task_web_workflow',
            summary: 'Render compact metadata rows',
            touched_paths: ['apps/gui-tauri/src/App.tsx'],
            diff_artifacts: [
              {
                kind: 'diff_artifact',
                artifact_id: 'artifact_web_patch_summary',
                trace_id: null,
                event_range: null,
                label: 'patch metadata',
                summary: 'UI projection summary',
              },
            ],
            risk_labels: ['read-only'],
            required_checkpoint_id: null,
            reviewer_gate_id: 'gate_web_review',
          },
        ],
        patch_applications: [],
        test_plans: [],
        test_runs: [],
        test_evidence_summaries: [
          {
            summary_id: 'test_web_projection',
            workflow_id: 'workflow_web_read_only',
            task_id: 'task_web_workflow',
            test_plan_ids: [],
            test_run_ids: [],
            status: 'incomplete',
            total_runs: 0,
            passed_runs: 0,
            failed_runs: 0,
            cancelled_runs: 0,
            error_runs: 0,
            required_artifact_kinds: ['test_report'],
            artifact_refs: [],
            diagnostics: [],
            redaction_status: 'clean',
            summary: 'No browser fallback tests run yet',
            execution_blocked: true,
          },
        ],
        review_bundles: [
          {
            review_bundle_id: 'review_web_projection',
            workflow_id: 'workflow_web_read_only',
            task_id: 'task_web_workflow',
            reviewer_gate_id: 'gate_web_review',
            patch_ids: ['patch_web_projection'],
            test_run_ids: [],
            evidence: [],
            summary: 'Read-only metadata ready for review',
          },
        ],
        restore_plans: [
          {
            restore_plan_id: 'restore_web_blocked',
            workflow_id: 'workflow_web_read_only',
            task_id: 'task_web_workflow',
            checkpoint_id: 'checkpoint_web_seed',
            target_paths: ['apps/gui-tauri/src/App.tsx'],
            reason: 'Restore execution is outside GUI surface',
            execution_blocked: true,
          },
        ],
      },
    ],
    worktree_lifecycles: [
      {
        worktree_id: 'worktree_web_read_only',
        workflow_id: 'workflow_web_read_only',
        task_id: 'task_web_workflow',
        trace_id: 'trace_web_seed',
        source_commit: 'mock',
        source_branch_label: 'mock-replay',
        worktree_root_label: 'isolated projection',
        worktree_base_key: 'mock-base',
        latest_status: 'retained',
        latest_reason: 'available for read-only inspection',
        created_for_request_id: null,
        created_for_patch_id: 'patch_web_projection',
        evidence: [],
      },
    ],
    subagent_sessions: [],
    subagent_runtime_decisions: [],
    subagent_transcripts: [],
    subagent_transcript_lifecycles: [],
    subagent_approval_forwarding: [],
    subagent_inactive_policies: [],
    subagent_cancellations: [],
    context_handles: [],
    draft_input: '',
  }
}
