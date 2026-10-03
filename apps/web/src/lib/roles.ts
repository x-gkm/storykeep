// Role rules from docs/api/README.md. These only decide what the UI offers;
// the server checks every request and has the final say.
import type { Role } from '../api/types'

/** Create memories, media, tags, development data, capsules. */
export function canWrite(role: Role | undefined): boolean {
  return role === 'OWNER' || role === 'PARENT' || role === 'MEMBER'
}

/** Manage the relationship, its members, and anyone's content. */
export function canManage(role: Role | undefined): boolean {
  return role === 'OWNER' || role === 'PARENT'
}

export function isOwner(role: Role | undefined): boolean {
  return role === 'OWNER'
}

/** Edit/delete an item: its creator while they can still write, or anyone who can manage. */
export function canEditItem(role: Role | undefined, creatorId: number, userId: number | undefined): boolean {
  return canManage(role) || (canWrite(role) && creatorId === userId)
}

/** Roles the current user may grant to others. Only an OWNER may grant OWNER. */
export function grantableRoles(role: Role | undefined): Role[] {
  if (role === 'OWNER') return ['OWNER', 'PARENT', 'MEMBER', 'VIEWER']
  if (role === 'PARENT') return ['PARENT', 'MEMBER', 'VIEWER']
  return []
}
