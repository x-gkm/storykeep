import type { Location } from 'react-router'

export interface LoginRedirectState {
  from?: Pick<Location, 'pathname' | 'search' | 'hash'>
}

/** Where to go after signing in: the remembered route, or the dashboard. */
export function redirectTarget(state: unknown): string {
  const from = (state as LoginRedirectState | null)?.from
  if (from && typeof from.pathname === 'string' && from.pathname.startsWith('/') && from.pathname !== '/login') {
    return `${from.pathname}${from.search ?? ''}${from.hash ?? ''}`
  }
  return '/'
}
