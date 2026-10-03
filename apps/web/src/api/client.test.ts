import { describe, expect, it, vi } from 'vitest'
import { mockApi } from '../test/utils'
import { ApiError, buildQuery, onUnauthorized, parseError, request, tokenStore } from './client'
import { api } from './endpoints'

describe('api client', () => {
  it('attaches the bearer token when one is stored', async () => {
    tokenStore.set('abc123')
    const calls = mockApi({ 'GET /api/users/me': { body: { id: 1 } } })
    await api.getMe()
    expect(calls[0]?.headers.get('Authorization')).toBe('Bearer abc123')
  })

  it('sends no Authorization header without a token', async () => {
    const calls = mockApi({ 'GET /api/reference': { body: {} } })
    await api.getReference()
    expect(calls[0]?.headers.has('Authorization')).toBe(false)
  })

  it('sends JSON bodies and parses JSON responses', async () => {
    const calls = mockApi({ 'POST /api/tags': { body: { name: 'Beach', memory_count: 0 } } })
    const tag = await api.createTag('Beach')
    expect(tag).toEqual({ name: 'Beach', memory_count: 0 })
    expect(calls[0]?.headers.get('Content-Type')).toBe('application/json')
    expect(calls[0]?.body).toEqual({ name: 'Beach' })
  })

  it('resolves 204 responses to undefined', async () => {
    mockApi({ 'DELETE /api/memories/5': { status: 204 } })
    await expect(api.deleteMemory(5)).resolves.toBeUndefined()
  })

  it('turns the error body into an ApiError with status, code and message', async () => {
    mockApi({ 'GET /api/relationships/9': { status: 403, body: { error: { code: 'forbidden', message: 'viewers cannot do this' } } } })
    const error = await api.getRelationship(9).catch((e: unknown) => e)
    expect(error).toBeInstanceOf(ApiError)
    expect(error).toMatchObject({ status: 403, code: 'forbidden', message: 'viewers cannot do this' })
  })

  it('handles non-JSON error bodies', () => {
    const error = parseError(502, 'Bad Gateway')
    expect(error).toMatchObject({ status: 502, code: 'unknown', message: 'Bad Gateway' })
  })

  it('reports network failures as status 0', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new TypeError('Failed to fetch')))
    await expect(request('/api/profiles')).rejects.toMatchObject({ status: 0, code: 'network_error' })
  })

  it('on 401 unauthorized clears the token and notifies listeners', async () => {
    tokenStore.set('expired')
    const listener = vi.fn()
    const unsubscribe = onUnauthorized(listener)
    mockApi({ 'GET /api/profiles': { status: 401, body: { error: { code: 'unauthorized', message: 'invalid token' } } } })
    await expect(api.listProfiles()).rejects.toMatchObject({ status: 401 })
    expect(tokenStore.get()).toBeNull()
    expect(listener).toHaveBeenCalledTimes(1)
    unsubscribe()
  })

  it('does not end the session on 401 invalid_credentials (e.g. a wrong current password)', async () => {
    tokenStore.set('valid')
    const listener = vi.fn()
    const unsubscribe = onUnauthorized(listener)
    mockApi({
      'PUT /api/users/me/password': { status: 401, body: { error: { code: 'invalid_credentials', message: 'wrong password' } } },
    })
    await expect(api.changePassword({ current_password: 'x', new_password: 'yyyyyyyy' })).rejects.toMatchObject({
      code: 'invalid_credentials',
    })
    expect(tokenStore.get()).toBe('valid')
    expect(listener).not.toHaveBeenCalled()
    unsubscribe()
  })

  it('builds query strings without empty values', () => {
    expect(buildQuery({ a: 'x', b: undefined, c: '', d: 0, e: null })).toBe('?a=x&d=0')
    expect(buildQuery({})).toBe('')
  })
})
