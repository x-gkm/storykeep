import { render } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
import { vi } from 'vitest'
import { AppProviders, AppRoutes } from '../App'
import { createQueryClient } from '../api/queries'
import type { Relationship, User } from '../api/types'
import { LocationProbe } from './LocationProbe'

export interface RecordedCall {
  method: string
  url: URL
  headers: Headers
  body: unknown
}

type Handler = (call: RecordedCall) => { status?: number; body?: unknown } | undefined

/**
 * Replaces global fetch with a router over `METHOD /path` keys. Unmatched requests
 * return 404 in the API's error shape. Returns the list of recorded calls.
 */
export function mockApi(routes: Record<string, Handler | { status?: number; body?: unknown }>): RecordedCall[] {
  const calls: RecordedCall[] = []
  vi.stubGlobal(
    'fetch',
    vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = new URL(String(input), 'http://localhost')
      const method = init?.method ?? 'GET'
      const call: RecordedCall = {
        method,
        url,
        headers: new Headers(init?.headers),
        body: typeof init?.body === 'string' ? JSON.parse(init.body) : undefined,
      }
      calls.push(call)
      const route = routes[`${method} ${url.pathname}`]
      const result = typeof route === 'function' ? route(call) : route
      if (!result) {
        return new Response(JSON.stringify({ error: { code: 'not_found', message: `no mock for ${method} ${url.pathname}` } }), {
          status: 404,
        })
      }
      const status = result.status ?? 200
      return new Response(status === 204 ? null : JSON.stringify(result.body ?? null), {
        status,
        headers: { 'Content-Type': 'application/json' },
      })
    }),
  )
  return calls
}

export const testUser: User = {
  id: 1,
  email: 'ada@example.com',
  first_name: 'Ada',
  last_name: 'Lovelace',
  date_of_birth: null,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
}

export function makeRelationship(overrides: Partial<Relationship> = {}): Relationship {
  return {
    id: 3,
    profile_id: 7,
    profile_name: 'Mira',
    profile_type: 'CHILD',
    relationship_type: 'PARENT_CHILD',
    started_at: '2024-03-01',
    ended_at: null,
    created_at: '2026-01-01T00:00:00Z',
    role: 'OWNER',
    ...overrides,
  }
}

/** Renders the whole app at `route` with a fresh query client. */
export function renderApp(route: string) {
  const client = createQueryClient()
  client.setDefaultOptions({ queries: { ...client.getDefaultOptions().queries, retry: false } })
  return render(
    <MemoryRouter initialEntries={[route]}>
      <AppProviders client={client}>
        <AppRoutes />
        <LocationProbe />
      </AppProviders>
    </MemoryRouter>,
  )
}
