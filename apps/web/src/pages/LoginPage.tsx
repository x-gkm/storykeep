import { useState, type FormEvent } from 'react'
import { Link, useLocation } from 'react-router'
import { useAuth } from '../auth/context'
import { Button, ErrorMessage, TextField } from '../components/ui'
import { email as checkEmail, hasErrors, required, type Errors } from '../lib/validation'

export function LoginPage() {
  const { login } = useAuth()
  const location = useLocation()
  const [form, setForm] = useState({ email: '', password: '' })
  const [errors, setErrors] = useState<Errors<'email' | 'password'>>({})
  const [submitError, setSubmitError] = useState<unknown>(null)
  const [busy, setBusy] = useState(false)

  const onSubmit = async (event: FormEvent) => {
    event.preventDefault()
    const next = { email: checkEmail(form.email), password: required(form.password, 'Password') }
    setErrors(next)
    if (hasErrors(next)) return
    setBusy(true)
    setSubmitError(null)
    try {
      // On success the auth state flips and <RedirectIfAuthenticated> sends the user on.
      await login({ email: form.email.trim(), password: form.password })
    } catch (error) {
      setSubmitError(error)
      setBusy(false)
    }
  }

  return (
    <>
      <h1>Log in</h1>
      <form onSubmit={onSubmit} noValidate className="form">
        <ErrorMessage error={submitError} />
        <TextField
          label="Email"
          type="email"
          autoComplete="email"
          value={form.email}
          error={errors.email}
          onChange={(e) => setForm({ ...form, email: e.target.value })}
        />
        <TextField
          label="Password"
          type="password"
          autoComplete="current-password"
          value={form.password}
          error={errors.password}
          onChange={(e) => setForm({ ...form, password: e.target.value })}
        />
        <Button type="submit" variant="primary" busy={busy}>
          Log in
        </Button>
      </form>
      <p className="auth-switch">
        New here?{' '}
        <Link to="/register" state={location.state}>
          Create an account
        </Link>
      </p>
    </>
  )
}
