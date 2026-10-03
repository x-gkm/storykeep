import { useQueryClient } from '@tanstack/react-query'
import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react'
import { onUnauthorized, tokenStore } from '../api/client'
import { api } from '../api/endpoints'
import type { LoginInput, RegisterInput, Session, User } from '../api/types'
import { AuthContext, type AuthState, type AuthStatus } from './context'

export function AuthProvider({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient()
  const [status, setStatus] = useState<AuthStatus>(() => (tokenStore.get() ? 'loading' : 'anonymous'))
  const [user, setUserState] = useState<User | null>(null)

  const signOutLocally = useCallback(() => {
    tokenStore.clear()
    setUserState(null)
    setStatus('anonymous')
    queryClient.clear()
  }, [queryClient])

  // Restore the session from a stored token.
  useEffect(() => {
    if (!tokenStore.get()) return
    let cancelled = false
    api
      .getMe()
      .then((me) => {
        if (cancelled) return
        setUserState(me)
        setStatus('authenticated')
      })
      .catch(() => {
        // A 401 has already cleared the token; for other failures we still can't
        // show protected pages without a user, so fall back to the login screen.
        if (!cancelled) {
          setUserState(null)
          setStatus('anonymous')
        }
      })
    return () => {
      cancelled = true
    }
  }, [])

  // Any 401 `unauthorized` anywhere ends the session; <RequireAuth> then redirects to
  // the login page and remembers where the user was.
  useEffect(() => onUnauthorized(signOutLocally), [signOutLocally])

  const startSession = useCallback(
    (session: Session) => {
      queryClient.clear()
      tokenStore.set(session.token)
      setUserState(session.user)
      setStatus('authenticated')
      return session.user
    },
    [queryClient],
  )

  const login = useCallback(async (input: LoginInput) => startSession(await api.login(input)), [startSession])
  const register = useCallback(
    async (input: RegisterInput) => startSession(await api.register(input)),
    [startSession],
  )

  const logout = useCallback(async () => {
    try {
      await api.logout()
    } catch {
      // Even if the server call fails, forget the token locally.
    }
    signOutLocally()
  }, [signOutLocally])

  const value = useMemo<AuthState>(
    () => ({ status, user, login, register, logout, setUser: setUserState }),
    [status, user, login, register, logout],
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}
