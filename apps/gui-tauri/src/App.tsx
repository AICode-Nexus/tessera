import { useEffect, useMemo, useState } from 'react'
import { Download, LoaderCircle, Plus, Send, Square, TerminalSquare } from 'lucide-react'

import {
  cancelTask,
  exportThread,
  loadShellState,
  submitClientIntent,
} from './ipc'
import type { ClientMessage, ClientSnapshot, GuiProfile, GuiShellState } from './types'
import { buildShellMetrics, visibleMessages } from './view-model'

function App() {
  const [shellState, setShellState] = useState<GuiShellState | null>(null)
  const [snapshot, setSnapshot] = useState<ClientSnapshot | null>(null)
  const [profiles, setProfiles] = useState<GuiProfile[]>([])
  const [draft, setDraft] = useState('')
  const [notice, setNotice] = useState('Loading GUI shell...')
  const [busy, setBusy] = useState(false)
  const metrics = useMemo(() => (snapshot ? buildShellMetrics(snapshot) : []), [snapshot])
  const messages = useMemo(() => (snapshot ? visibleMessages(snapshot) : []), [snapshot])

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
