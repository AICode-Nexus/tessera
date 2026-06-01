import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import App from './App'

describe('GUI shell smoke path', () => {
  it('exposes stable accessible names for toolbar icon actions', async () => {
    render(<App />)

    expect(await screen.findByText('Mock/replay projection loaded')).toBeInTheDocument()
    expect(screen.getByTitle('New thread')).toHaveAttribute('aria-label', 'New thread')
    expect(screen.getByTitle('Export thread')).toHaveAttribute('aria-label', 'Export thread')
    expect(screen.getByTitle('Cancel task')).toHaveAttribute('aria-label', 'Cancel task')
  })

  it('renders read-only workflow and worktree metadata from the mock snapshot', async () => {
    render(<App />)

    expect(await screen.findByText('Mock/replay projection loaded')).toBeInTheDocument()
    expect(screen.getByLabelText('Coding workflows')).toHaveTextContent('workflow_web_read_only')
    expect(screen.getByLabelText('Coding workflows')).not.toHaveTextContent(
      'Project read-only workflow metadata',
    )
    expect(screen.getByLabelText('Coding workflows')).toHaveTextContent('Allowed2')
    expect(screen.getByLabelText('Coding workflows')).toHaveTextContent('Requiredyes')
    expect(screen.getByLabelText('Coding workflows')).not.toHaveTextContent('apps/gui-tauri/src/App.tsx')
    expect(screen.getByLabelText('Coding workflows')).not.toHaveTextContent('Render compact metadata rows')
    expect(screen.getByLabelText('Coding workflows')).not.toHaveTextContent('No browser fallback tests run yet')
    expect(screen.getByLabelText('Coding workflows')).not.toHaveTextContent(
      'Restore execution is outside GUI surface',
    )
    expect(screen.getByLabelText('Worktree lifecycles')).toHaveTextContent('worktree_web_read_only')
    expect(screen.getByLabelText('Worktree lifecycles')).toHaveTextContent('mock-base')
    expect(screen.getByLabelText('Worktree lifecycles')).not.toHaveTextContent(
      'available for read-only inspection',
    )
  })

  it('renders safe review inspection metadata from the mock snapshot', async () => {
    render(<App />)

    expect(await screen.findByText('Mock/replay projection loaded')).toBeInTheDocument()
    const panel = screen.getByLabelText('Review Inspection')
    expect(within(panel).getByRole('heading', { name: 'Review Inspection' })).toBeInTheDocument()
    expect(panel).toHaveTextContent(
      'inspection workflows 1 / review gates 4 accepted / approvals 1 pending / diff refs 3',
    )
    expect(panel).toHaveTextContent('workflow:1')
    expect(panel).toHaveTextContent('task:1')
    expect(panel).toHaveTextContent('Patches2')
    expect(panel).toHaveTextContent('Diff Refs3')
    expect(panel).toHaveTextContent('Needs Review1')
    expect(panel).toHaveTextContent('Review Bundles4')
    expect(panel).toHaveTextContent('Reviewer Gates6')
    expect(panel).toHaveTextContent('Accepted4')
    expect(panel).toHaveTextContent('Pending1')
    expect(panel).toHaveTextContent('Approvals3')
    expect(panel).toHaveTextContent('Pending Approvals1')
    expect(panel).toHaveTextContent('Preflights4')
    expect(panel).toHaveTextContent('Executor Ready1')
    expect(panel).toHaveTextContent('Executions2')
    expect(panel).toHaveTextContent('Failed Executions1')
    expect(panel).not.toHaveTextContent('workflow_web_read_only')
    expect(panel).not.toHaveTextContent('task_web_workflow')
    expect(panel).not.toHaveTextContent('gate_web_review')
    expect(panel).not.toHaveTextContent('approval_web_sensitive')
    expect(panel).not.toHaveTextContent('Project read-only workflow metadata')
    expect(panel).not.toHaveTextContent('Render compact metadata rows')
    expect(panel).not.toHaveTextContent('Read-only metadata ready for review')
    expect(panel).not.toHaveTextContent('No browser fallback tests run yet')
    expect(panel).not.toHaveTextContent('Restore execution is outside GUI surface')
    expect(panel).not.toHaveTextContent('apps/gui-tauri/src/App.tsx')
    expect(panel).not.toHaveTextContent('/Users/admin/work/tessera/.env')
    expect(panel).not.toHaveTextContent('reviewer private comment')
    expect(panel).not.toHaveTextContent('approval reason')
    expect(panel).not.toHaveTextContent('diagnostic summary')
  })

  it('projects submit, cancel, and new-thread actions through the mock shell', async () => {
    render(<App />)

    expect(await screen.findByText('Mock/replay projection loaded')).toBeInTheDocument()
    const transcript = screen.getByTestId('transcript')
    expect(within(transcript).getByText(/This mock\/replay snapshot/)).toBeInTheDocument()

    await act(async () => {
      fireEvent.change(screen.getByLabelText('Prompt'), {
        target: { value: 'smoke prompt' },
      })
    })
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Send' }))
    })

    expect(await within(transcript).findByText('smoke prompt')).toBeInTheDocument()
    expect(await within(transcript).findByText(/mock\/replay response accepted/)).toBeInTheDocument()

    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Cancel task' }))
    })
    expect(
      await within(transcript).findByText(/Cancel requested for mock\/replay projection only/),
    ).toBeInTheDocument()

    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'New thread' }))
    })
    await waitFor(() => {
      expect(within(transcript).queryByText('smoke prompt')).not.toBeInTheDocument()
    })
  })
})
