import { useState } from 'react'
import { Link, NavLink, Outlet, useNavigate } from 'react-router'
import { useAuth } from '../auth/context'

export function AppLayout() {
  const { user, logout } = useAuth()
  const navigate = useNavigate()
  const [busy, setBusy] = useState(false)
  return (
    <div className="app">
      <a className="skip-link" href="#main">
        Skip to content
      </a>
      <header className="topbar">
        <div className="topbar-inner">
          <Link to="/" className="brand">
            <img src="/favicon.svg" alt="" width={24} height={24} />
            Storykeep
          </Link>
          <nav aria-label="Main" className="topnav">
            <NavLink to="/" end>
              Relationships
            </NavLink>
            <NavLink to="/account">{user ? user.first_name : 'Account'}</NavLink>
            <button
              type="button"
              className="btn btn-ghost btn-small"
              disabled={busy}
              onClick={() => {
                setBusy(true)
                void logout().then(() => navigate('/login', { replace: true }))
              }}
            >
              Log out
            </button>
          </nav>
        </div>
      </header>
      <main id="main" className="container">
        <Outlet />
      </main>
    </div>
  )
}

export function AuthLayout() {
  return (
    <div className="auth-shell">
      <main id="main" className="auth-card">
        <p className="brand brand-center">
          <img src="/favicon.svg" alt="" width={28} height={28} />
          Storykeep
        </p>
        <Outlet />
      </main>
    </div>
  )
}
