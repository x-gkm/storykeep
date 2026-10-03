import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useState, type FormEvent } from 'react'
import { api } from '../../api/endpoints'
import { keys } from '../../api/queries'
import {
  DEVELOPMENT_DOMAINS,
  type DevelopmentDomain,
  type DevelopmentRecord,
  type DevelopmentRecordInput,
  type Profile,
} from '../../api/types'
import { useCurrentUser } from '../../auth/context'
import { Badge, Button, EmptyState, ErrorMessage, QueryView, SelectField, TextArea, TextField } from '../../components/ui'
import { formatDate, fullName, label, todayIso } from '../../lib/format'
import { canEditItem, canWrite } from '../../lib/roles'
import { maxLength, notBefore, notInFuture, optionalText, required } from '../../lib/validation'

interface DraftObservation {
  domain: DevelopmentDomain
  observation: string
}

function RecordForm({
  profile,
  initial,
  submitLabel,
  onSubmit,
  busy,
  error,
  onCancel,
}: {
  profile: Profile
  initial?: DevelopmentRecord
  submitLabel: string
  onSubmit: (input: DevelopmentRecordInput) => void
  busy: boolean
  error: unknown
  onCancel?: () => void
}) {
  const [date, setDate] = useState(initial?.record_date ?? todayIso())
  const [notes, setNotes] = useState(initial?.notes ?? '')
  const [observations, setObservations] = useState<DraftObservation[]>(
    initial?.observations.map((o) => ({ domain: o.domain, observation: o.observation })) ?? [{ domain: 'MOTOR', observation: '' }],
  )
  const [problem, setProblem] = useState<string>()

  const update = (index: number, change: Partial<DraftObservation>) =>
    setObservations(observations.map((o, i) => (i === index ? { ...o, ...change } : o)))

  const submit = (event: FormEvent) => {
    event.preventDefault()
    const filled = observations.filter((o) => o.observation.trim() !== '')
    const issue =
      required(date, 'Date') ??
      notInFuture(date) ??
      notBefore(date, profile.date_of_birth, `Date can't be before ${profile.name}'s birth date.`) ??
      maxLength(notes, 10_000, 'Notes') ??
      (filled.length > 50 ? 'At most 50 observations.' : undefined) ??
      filled.map((o) => maxLength(o.observation, 5000, 'An observation')).find(Boolean) ??
      (notes.trim() === '' && filled.length === 0 ? 'Add notes or at least one observation.' : undefined)
    setProblem(issue)
    if (issue) return
    onSubmit({
      record_date: date,
      notes: optionalText(notes),
      observations: filled.map((o) => ({ domain: o.domain, observation: o.observation.trim() })),
    })
  }

  return (
    <form className="form" onSubmit={submit} noValidate>
      <ErrorMessage error={error} />
      <TextField
        label="Date"
        type="date"
        max={todayIso()}
        min={profile.date_of_birth ?? undefined}
        value={date}
        onChange={(e) => setDate(e.target.value)}
      />
      <fieldset className="observations">
        <legend>Observations</legend>
        {observations.map((o, index) => (
          <div className="observation-row" key={index}>
            <SelectField
              label={`Domain ${index + 1}`}
              options={DEVELOPMENT_DOMAINS}
              value={o.domain}
              onChange={(e) => update(index, { domain: e.target.value as DevelopmentDomain })}
            />
            <TextField
              label={`Observation ${index + 1}`}
              value={o.observation}
              maxLength={5000}
              onChange={(e) => update(index, { observation: e.target.value })}
            />
            <Button small variant="ghost" aria-label={`Remove observation ${index + 1}`} onClick={() => setObservations(observations.filter((_, i) => i !== index))}>
              Remove
            </Button>
          </div>
        ))}
        <Button small onClick={() => setObservations([...observations, { domain: observations.at(-1)?.domain ?? 'MOTOR', observation: '' }])}>
          + Add observation
        </Button>
      </fieldset>
      <TextArea label="Notes (optional)" value={notes} onChange={(e) => setNotes(e.target.value)} />
      {problem && (
        <p className="field-error" role="alert">
          {problem}
        </p>
      )}
      <div className="form-actions">
        <Button type="submit" variant="primary" busy={busy}>
          {submitLabel}
        </Button>
        {onCancel && (
          <Button variant="ghost" onClick={onCancel}>
            Cancel
          </Button>
        )}
      </div>
    </form>
  )
}

function RecordCard({ record, profile, onChanged }: { record: DevelopmentRecord; profile: Profile; onChanged: () => void }) {
  const user = useCurrentUser()
  const [editing, setEditing] = useState(false)
  const editable = canEditItem(profile.role, record.created_by.id, user.id)
  const update = useMutation({
    mutationFn: (input: DevelopmentRecordInput) => api.updateDevelopmentRecord(record.id, input),
    onSuccess: () => {
      setEditing(false)
      onChanged()
    },
  })
  const remove = useMutation({ mutationFn: () => api.deleteDevelopmentRecord(record.id), onSuccess: onChanged })

  return (
    <li className="record">
      <div className="record-head">
        <h3>{formatDate(record.record_date)}</h3>
        <span className="muted">by {fullName(record.created_by)}</span>
        {editable && !editing && (
          <span className="row-actions">
            <Button small variant="ghost" onClick={() => setEditing(true)}>
              Edit
            </Button>
            <Button
              small
              variant="ghost"
              busy={remove.isPending}
              onClick={() => {
                if (window.confirm('Delete this development record and its observations?')) remove.mutate()
              }}
            >
              Delete
            </Button>
          </span>
        )}
      </div>
      <ErrorMessage error={remove.error} />
      {editing ? (
        <RecordForm
          profile={profile}
          initial={record}
          submitLabel="Save"
          busy={update.isPending}
          error={update.error}
          onSubmit={(input) => update.mutate(input)}
          onCancel={() => setEditing(false)}
        />
      ) : (
        <>
          {record.observations.length > 0 && (
            <ul className="observation-list">
              {record.observations.map((o) => (
                <li key={o.id}>
                  <Badge tone="neutral">{label(o.domain)}</Badge> {o.observation}
                </li>
              ))}
            </ul>
          )}
          {record.notes && <p className="prewrap">{record.notes}</p>}
        </>
      )}
    </li>
  )
}

/** Every observation grouped under its domain, newest first. */
function ByDomain({ records }: { records: readonly DevelopmentRecord[] }) {
  return (
    <div className="domain-grid">
      {DEVELOPMENT_DOMAINS.map((domain) => {
        const items = records.flatMap((r) => r.observations.filter((o) => o.domain === domain).map((o) => ({ ...o, date: r.record_date })))
        return (
          <section key={domain} className="domain-column" aria-labelledby={`domain-${domain}`}>
            <h3 id={`domain-${domain}`}>
              {label(domain)} <span className="muted">({items.length})</span>
            </h3>
            {items.length === 0 ? (
              <p className="muted">Nothing noted yet.</p>
            ) : (
              <ul>
                {items.map((item) => (
                  <li key={item.id}>
                    <span className="muted">{formatDate(item.date)}</span> — {item.observation}
                  </li>
                ))}
              </ul>
            )}
          </section>
        )
      })}
    </div>
  )
}

export function DevelopmentSection({ profile }: { profile: Profile }) {
  const queryClient = useQueryClient()
  const [view, setView] = useState<'records' | 'domains'>('domains')
  const [domainFilter, setDomain] = useState<DevelopmentDomain | ''>('')
  // The domain filter only applies to the per-record view; the domain view shows everything grouped.
  const domain = view === 'records' ? domainFilter : ''
  const query = { domain: domain || undefined }
  const records = useQuery({
    queryKey: keys.development(profile.id, query),
    queryFn: () => api.listDevelopmentRecords(profile.id, query),
  })
  const refresh = () => void queryClient.invalidateQueries({ queryKey: keys.developmentAll(profile.id) })
  const [formKey, setFormKey] = useState(0)
  const create = useMutation({ mutationFn: (input: DevelopmentRecordInput) => api.createDevelopmentRecord(profile.id, input) })

  return (
    <section className="card" aria-labelledby="development-heading">
      <div className="section-header">
        <h2 id="development-heading">Development</h2>
        <div className="segmented" role="group" aria-label="View">
          <button type="button" aria-pressed={view === 'domains'} onClick={() => setView('domains')}>
            By domain
          </button>
          <button type="button" aria-pressed={view === 'records'} onClick={() => setView('records')}>
            By record
          </button>
        </div>
      </div>
      <p className="muted">Notes and observations as you recorded them — no scoring or interpretation.</p>

      {canWrite(profile.role) && (
        <details className="disclosure">
          <summary>Add a development record</summary>
          <RecordForm
            key={formKey}
            profile={profile}
            submitLabel="Save record"
            busy={create.isPending}
            error={create.error}
            onSubmit={(input) =>
              create.mutate(input, {
                onSuccess: () => {
                  setFormKey((k) => k + 1)
                  refresh()
                },
              })
            }
          />
        </details>
      )}

      {view === 'records' && (
        <div className="filters-inline">
          <SelectField
            label="Domain"
            options={DEVELOPMENT_DOMAINS}
            emptyOption="All domains"
            value={domainFilter}
            onChange={(e) => setDomain(e.target.value as DevelopmentDomain | '')}
          />
        </div>
      )}

      <QueryView query={records} loadingLabel="Loading development records…">
        {(list) =>
          list.length === 0 && !domain ? (
            <EmptyState title="No development records yet" />
          ) : view === 'domains' ? (
            <ByDomain records={list} />
          ) : list.length === 0 ? (
            <EmptyState title="No records with observations in this domain" />
          ) : (
            <ul className="record-list">
              {list.map((record) => (
                <RecordCard key={record.id} record={record} profile={profile} onChanged={refresh} />
              ))}
            </ul>
          )
        }
      </QueryView>
    </section>
  )
}
