import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useState, type FormEvent } from 'react'
import { api } from '../../api/endpoints'
import { keys } from '../../api/queries'
import { MEASUREMENT_TYPES, MEASUREMENT_UNITS, type Measurement, type MeasurementInput, type MeasurementType, type Profile } from '../../api/types'
import { useCurrentUser } from '../../auth/context'
import { LineChart } from '../../components/LineChart'
import { Button, EmptyState, ErrorMessage, QueryView, SelectField, TextField } from '../../components/ui'
import { formatDate, fullName, label, todayIso } from '../../lib/format'
import { canEditItem, canWrite } from '../../lib/roles'
import { hasErrors, measurementValue, notBefore, notInFuture, required, type Errors } from '../../lib/validation'

function MeasurementForm({
  profile,
  initial,
  submitLabel,
  onSubmit,
  busy,
  error,
  onCancel,
}: {
  profile: Profile
  initial?: Measurement
  submitLabel: string
  onSubmit: (input: MeasurementInput) => void
  busy: boolean
  error: unknown
  onCancel?: () => void
}) {
  const [type, setType] = useState<MeasurementType>(initial?.measurement_type ?? 'HEIGHT')
  const [value, setValue] = useState(initial ? String(initial.value) : '')
  const [date, setDate] = useState(initial?.measurement_date ?? todayIso())
  const [errors, setErrors] = useState<Errors<'value' | 'date'>>({})

  const submit = (event: FormEvent) => {
    event.preventDefault()
    const next = {
      value: measurementValue(value),
      date:
        required(date, 'Date') ??
        notInFuture(date) ??
        notBefore(date, profile.date_of_birth, `Date can't be before ${profile.name}'s birth date.`),
    }
    setErrors(next)
    if (!hasErrors(next)) onSubmit({ measurement_type: type, value: Number(value), measurement_date: date })
  }

  return (
    <form className="form" onSubmit={submit} noValidate>
      <ErrorMessage error={error} />
      <div className="form-row form-row-end">
        <SelectField label="Type" options={MEASUREMENT_TYPES} value={type} onChange={(e) => setType(e.target.value as MeasurementType)} />
        <TextField
          label={`Value (${MEASUREMENT_UNITS[type]})`}
          type="number"
          inputMode="decimal"
          step="0.001"
          min="0"
          value={value}
          error={errors.value}
          onChange={(e) => setValue(e.target.value)}
        />
        <TextField
          label="Date"
          type="date"
          max={todayIso()}
          min={profile.date_of_birth ?? undefined}
          value={date}
          error={errors.date}
          onChange={(e) => setDate(e.target.value)}
        />
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

function MeasurementRow({ measurement, profile, onChanged }: { measurement: Measurement; profile: Profile; onChanged: () => void }) {
  const user = useCurrentUser()
  const [editing, setEditing] = useState(false)
  const editable = canEditItem(profile.role, measurement.created_by.id, user.id)
  const update = useMutation({
    mutationFn: (input: MeasurementInput) => api.updateMeasurement(measurement.id, input),
    onSuccess: () => {
      setEditing(false)
      onChanged()
    },
  })
  const remove = useMutation({ mutationFn: () => api.deleteMeasurement(measurement.id), onSuccess: onChanged })

  if (editing) {
    return (
      <tr>
        <td colSpan={5}>
          <MeasurementForm
            profile={profile}
            initial={measurement}
            submitLabel="Save"
            busy={update.isPending}
            error={update.error}
            onSubmit={(input) => update.mutate(input)}
            onCancel={() => setEditing(false)}
          />
        </td>
      </tr>
    )
  }
  return (
    <tr>
      <td>{formatDate(measurement.measurement_date)}</td>
      <td>{label(measurement.measurement_type)}</td>
      <td className="num">
        {measurement.value} {measurement.unit}
      </td>
      <td className="muted">{fullName(measurement.created_by)}</td>
      <td className="row-actions">
        {editable && (
          <>
            <Button small variant="ghost" onClick={() => setEditing(true)}>
              Edit
            </Button>
            <Button
              small
              variant="ghost"
              busy={remove.isPending}
              onClick={() => {
                if (window.confirm('Delete this measurement?')) remove.mutate()
              }}
            >
              Delete
            </Button>
          </>
        )}
        {Boolean(remove.error) && <ErrorMessage error={remove.error} />}
      </td>
    </tr>
  )
}

export function MeasurementsSection({ profile }: { profile: Profile }) {
  const queryClient = useQueryClient()
  const [typeFilter, setTypeFilter] = useState<MeasurementType | ''>('')
  const series = useQuery({ queryKey: keys.series(profile.id), queryFn: () => api.listMeasurementSeries(profile.id) })
  const list = useQuery({
    queryKey: [...keys.measurements(profile.id), 'list', typeFilter],
    queryFn: () => api.listMeasurements(profile.id, { type: typeFilter || undefined }),
  })
  const refresh = () => void queryClient.invalidateQueries({ queryKey: keys.measurements(profile.id) })
  const create = useMutation({ mutationFn: (input: MeasurementInput) => api.createMeasurement(profile.id, input), onSuccess: refresh })
  const [formKey, setFormKey] = useState(0)

  return (
    <section className="card" aria-labelledby="measurements-heading">
      <h2 id="measurements-heading">Measurements</h2>
      <p className="muted">Recorded values only — Storykeep doesn't compare them with growth standards.</p>

      <QueryView query={series} loadingLabel="Loading charts…">
        {(allSeries) =>
          allSeries.length === 0 ? null : (
            <div className="charts">
              {allSeries.map((s) => (
                <LineChart key={s.measurement_type} title={label(s.measurement_type)} unit={s.unit} points={s.points} />
              ))}
            </div>
          )
        }
      </QueryView>

      {canWrite(profile.role) && (
        <details className="disclosure">
          <summary>Add a measurement</summary>
          <MeasurementForm
            key={formKey}
            profile={profile}
            submitLabel="Add"
            busy={create.isPending}
            error={create.error}
            onSubmit={(input) => create.mutate(input, { onSuccess: () => setFormKey((k) => k + 1) })}
          />
        </details>
      )}

      <div className="filters-inline">
        <SelectField
          label="Show"
          options={MEASUREMENT_TYPES}
          emptyOption="All types"
          value={typeFilter}
          onChange={(e) => setTypeFilter(e.target.value as MeasurementType | '')}
        />
      </div>
      <QueryView query={list} loadingLabel="Loading measurements…">
        {(measurements) =>
          measurements.length === 0 ? (
            <EmptyState title="No measurements recorded" />
          ) : (
            <div className="table-wrap">
              <table className="table">
                <caption className="visually-hidden">Measurements, oldest first</caption>
                <thead>
                  <tr>
                    <th scope="col">Date</th>
                    <th scope="col">Type</th>
                    <th scope="col" className="num">
                      Value
                    </th>
                    <th scope="col">Recorded by</th>
                    <th scope="col">
                      <span className="visually-hidden">Actions</span>
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {measurements.map((m) => (
                    <MeasurementRow key={m.id} measurement={m} profile={profile} onChanged={refresh} />
                  ))}
                </tbody>
              </table>
            </div>
          )
        }
      </QueryView>
    </section>
  )
}
