import { useState, type FormEvent } from 'react'
import { Link, useLocation } from 'react-router'
import { useAuth } from '../auth/context'
import { Button, ErrorMessage, TextField } from '../components/ui'
import { todayIso } from '../lib/format'
import {
  email as checkEmail,
  hasErrors,
  notInFuture,
  optionalDate,
  password as checkPassword,
  requiredText,
  type Errors,
} from '../lib/validation'

type Field = 'email' | 'password' | 'confirm' | 'first_name' | 'last_name' | 'date_of_birth'

export function RegisterPage() {
  const { register } = useAuth()
  const location = useLocation()
  const [form, setForm] = useState({ email: '', password: '', confirm: '', first_name: '', last_name: '', date_of_birth: '' })
  const [errors, setErrors] = useState<Errors<Field>>({})
  const [submitError, setSubmitError] = useState<unknown>(null)
  const [busy, setBusy] = useState(false)
  const set = (field: Field) => (event: { target: { value: string } }) => setForm({ ...form, [field]: event.target.value })

  const onSubmit = async (event: FormEvent) => {
    event.preventDefault()
    const next: Errors<Field> = {
      email: checkEmail(form.email),
      password: checkPassword(form.password),
      confirm: form.confirm !== form.password ? 'Passwords do not match.' : undefined,
      first_name: requiredText(form.first_name, 100, 'First name'),
      last_name: requiredText(form.last_name, 100, 'Last name'),
      date_of_birth: notInFuture(form.date_of_birth, 'Date of birth'),
    }
    setErrors(next)
    if (hasErrors(next)) return
    setBusy(true)
    setSubmitError(null)
    try {
      await register({
        email: form.email.trim(),
        password: form.password,
        first_name: form.first_name.trim(),
        last_name: form.last_name.trim(),
        date_of_birth: optionalDate(form.date_of_birth),
      })
    } catch (error) {
      setSubmitError(error)
      setBusy(false)
    }
  }

  return (
    <>
      <h1>Create your account</h1>
      <form onSubmit={onSubmit} noValidate className="form">
        <ErrorMessage error={submitError} />
        <div className="form-row">
          <TextField label="First name" autoComplete="given-name" value={form.first_name} error={errors.first_name} onChange={set('first_name')} />
          <TextField label="Last name" autoComplete="family-name" value={form.last_name} error={errors.last_name} onChange={set('last_name')} />
        </div>
        <TextField label="Email" type="email" autoComplete="email" value={form.email} error={errors.email} onChange={set('email')} />
        <TextField
          label="Password"
          type="password"
          autoComplete="new-password"
          value={form.password}
          error={errors.password}
          hint="8–128 characters."
          onChange={set('password')}
        />
        <TextField
          label="Confirm password"
          type="password"
          autoComplete="new-password"
          value={form.confirm}
          error={errors.confirm}
          onChange={set('confirm')}
        />
        <TextField
          label="Date of birth (optional)"
          type="date"
          max={todayIso()}
          value={form.date_of_birth}
          error={errors.date_of_birth}
          onChange={set('date_of_birth')}
        />
        <Button type="submit" variant="primary" busy={busy}>
          Create account
        </Button>
      </form>
      <p className="auth-switch">
        Already have an account?{' '}
        <Link to="/login" state={location.state}>
          Log in
        </Link>
      </p>
    </>
  )
}
