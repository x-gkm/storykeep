import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState, type FormEvent } from 'react'
import { Link, useNavigate } from 'react-router'
import { api } from '../api/endpoints'
import { keys } from '../api/queries'
import { PROFILE_TYPES, RELATIONSHIP_TYPES, type ProfileType, type RelationshipType } from '../api/types'
import { Button, ErrorMessage, PageHeader, SelectField, TextField } from '../components/ui'
import { todayIso } from '../lib/format'
import { hasErrors, notInFuture, optionalDate, requiredText, type Errors } from '../lib/validation'

/** A sensible default relationship for each profile type. */
const DEFAULT_RELATIONSHIP: Record<ProfileType, RelationshipType> = {
  CHILD: 'PARENT_CHILD',
  PET: 'OWNER_PET',
  PERSON: 'FRIEND',
  OTHER: 'OTHER',
}

type Field = 'name' | 'date_of_birth' | 'started_at'

export function NewProfilePage() {
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [form, setForm] = useState({
    profile_type: 'CHILD' as ProfileType,
    name: '',
    date_of_birth: '',
    relationship_type: 'PARENT_CHILD' as RelationshipType,
    started_at: '',
  })
  const [errors, setErrors] = useState<Errors<Field>>({})

  const create = useMutation({
    mutationFn: api.createProfile,
    onSuccess: (result) => {
      void queryClient.invalidateQueries({ queryKey: keys.relationships })
      void queryClient.invalidateQueries({ queryKey: keys.profiles })
      navigate(`/relationships/${result.relationship.id}`)
    },
  })

  const onSubmit = (event: FormEvent) => {
    event.preventDefault()
    const next: Errors<Field> = {
      name: requiredText(form.name, 100, 'Name'),
      date_of_birth: notInFuture(form.date_of_birth, 'Birth date'),
    }
    setErrors(next)
    if (hasErrors(next)) return
    create.mutate({
      profile_type: form.profile_type,
      name: form.name.trim(),
      date_of_birth: optionalDate(form.date_of_birth),
      relationship_type: form.relationship_type,
      started_at: optionalDate(form.started_at),
    })
  }

  return (
    <>
      <PageHeader
        title="New profile"
        subtitle="Who are these memories about? You'll be the owner of the new relationship and can invite others afterwards."
        back={<Link to="/">← My relationships</Link>}
      />
      <form className="card form form-narrow" onSubmit={onSubmit} noValidate>
        <ErrorMessage error={create.error} />
        <fieldset className="choice-group">
          <legend>Profile type</legend>
          <div className="choices">
            {PROFILE_TYPES.map((type) => (
              <label key={type} className={`choice${form.profile_type === type ? ' is-selected' : ''}`}>
                <input
                  type="radio"
                  name="profile_type"
                  value={type}
                  checked={form.profile_type === type}
                  onChange={() =>
                    setForm({ ...form, profile_type: type, relationship_type: DEFAULT_RELATIONSHIP[type] })
                  }
                />
                {type === 'CHILD' ? 'Child' : type === 'PET' ? 'Pet' : type === 'PERSON' ? 'Person' : 'Other'}
              </label>
            ))}
          </div>
        </fieldset>
        <TextField label="Name" value={form.name} error={errors.name} maxLength={100} onChange={(e) => setForm({ ...form, name: e.target.value })} />
        <TextField
          label="Birth date (optional)"
          type="date"
          max={todayIso()}
          value={form.date_of_birth}
          error={errors.date_of_birth}
          onChange={(e) => setForm({ ...form, date_of_birth: e.target.value })}
        />
        <SelectField
          label="Your relationship"
          options={RELATIONSHIP_TYPES}
          value={form.relationship_type}
          onChange={(e) => setForm({ ...form, relationship_type: e.target.value as RelationshipType })}
        />
        <TextField
          label="Relationship started (optional)"
          type="date"
          value={form.started_at}
          error={errors.started_at}
          onChange={(e) => setForm({ ...form, started_at: e.target.value })}
        />
        <div className="form-actions">
          <Button type="submit" variant="primary" busy={create.isPending}>
            Create profile
          </Button>
          <Link to="/" className="btn btn-ghost">
            Cancel
          </Link>
        </div>
      </form>
    </>
  )
}
