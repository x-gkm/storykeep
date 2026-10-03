import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { tokenStore } from '../api/client'
import type { Capsule } from '../api/types'
import { CapsuleView } from '../pages/capsule/CapsuleView'
import { makeRelationship, mockApi, renderApp, testUser } from './utils'

const base: Capsule = {
  id: 12,
  relationship_id: 3,
  title: 'For your 18th birthday',
  status: 'LOCKED',
  unlock_at: new Date(Date.now() + 5 * 86_400_000).toISOString(),
  created_by: 1,
  created_at: '2026-10-03T17:24:13Z',
  updated_at: '2026-10-03T17:24:13Z',
  media_count: 2,
}

function renderView(capsule: Capsule) {
  return render(
    <MemoryRouter>
      <CapsuleView capsule={capsule} onOpen={vi.fn()} />
    </MemoryRouter>,
  )
}

describe('capsule view', () => {
  it('never renders content for a locked capsule, even if a message were present', () => {
    renderView({ ...base, message: 'SECRET MESSAGE', media: [] })
    expect(screen.queryByText('SECRET MESSAGE')).not.toBeInTheDocument()
    expect(screen.queryByTestId('capsule-content')).not.toBeInTheDocument()
    expect(screen.getByTestId('capsule-sealed')).toHaveTextContent(/Unlocks in/)
    expect(screen.getByText(/2 attachments/)).toBeInTheDocument()
  })

  it('does not render content for AVAILABLE or CANCELLED capsules', () => {
    const { unmount } = renderView({ ...base, status: 'AVAILABLE', message: 'SECRET MESSAGE' })
    expect(screen.queryByText('SECRET MESSAGE')).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Open capsule' })).toBeInTheDocument()
    unmount()
    renderView({ ...base, status: 'CANCELLED', message: 'SECRET MESSAGE' })
    expect(screen.queryByText('SECRET MESSAGE')).not.toBeInTheDocument()
  })

  it('shows the message once opened', () => {
    renderView({ ...base, status: 'OPENED', message: 'Happy birthday!', media: [] })
    expect(screen.getByTestId('capsule-content')).toHaveTextContent('Happy birthday!')
  })
})

describe('capsule page', () => {
  beforeEach(() => tokenStore.set('tok'))

  it('opens an available capsule and then shows its message', async () => {
    const calls = mockApi({
      'GET /api/users/me': { body: testUser },
      'GET /api/capsules/12': { body: { ...base, status: 'AVAILABLE', unlock_at: '2026-01-01T00:00:00Z' } },
      'GET /api/relationships/3': { body: makeRelationship({ role: 'VIEWER' }) },
      'POST /api/capsules/12/open': {
        body: { ...base, status: 'OPENED', unlock_at: '2026-01-01T00:00:00Z', message: 'Hello from the past', media: [] },
      },
    })
    renderApp('/capsules/12')
    expect(await screen.findByRole('heading', { name: 'For your 18th birthday' })).toBeInTheDocument()
    expect(screen.queryByText('Hello from the past')).not.toBeInTheDocument()
    await userEvent.setup().click(screen.getByRole('button', { name: 'Open capsule' }))
    expect(await screen.findByText('Hello from the past')).toBeInTheDocument()
    expect(calls.some((c) => c.method === 'POST' && c.url.pathname === '/api/capsules/12/open')).toBe(true)
  })

  it('offers no edit/cancel controls to a VIEWER of a locked capsule', async () => {
    mockApi({
      'GET /api/users/me': { body: testUser },
      'GET /api/capsules/12': { body: { ...base, created_by: 99 } },
      'GET /api/relationships/3': { body: makeRelationship({ role: 'VIEWER' }) },
    })
    renderApp('/capsules/12')
    expect(await screen.findByTestId('capsule-sealed')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Cancel capsule' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Delete' })).not.toBeInTheDocument()
    expect(screen.queryByText('Add media to the capsule')).not.toBeInTheDocument()
  })
})
