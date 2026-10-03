import { useMutation } from '@tanstack/react-query'
import { useState, type FormEvent } from 'react'
import { useNavigate } from 'react-router'
import { api } from '../api/endpoints'
import { useAuth, useCurrentUser } from '../auth/context'
import { Button, ErrorMessage, Notice, PageHeader, TextField } from '../components/ui'
import { formatDate, todayIso } from '../lib/format'
import { hasErrors, notInFuture, optionalDate, password as checkPassword, required, requiredText, type Errors } from '../lib/validation'

function ProfileForm() {
  const user = useCurrentUser()
  const { setUser } = useAuth()
  const [form, setForm] = useState({ first_name: user.first_name, last_name: user.last_name, date_of_birth: user.date_of_birth ?? '' })
  const [errors, setErrors] = useState<Errors<'first_name' | 'last_name' | 'date_of_birth'>>({})
  const save = useMutation({
    mutationFn: () =>
      api.updateMe({ first_name: form.first_name.trim(), last_name: form.last_name.trim(), date_of_birth: optionalDate(form.date_of_birth) }),
    onSuccess: setUser,
  })
  const submit = (event: FormEvent) => {
    event.preventDefault()
    const next = {
      first_name: requiredText(form.first_name, 100, 'First name'),
      last_name: requiredText(form.last_name, 100, 'Last name'),
      date_of_birth: notInFuture(form.date_of_birth, 'Date of birth'),
    }
    setErrors(next)
    if (!hasErrors(next)) save.mutate()
  }
  return (
    <form className="card form form-narrow" onSubmit={submit} noValidate aria-labelledby="account-details">
      <h2 id="account-details">Your details</h2>
      <ErrorMessage error={save.error} />
      {save.isSuccess && <Notice>Saved.</Notice>}
      <TextField label="Email" value={user.email} readOnly disabled hint="Your email can't be changed." />
      <div className="form-row">
        <TextField label="First name" value={form.first_name} error={errors.first_name} onChange={(e) => setForm({ ...form, first_name: e.target.value })} />
        <TextField label="Last name" value={form.last_name} error={errors.last_name} onChange={(e) => setForm({ ...form, last_name: e.target.value })} />
      </div>
      <TextField
        label="Date of birth"
        type="date"
        max={todayIso()}
        value={form.date_of_birth}
        error={errors.date_of_birth}
        onChange={(e) => setForm({ ...form, date_of_birth: e.target.value })}
      />
      <div className="form-actions">
        <Button type="submit" variant="primary" busy={save.isPending}>
          Save details
        </Button>
      </div>
    </form>
  )
}

function PasswordForm() {
  const [form, setForm] = useState({ current: '', next: '', confirm: '' })
  const [errors, setErrors] = useState<Errors<'current' | 'next' | 'confirm'>>({})
  const change = useMutation({
    mutationFn: () => api.changePassword({ current_password: form.current, new_password: form.next }),
    onSuccess: () => setForm({ current: '', next: '', confirm: '' }),
  })
  const submit = (event: FormEvent) => {
    event.preventDefault()
    const next = {
      current: required(form.current, 'Current password'),
      next: checkPassword(form.next, 'New password'),
      confirm: form.confirm !== form.next ? 'Passwords do not match.' : undefined,
    }
    setErrors(next)
    if (!hasErrors(next)) change.mutate()
  }
  return (
    <form className="card form form-narrow" onSubmit={submit} noValidate aria-labelledby="password-heading">
      <h2 id="password-heading">Change password</h2>
      <ErrorMessage error={change.error} />
      {change.isSuccess && <Notice>Password changed. Your other devices have been signed out.</Notice>}
      <TextField
        label="Current password"
        type="password"
        autoComplete="current-password"
        value={form.current}
        error={errors.current}
        onChange={(e) => setForm({ ...form, current: e.target.value })}
      />
      <TextField
        label="New password"
        type="password"
        autoComplete="new-password"
        hint="8–128 characters."
        value={form.next}
        error={errors.next}
        onChange={(e) => setForm({ ...form, next: e.target.value })}
      />
      <TextField
        label="Confirm new password"
        type="password"
        autoComplete="new-password"
        value={form.confirm}
        error={errors.confirm}
        onChange={(e) => setForm({ ...form, confirm: e.target.value })}
      />
      <div className="form-actions">
        <Button type="submit" variant="primary" busy={change.isPending}>
          Change password
        </Button>
      </div>
    </form>
  )
}

export function AccountPage() {
  const user = useCurrentUser()
  const { logout } = useAuth()
  const navigate = useNavigate()
  const [busy, setBusy] = useState(false)
  return (
    <>
      <PageHeader title="Account settings" subtitle={`Member since ${formatDate(user.created_at.slice(0, 10))}`} />
      <ProfileForm />
      <PasswordForm />
      <section className="card form-narrow" aria-labelledby="session-heading">
        <h2 id="session-heading">Session</h2>
        <p className="muted">Logging out ends this session; other devices stay signed in.</p>
        <Button
          busy={busy}
          onClick={() => {
            setBusy(true)
            void logout().then(() => navigate('/login', { replace: true }))
          }}
        >
          Log out
        </Button>
      </section>
    </>
  )
}
