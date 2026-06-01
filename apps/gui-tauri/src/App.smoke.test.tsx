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
    expect(screen.getByLabelText('Coding workflows')).toHaveTextContent(
      'Project read-only workflow metadata',
    )
    expect(screen.getByLabelText('Coding workflows')).toHaveTextContent('apps/gui-tauri/src/App.tsx')
    expect(screen.getByLabelText('Worktree lifecycles')).toHaveTextContent('worktree_web_read_only')
    expect(screen.getByLabelText('Worktree lifecycles')).toHaveTextContent('mock-base')
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
