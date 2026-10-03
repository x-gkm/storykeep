import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import { tokenStore } from '../api/client'
import { makeRelationship, mockApi, renderApp, testUser } from './utils'

const emptyTimeline = { memories: [], total: 0, limit: 20, offset: 0 }

describe('auth flow', () => {
  it('redirects to login, stores the token on success and returns to the intended route', async () => {
    const calls = mockApi({
      'POST /api/auth/login': ({ body }) =>
        (body as { password: string }).password === 'correct horse'
          ? { body: { token: 'tok-1', expires_at: '2026-12-01T00:00:00Z', user: testUser } }
          : { status: 401, body: { error: { code: 'invalid_credentials', message: 'invalid email or password' } } },
      'GET /api/relationships/3': { body: makeRelationship() },
      'GET /api/relationships/3/memories': { body: emptyTimeline },
      'GET /api/relationships/3/tags': { body: [] },
    })
    renderApp('/relationships/3?category=BIRTHDAY')

    expect(await screen.findByRole('heading', { name: 'Log in' })).toBeInTheDocument()
    expect(screen.getByTestId('location')).toHaveTextContent('/login')

    const user = userEvent.setup()
    // Client-side validation first: nothing is sent.
    await user.click(screen.getByRole('button', { name: 'Log in' }))
    expect(screen.getByText('Email is required.')).toBeInTheDocument()
    expect(calls).toHaveLength(0)

    await user.type(screen.getByLabelText('Email'), 'ada@example.com')
    await user.type(screen.getByLabelText('Password'), 'wrong')
    await user.click(screen.getByRole('button', { name: 'Log in' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Invalid email or password')
    expect(tokenStore.get()).toBeNull()

    await user.clear(screen.getByLabelText('Password'))
    await user.type(screen.getByLabelText('Password'), 'correct horse')
    await user.click(screen.getByRole('button', { name: 'Log in' }))

    await waitFor(() => expect(screen.getByTestId('location')).toHaveTextContent('/relationships/3?category=BIRTHDAY'))
    expect(tokenStore.get()).toBe('tok-1')
    expect(await screen.findByRole('heading', { name: 'Mira' })).toBeInTheDocument()
    const timelineCall = calls.find((c) => c.url.pathname === '/api/relationships/3/memories')
    expect(timelineCall?.headers.get('Authorization')).toBe('Bearer tok-1')
  })

  it('restores a session from a stored token', async () => {
    tokenStore.set('stored')
    mockApi({ 'GET /api/users/me': { body: testUser }, 'GET /api/relationships': { body: [] } })
    renderApp('/')
    expect(await screen.findByText('Welcome back, Ada.')).toBeInTheDocument()
  })

  it('drops an expired token and sends the user to login, remembering the route', async () => {
    tokenStore.set('expired')
    mockApi({ 'GET /api/users/me': { status: 401, body: { error: { code: 'unauthorized', message: 'invalid or expired token' } } } })
    renderApp('/account')
    expect(await screen.findByRole('heading', { name: 'Log in' })).toBeInTheDocument()
    expect(tokenStore.get()).toBeNull()
  })

  it('logs out when an API call returns 401 mid-session', async () => {
    tokenStore.set('soon-expired')
    mockApi({
      'GET /api/users/me': { body: testUser },
      'GET /api/relationships': { status: 401, body: { error: { code: 'unauthorized', message: 'expired' } } },
    })
    renderApp('/')
    expect(await screen.findByRole('heading', { name: 'Log in' })).toBeInTheDocument()
    expect(tokenStore.get()).toBeNull()
  })
})
