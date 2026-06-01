import { useEffect, useMemo, useState } from 'react'
import { Download, LoaderCircle, Plus, Send, Square, TerminalSquare } from 'lucide-react'

import {
  cancelTask,
  exportThread,
  loadShellState,
  submitClientIntent,
} from './ipc'
import type { ClientMessage, ClientSnapshot, GuiProfile, GuiShellState } from './types'
import {
  buildCodingWorkflowRows,
  buildShellMetrics,
  buildWorkflowInspectionRows,
  buildWorktreeLifecycleRows,
  visibleMessages,
} from './view-model'

function App() {
  const [shellState, setShellState] = useState<GuiShellState | null>(null)
  const [snapshot, setSnapshot] = useState<ClientSnapshot | null>(null)
  const [profiles, setProfiles] = useState<GuiProfile[]>([])
  const [draft, setDraft] = useState('')
  const [notice, setNotice] = useState('Loading GUI shell...')
  const [busy, setBusy] = useState(false)
  const metrics = useMemo(() => (snapshot ? buildShellMetrics(snapshot) : []), [snapshot])
  const messages = useMemo(() => (snapshot ? visibleMessages(snapshot) : []), [snapshot])
  const workflowRows = useMemo(
    () => (snapshot ? buildCodingWorkflowRows(snapshot) : []),
    [snapshot],
  )
  const workflowInspectionRows = useMemo(
    () => (snapshot ? buildWorkflowInspectionRows(snapshot) : []),
    [snapshot],
  )
  const worktreeRows = useMemo(
    () => (snapshot ? buildWorktreeLifecycleRows(snapshot) : []),
    [snapshot],
  )

  useEffect(() => {
    let mounted = true
    loadShellState()
      .then((state) => {
        if (!mounted) return
        setShellState(state)
        setSnapshot(state.snapshot)
        setProfiles(state.profiles)
        setNotice('Mock/replay projection loaded')
      })
      .catch((error: unknown) => {
        if (!mounted) return
        setNotice(error instanceof Error ? error.message : String(error))
      })
    return () => {
      mounted = false
    }
  }, [])

  async function submitPrompt() {
    const prompt = draft.trim()
    if (!snapshot || prompt.length === 0) return
    setBusy(true)
    try {
      const outcome = await submitClientIntent({
        submit_prompt: {
          profile_id: snapshot.status.active_profile,
          prompt,
        },
      })
      setDraft('')
      setSnapshot(outcome.snapshot)
      setNotice(outcome.notice ?? 'Prompt projected')
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error))
    } finally {
      setBusy(false)
    }
  }

  async function switchProfile(profileId: string) {
    if (!snapshot) return
    const outcome = await submitClientIntent({ switch_profile: { profile_id: profileId } })
    setSnapshot(outcome.snapshot)
    setNotice(outcome.notice ?? 'Profile switched')
  }

  async function newThread() {
    const outcome = await submitClientIntent('new_thread')
    setSnapshot(outcome.snapshot)
    setNotice(outcome.notice ?? 'New thread')
  }

  async function cancelCurrentTask() {
    const outcome = await cancelTask(null)
    setSnapshot(outcome.snapshot)
    setNotice(outcome.notice ?? 'Cancel recorded')
  }

  async function copyExport() {
    const markdown = await exportThread()
    await navigator.clipboard?.writeText(markdown)
    setNotice('Export copied')
  }

  return (
    <main className="app-shell">
      <aside className="side-panel" aria-label="Tessera shell controls">
        <div className="brand-lockup">
          <TerminalSquare aria-hidden="true" size={22} />
          <div>
            <strong>Tessera</strong>
            <span>GUI Spike</span>
          </div>
        </div>

        <label className="field-label" htmlFor="profile-select">
          Profile
        </label>
        <select
          id="profile-select"
          value={snapshot?.status.active_profile ?? 'mock-replay'}
          onChange={(event) => void switchProfile(event.target.value)}
        >
          {profiles.map((profile) => (
            <option key={profile.id} value={profile.id}>
              {profile.label}
            </option>
          ))}
        </select>

        <div className="metric-list" aria-label="Status metrics">
          {metrics.map((metric) => (
            <div className="metric-row" key={metric.label}>
              <span>{metric.label}</span>
              <strong>{metric.value}</strong>
            </div>
          ))}
        </div>

        <div className="bridge-footprint">
          <span>IPC v{shellState?.ipc_version ?? 1}</span>
          <span>Buffer {shellState?.event_buffer_capacity ?? 0}</span>
        </div>
      </aside>

      <section className="workspace" aria-label="Projected conversation">
        <header className="workspace-header">
          <div>
            <h1>Projected Thread</h1>
            <p>{notice}</p>
          </div>
          <div className="toolbar" aria-label="Thread actions">
            <button
              type="button"
              onClick={() => void newThread()}
              title="New thread"
              aria-label="New thread"
            >
              <Plus aria-hidden="true" size={17} />
            </button>
            <button
              type="button"
              onClick={() => void copyExport()}
              title="Export thread"
              aria-label="Export thread"
            >
              <Download aria-hidden="true" size={17} />
            </button>
            <button
              type="button"
              onClick={() => void cancelCurrentTask()}
              title="Cancel task"
              aria-label="Cancel task"
            >
              <Square aria-hidden="true" size={15} />
            </button>
          </div>
        </header>

        <div className="transcript" data-testid="transcript">
          {messages.map((message, index) => (
            <MessageRow key={message.item_id ?? `${message.role}-${index}`} message={message} />
          ))}
        </div>

        <section className="metadata-panels" aria-label="Read-only workflow metadata">
          <div className="metadata-panel" aria-label="Coding workflows">
            <div className="metadata-heading">
              <h2>Coding Workflows</h2>
              <span>{snapshot?.status.coding_workflow_summary ?? 'workflows 0'}</span>
            </div>
            {workflowRows.length === 0 ? (
              <p className="empty-metadata">No workflow metadata projected.</p>
            ) : (
              workflowRows.map((row) => (
                <article className="metadata-row" key={row.id}>
                  <div className="metadata-title">
                    <strong>{row.id}</strong>
                    <span>{row.active ? 'active' : 'inactive'}</span>
                  </div>
                  <dl className="metadata-grid">
                    <div>
                      <dt>Scope</dt>
                      <dd>{row.scopeRootLabel}</dd>
                    </div>
                    <div>
                      <dt>Allowed</dt>
                      <dd>{row.allowedPathCount}</dd>
                    </div>
                    <div>
                      <dt>Denied</dt>
                      <dd>{row.deniedPathCount}</dd>
                    </div>
                    <div>
                      <dt>Mode</dt>
                      <dd>{row.mutationMode}</dd>
                    </div>
                    <div>
                      <dt>Required</dt>
                      <dd>{row.worktreeRequired ? 'yes' : 'no'}</dd>
                    </div>
                    <div>
                      <dt>Patches</dt>
                      <dd>{row.patchCount}</dd>
                    </div>
                    <div>
                      <dt>Reviews</dt>
                      <dd>{row.reviewCount}</dd>
                    </div>
                    <div>
                      <dt>Plans</dt>
                      <dd>{row.testPlanCount}</dd>
                    </div>
                    <div>
                      <dt>Runs</dt>
                      <dd>{row.testRunCount}</dd>
                    </div>
                    <div>
                      <dt>Evidence</dt>
                      <dd>{row.testEvidenceCount}</dd>
                    </div>
                    <div>
                      <dt>Failures</dt>
                      <dd>{row.failedTestEvidenceCount}</dd>
                    </div>
                    <div>
                      <dt>Blocked Tests</dt>
                      <dd>{row.blockedTestEvidenceCount}</dd>
                    </div>
                    <div>
                      <dt>Restores</dt>
                      <dd>{row.restoreCount}</dd>
                    </div>
                    <div>
                      <dt>Blocked Restores</dt>
                      <dd>{row.blockedRestoreCount}</dd>
                    </div>
                  </dl>
                </article>
              ))
            )}
          </div>

          <div className="metadata-panel" aria-label="Review Inspection">
            <div className="metadata-heading">
              <h2>Review Inspection</h2>
              <span>
                {snapshot?.status.workflow_inspection_summary ??
                  'inspection workflows 0 / review gates 0 accepted 0 rejected 0 revision_requested 0 pending 0 / approvals 0 pending / diff refs 0'}
              </span>
            </div>
            {workflowInspectionRows.length === 0 ? (
              <p className="empty-metadata">No review inspection metadata projected.</p>
            ) : (
              workflowInspectionRows.map((row) => (
                <article className="metadata-row" key={`${row.workflowRef}-${row.taskRef}`}>
                  <div className="metadata-title">
                    <strong>{row.workflowRef}</strong>
                    <span>{row.active ? 'active' : 'inactive'}</span>
                  </div>
                  <dl className="metadata-grid">
                    <div>
                      <dt>Task</dt>
                      <dd>{row.taskRef}</dd>
                    </div>
                    <div>
                      <dt>Mode</dt>
                      <dd>{row.mutationMode}</dd>
                    </div>
                    <div>
                      <dt>Required</dt>
                      <dd>{row.worktreeRequired ? 'yes' : 'no'}</dd>
                    </div>
                    <div>
                      <dt>Patches</dt>
                      <dd>{row.patchCount}</dd>
                    </div>
                    <div>
                      <dt>Diff Refs</dt>
                      <dd>{row.diffArtifactRefCount}</dd>
                    </div>
                    <div>
                      <dt>Needs Review</dt>
                      <dd>{row.patchesRequiringReviewCount}</dd>
                    </div>
                    <div>
                      <dt>Review Bundles</dt>
                      <dd>{row.reviewBundleCount}</dd>
                    </div>
                    <div>
                      <dt>Review Evidence</dt>
                      <dd>{row.reviewEvidenceRefCount}</dd>
                    </div>
                    <div>
                      <dt>Reviewer Gates</dt>
                      <dd>{row.reviewerGateCount}</dd>
                    </div>
                    <div>
                      <dt>Accepted</dt>
                      <dd>{row.acceptedReviewerGateCount}</dd>
                    </div>
                    <div>
                      <dt>Rejected</dt>
                      <dd>{row.rejectedReviewerGateCount}</dd>
                    </div>
                    <div>
                      <dt>Revisions</dt>
                      <dd>{row.revisionRequestedReviewerGateCount}</dd>
                    </div>
                    <div>
                      <dt>Pending</dt>
                      <dd>{row.pendingReviewerGateCount}</dd>
                    </div>
                    <div>
                      <dt>Approvals</dt>
                      <dd>{row.approvalCount}</dd>
                    </div>
                    <div>
                      <dt>Pending Approvals</dt>
                      <dd>{row.pendingApprovalCount}</dd>
                    </div>
                    <div>
                      <dt>Resolved Approvals</dt>
                      <dd>{row.resolvedApprovalCount}</dd>
                    </div>
                    <div>
                      <dt>Preflights</dt>
                      <dd>{row.applyPatchPreflightCount}</dd>
                    </div>
                    <div>
                      <dt>Executor Ready</dt>
                      <dd>{row.executorReadyPreflightCount}</dd>
                    </div>
                    <div>
                      <dt>Executor Blocked</dt>
                      <dd>{row.executorBlockedPreflightCount}</dd>
                    </div>
                    <div>
                      <dt>Executions</dt>
                      <dd>{row.applyPatchExecutionCount}</dd>
                    </div>
                    <div>
                      <dt>Successful Executions</dt>
                      <dd>{row.successfulApplyPatchExecutionCount}</dd>
                    </div>
                    <div>
                      <dt>Failed Executions</dt>
                      <dd>{row.failedApplyPatchExecutionCount}</dd>
                    </div>
                  </dl>
                </article>
              ))
            )}
          </div>

          <div className="metadata-panel" aria-label="Worktree lifecycles">
            <div className="metadata-heading">
              <h2>Worktrees</h2>
              <span>{snapshot?.status.worktree_summary ?? 'worktrees 0'}</span>
            </div>
            {worktreeRows.length === 0 ? (
              <p className="empty-metadata">No worktree lifecycle metadata projected.</p>
            ) : (
              worktreeRows.map((row) => (
                <article className="metadata-row" key={row.id}>
                  <div className="metadata-title">
                    <strong>{row.id}</strong>
                    <span>{row.status}</span>
                  </div>
                  <dl className="metadata-grid">
                    <div>
                      <dt>Root</dt>
                      <dd>{row.rootLabel}</dd>
                    </div>
                    <div>
                      <dt>Base</dt>
                      <dd>{row.baseKey}</dd>
                    </div>
                    <div>
                      <dt>Workflow</dt>
                      <dd>{row.workflowId}</dd>
                    </div>
                  </dl>
                </article>
              ))
            )}
          </div>
        </section>

        <form
          className="composer"
          onSubmit={(event) => {
            event.preventDefault()
            void submitPrompt()
          }}
        >
          <input
            aria-label="Prompt"
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            placeholder="Send a mock/replay prompt"
          />
          <button type="submit" disabled={busy || draft.trim().length === 0}>
            {busy ? <LoaderCircle aria-hidden="true" size={17} /> : <Send aria-hidden="true" size={17} />}
            <span>Send</span>
          </button>
        </form>
      </section>
    </main>
  )
}

function MessageRow({ message }: { message: ClientMessage }) {
  return (
    <article className={`message-row message-${message.role}`}>
      <span className="message-role">{message.role}</span>
      <p>{message.content}</p>
    </article>
  )
}

export default App
