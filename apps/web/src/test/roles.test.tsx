import { screen, within } from '@testing-library/react'
import { beforeEach, describe, expect, it } from 'vitest'
import { tokenStore } from '../api/client'
import type { Member, Role } from '../api/types'
import { canEditItem, canManage, canWrite, grantableRoles } from '../lib/roles'
import { makeRelationship, mockApi, renderApp, testUser } from './utils'

const members: Member[] = [
  { user_id: 2, email: 'grace@example.com', first_name: 'Grace', last_name: 'Hopper', role: 'OWNER', joined_at: '2026-01-01T00:00:00Z' },
  { user_id: 1, email: 'ada@example.com', first_name: 'Ada', last_name: 'Lovelace', role: 'VIEWER', joined_at: '2026-01-02T00:00:00Z' },
  { user_id: 4, email: 'alan@example.com', first_name: 'Alan', last_name: 'Turing', role: 'MEMBER', joined_at: '2026-01-03T00:00:00Z' },
]

function mockRelationship(role: Role) {
  const withMe = members.map((m) => (m.user_id === 1 ? { ...m, role } : m))
  return mockApi({
    'GET /api/users/me': { body: testUser },
    'GET /api/relationships/3': { body: makeRelationship({ role }) },
    'GET /api/relationships/3/memories': { body: { memories: [], total: 0, limit: 20, offset: 0 } },
    'GET /api/relationships/3/tags': { body: [] },
    'GET /api/relationships/3/members': { body: withMe },
    'GET /api/relationships/3/capsules': { body: [] },
  })
}

describe('role helpers', () => {
  it('match the documented permission table', () => {
    expect((['OWNER', 'PARENT', 'MEMBER', 'VIEWER'] as const).map(canWrite)).toEqual([true, true, true, false])
    expect((['OWNER', 'PARENT', 'MEMBER', 'VIEWER'] as const).map(canManage)).toEqual([true, true, false, false])
    expect(canEditItem('MEMBER', 1, 1)).toBe(true)
    expect(canEditItem('MEMBER', 2, 1)).toBe(false)
    expect(canEditItem('VIEWER', 1, 1)).toBe(false)
    expect(canEditItem('PARENT', 2, 1)).toBe(true)
    expect(grantableRoles('PARENT')).not.toContain('OWNER')
    expect(grantableRoles('MEMBER')).toEqual([])
  })
})

describe('role-based UI', () => {
  beforeEach(() => tokenStore.set('tok'))

  it('hides write and manage actions from a VIEWER', async () => {
    mockRelationship('VIEWER')
    renderApp('/relationships/3')
    expect(await screen.findByRole('heading', { name: 'Timeline' })).toBeInTheDocument()
    expect(screen.queryByRole('link', { name: /add memory/i })).not.toBeInTheDocument()
    expect(screen.queryByRole('link', { name: 'Settings' })).not.toBeInTheDocument()
    expect(await screen.findByText('Memories added by other members will appear here.')).toBeInTheDocument()
  })

  it('shows write and manage actions to an OWNER', async () => {
    mockRelationship('OWNER')
    renderApp('/relationships/3')
    expect(await screen.findByRole('link', { name: 'Add memory' })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Settings' })).toBeInTheDocument()
  })

  it('lets a VIEWER only leave on the members page', async () => {
    mockRelationship('VIEWER')
    renderApp('/relationships/3/members')
    const me = (await screen.findByText('(you)')).closest('li')!
    expect(within(me).getByRole('button', { name: 'Leave' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Remove' })).not.toBeInTheDocument()
    expect(screen.queryByRole('combobox')).not.toBeInTheDocument()
    expect(screen.queryByRole('heading', { name: 'Add a member' })).not.toBeInTheDocument()
  })

  it('lets a PARENT manage members but not owners', async () => {
    mockRelationship('PARENT')
    renderApp('/relationships/3/members')
    expect(await screen.findByRole('heading', { name: 'Add a member' })).toBeInTheDocument()
    const owner = (await screen.findByText('Grace Hopper')).closest('li')!
    expect(within(owner).queryByRole('button', { name: 'Remove' })).not.toBeInTheDocument()
    expect(within(owner).queryByRole('combobox')).not.toBeInTheDocument()
    const member = screen.getByText('Alan Turing').closest('li')!
    expect(within(member).getByRole('button', { name: 'Remove' })).toBeInTheDocument()
    const roleSelect = within(member).getByRole('combobox')
    expect(within(roleSelect).queryByRole('option', { name: 'Owner' })).not.toBeInTheDocument()
  })

  it('hides "New capsule" from a VIEWER', async () => {
    mockRelationship('VIEWER')
    renderApp('/relationships/3/capsules')
    expect(await screen.findByText('No time capsules yet')).toBeInTheDocument()
    expect(screen.queryByRole('link', { name: /new capsule|seal a message/i })).not.toBeInTheDocument()
  })
})
