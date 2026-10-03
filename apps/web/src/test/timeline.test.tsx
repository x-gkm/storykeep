import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it } from 'vitest'
import { tokenStore } from '../api/client'
import type { Memory } from '../api/types'
import { timelineQueryFromParams } from '../pages/relationship/timelineParams'
import { makeRelationship, mockApi, renderApp, testUser, type RecordedCall } from './utils'

const memory: Memory = {
  id: 12,
  relationship_id: 3,
  category: 'MILESTONE',
  title: 'First steps',
  description: 'Across the living room',
  memory_date: '2025-03-14',
  created_by: { id: 1, first_name: 'Ada', last_name: 'Lovelace' },
  created_at: '2026-10-03T17:24:13Z',
  updated_at: '2026-10-03T17:24:13Z',
  tags: ['Home', 'walking'],
  media: [],
}

function timelineCalls(calls: RecordedCall[]) {
  return calls.filter((c) => c.url.pathname === '/api/relationships/3/memories')
}

describe('timeline', () => {
  beforeEach(() => tokenStore.set('tok'))

  it('maps URL params to the API query', () => {
    expect(timelineQueryFromParams(new URLSearchParams('from=2025-01-01&category=BIRTHDAY&page=3&order=asc&q=%20cake%20'))).toEqual({
      from: '2025-01-01',
      to: undefined,
      category: 'BIRTHDAY',
      tag: undefined,
      q: 'cake',
      order: 'asc',
      limit: 20,
      offset: 40,
    })
    // Unknown categories are ignored rather than sent.
    expect(timelineQueryFromParams(new URLSearchParams('category=NOPE')).category).toBeUndefined()
  })

  it('sends the chosen filters as query parameters and shows event vs recorded dates', async () => {
    const calls = mockApi({
      'GET /api/users/me': { body: testUser },
      'GET /api/relationships/3': { body: makeRelationship() },
      'GET /api/relationships/3/memories': { body: { memories: [memory], total: 45, limit: 20, offset: 0 } },
      'GET /api/relationships/3/tags': { body: [{ name: 'Home', memory_count: 1 }, { name: 'walking', memory_count: 1 }] },
    })
    renderApp('/relationships/3')

    expect(await screen.findByRole('link', { name: 'First steps' })).toBeInTheDocument()
    expect(screen.getByText('Happened')).toBeInTheDocument()
    expect(screen.getByText('Recorded')).toBeInTheDocument()
    const first = timelineCalls(calls)[0]
    expect(first?.url.searchParams.get('limit')).toBe('20')
    expect(first?.url.searchParams.get('offset')).toBe('0')
    expect(first?.url.searchParams.get('order')).toBe('desc')

    const user = userEvent.setup()
    await user.type(screen.getByLabelText('Search'), 'steps')
    await user.type(screen.getByLabelText('From'), '2025-01-01')
    await user.type(screen.getByLabelText('To'), '2025-12-31')
    await user.selectOptions(screen.getByLabelText('Category'), 'MILESTONE')
    await user.selectOptions(screen.getByLabelText('Tag'), 'walking')
    await user.click(screen.getByRole('button', { name: 'Apply' }))

    await waitFor(() => expect(timelineCalls(calls).length).toBeGreaterThan(1))
    const filtered = timelineCalls(calls).at(-1)!.url.searchParams
    expect(Object.fromEntries(filtered)).toEqual({
      from: '2025-01-01',
      to: '2025-12-31',
      category: 'MILESTONE',
      tag: 'walking',
      q: 'steps',
      order: 'desc',
      limit: '20',
      offset: '0',
    })
    expect(screen.getByTestId('location')).toHaveTextContent('category=MILESTONE')

    // Pagination: 45 results → 3 pages; "Next" requests offset 20.
    await user.click(await screen.findByRole('button', { name: 'Next →' }))
    await waitFor(() => expect(timelineCalls(calls).at(-1)!.url.searchParams.get('offset')).toBe('20'))
    expect(timelineCalls(calls).at(-1)!.url.searchParams.get('category')).toBe('MILESTONE')
  })

  it('rejects an end date before the start date without calling the API', async () => {
    const calls = mockApi({
      'GET /api/users/me': { body: testUser },
      'GET /api/relationships/3': { body: makeRelationship() },
      'GET /api/relationships/3/memories': { body: { memories: [], total: 0, limit: 20, offset: 0 } },
      'GET /api/relationships/3/tags': { body: [] },
    })
    renderApp('/relationships/3')
    await screen.findByText('No memories yet')
    const before = timelineCalls(calls).length
    const user = userEvent.setup()
    await user.type(screen.getByLabelText('From'), '2025-05-01')
    await user.type(screen.getByLabelText('To'), '2025-01-01')
    await user.click(screen.getByRole('button', { name: 'Apply' }))
    expect(screen.getByText('The end date must not be before the start date.')).toBeInTheDocument()
    expect(timelineCalls(calls).length).toBe(before)
  })
})
