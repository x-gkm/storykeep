import type { ReactNode } from 'react'
import { Navigate, useLocation } from 'react-router'
import { Loading } from '../components/ui'
import { useAuth } from './context'
import { redirectTarget, type LoginRedirectState } from './redirect'

/** Renders children only for a signed-in user; otherwise sends them to /login, remembering the route. */
export function RequireAuth({ children }: { children: ReactNode }) {
  const { status } = useAuth()
  const location = useLocation()
  if (status === 'loading') return <Loading label="Restoring your session…" />
  if (status === 'anonymous') {
    const state: LoginRedirectState = {
      from: { pathname: location.pathname, search: location.search, hash: location.hash },
    }
    return <Navigate to="/login" replace state={state} />
  }
  return children
}

/** For login/register: signed-in users go straight to the app (or the route they were sent away from). */
export function RedirectIfAuthenticated({ children }: { children: ReactNode }) {
  const { status } = useAuth()
  const location = useLocation()
  if (status === 'loading') return <Loading label="Restoring your session…" />
  if (status === 'authenticated') return <Navigate to={redirectTarget(location.state)} replace />
  return children
}
