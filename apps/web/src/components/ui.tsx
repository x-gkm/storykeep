import { useId, type ButtonHTMLAttributes, type InputHTMLAttributes, type ReactNode, type SelectHTMLAttributes, type TextareaHTMLAttributes } from 'react'
import { errorMessage } from '../lib/errors'
import { label as humanize } from '../lib/format'

export function Loading({ label = 'Loading…' }: { label?: string }) {
  return (
    <div className="state state-loading" role="status" aria-live="polite">
      <span className="spinner" aria-hidden="true" />
      {label}
    </div>
  )
}

export function ErrorMessage({ error, onRetry }: { error: unknown; onRetry?: () => void }) {
  if (!error) return null
  return (
    <div className="alert alert-error" role="alert">
      <span>{errorMessage(error)}</span>
      {onRetry && (
        <button type="button" className="btn btn-small" onClick={onRetry}>
          Try again
        </button>
      )}
    </div>
  )
}

export function Notice({ children }: { children: ReactNode }) {
  return (
    <div className="alert alert-info" role="status">
      {children}
    </div>
  )
}

export function EmptyState({ title, children }: { title: string; children?: ReactNode }) {
  return (
    <div className="state state-empty">
      <p className="state-title">{title}</p>
      {children}
    </div>
  )
}

/** Renders loading / error states of a query, then `children` with the data. */
export function QueryView<T>({
  query,
  children,
  loadingLabel,
}: {
  query: { data: T | undefined; error: unknown; isPending: boolean; refetch: () => unknown }
  children: (data: T) => ReactNode
  loadingLabel?: string
}) {
  if (query.isPending) return <Loading label={loadingLabel} />
  if (query.error || query.data === undefined) return <ErrorMessage error={query.error} onRetry={() => void query.refetch()} />
  return <>{children(query.data)}</>
}

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: 'primary' | 'secondary' | 'danger' | 'ghost'
  busy?: boolean
  small?: boolean
}

export function Button({ variant = 'secondary', busy = false, small = false, className, disabled, children, type = 'button', ...rest }: ButtonProps) {
  const classes = ['btn', `btn-${variant}`, small ? 'btn-small' : '', className ?? ''].filter(Boolean).join(' ')
  return (
    <button type={type} className={classes} disabled={disabled || busy} aria-busy={busy || undefined} {...rest}>
      {busy && <span className="spinner spinner-inline" aria-hidden="true" />}
      {children}
    </button>
  )
}

interface FieldShellProps {
  label: string
  error?: string
  hint?: ReactNode
  children: (ids: { id: string; describedBy: string | undefined; invalid: boolean }) => ReactNode
}

function FieldShell({ label, error, hint, children }: FieldShellProps) {
  const id = useId()
  const hintId = hint ? `${id}-hint` : undefined
  const errorId = error ? `${id}-error` : undefined
  const describedBy = [hintId, errorId].filter(Boolean).join(' ') || undefined
  return (
    <div className={`field${error ? ' field-invalid' : ''}`}>
      <label htmlFor={id}>{label}</label>
      {children({ id, describedBy, invalid: Boolean(error) })}
      {hint && (
        <p className="field-hint" id={hintId}>
          {hint}
        </p>
      )}
      {error && (
        <p className="field-error" id={errorId}>
          {error}
        </p>
      )}
    </div>
  )
}

type TextFieldProps = InputHTMLAttributes<HTMLInputElement> & { label: string; error?: string; hint?: ReactNode }

export function TextField({ label, error, hint, ...input }: TextFieldProps) {
  return (
    <FieldShell label={label} error={error} hint={hint}>
      {({ id, describedBy, invalid }) => (
        <input id={id} aria-describedby={describedBy} aria-invalid={invalid || undefined} {...input} />
      )}
    </FieldShell>
  )
}

type TextAreaProps = TextareaHTMLAttributes<HTMLTextAreaElement> & { label: string; error?: string; hint?: ReactNode }

export function TextArea({ label, error, hint, ...input }: TextAreaProps) {
  return (
    <FieldShell label={label} error={error} hint={hint}>
      {({ id, describedBy, invalid }) => (
        <textarea id={id} aria-describedby={describedBy} aria-invalid={invalid || undefined} rows={4} {...input} />
      )}
    </FieldShell>
  )
}

type SelectFieldProps = Omit<SelectHTMLAttributes<HTMLSelectElement>, 'children'> & {
  label: string
  error?: string
  hint?: ReactNode
  options: readonly string[]
  /** Text for an empty first option, e.g. "Any category". */
  emptyOption?: string
  format?: (value: string) => string
}

export function SelectField({ label, error, hint, options, emptyOption, format = humanize, ...select }: SelectFieldProps) {
  return (
    <FieldShell label={label} error={error} hint={hint}>
      {({ id, describedBy, invalid }) => (
        <select id={id} aria-describedby={describedBy} aria-invalid={invalid || undefined} {...select}>
          {emptyOption !== undefined && <option value="">{emptyOption}</option>}
          {options.map((option) => (
            <option key={option} value={option}>
              {format(option)}
            </option>
          ))}
        </select>
      )}
    </FieldShell>
  )
}

export function Badge({ children, tone = 'neutral' }: { children: ReactNode; tone?: 'neutral' | 'accent' | 'warn' | 'muted' }) {
  return <span className={`badge badge-${tone}`}>{children}</span>
}

export function RoleBadge({ role }: { role: string }) {
  return <Badge tone={role === 'OWNER' ? 'accent' : 'neutral'}>{humanize(role)}</Badge>
}

export function PageHeader({ title, subtitle, actions, back }: { title: ReactNode; subtitle?: ReactNode; actions?: ReactNode; back?: ReactNode }) {
  return (
    <header className="page-header">
      {back && <div className="page-back">{back}</div>}
      <div className="page-header-row">
        <div>
          <h1>{title}</h1>
          {subtitle && <p className="page-subtitle">{subtitle}</p>}
        </div>
        {actions && <div className="page-actions">{actions}</div>}
      </div>
    </header>
  )
}
