import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState, type FormEvent } from 'react'
import { Link, useNavigate } from 'react-router'
import { api } from '../../api/endpoints'
import { keys, useProfile, useRelationships } from '../../api/queries'
import { PROFILE_TYPES, RELATIONSHIP_TYPES, type Profile, type ProfileType, type RelationshipType } from '../../api/types'
import { Badge, Button, ErrorMessage, Loading, PageHeader, RoleBadge, SelectField, TextField } from '../../components/ui'
import { formatDate, label, todayIso } from '../../lib/format'
import { useIdParam } from '../../lib/params'
import { canManage, isOwner } from '../../lib/roles'
import { hasErrors, notInFuture, optionalDate, requiredText, type Errors } from '../../lib/validation'
import { DevelopmentSection } from './DevelopmentSection'
import { MeasurementsSection } from './MeasurementsSection'

function EditProfileForm({ profile, onDone }: { profile: Profile; onDone: () => void }) {
  const queryClient = useQueryClient()
  const [form, setForm] = useState({ profile_type: profile.profile_type, name: profile.name, date_of_birth: profile.date_of_birth ?? '' })
  const [errors, setErrors] = useState<Errors<'name' | 'date_of_birth'>>({})
  const save = useMutation({
    mutationFn: () =>
      api.updateProfile(profile.id, { profile_type: form.profile_type, name: form.name.trim(), date_of_birth: optionalDate(form.date_of_birth) }),
    onSuccess: (updated) => {
      queryClient.setQueryData(keys.profile(profile.id), updated)
      void queryClient.invalidateQueries({ queryKey: keys.relationships })
      void queryClient.invalidateQueries({ queryKey: keys.profiles })
      onDone()
    },
  })
  const submit = (event: FormEvent) => {
    event.preventDefault()
    const next = { name: requiredText(form.name, 100, 'Name'), date_of_birth: notInFuture(form.date_of_birth, 'Birth date') }
    setErrors(next)
    if (!hasErrors(next)) save.mutate()
  }
  return (
    <form className="form" onSubmit={submit} noValidate>
      <ErrorMessage error={save.error} />
      <TextField label="Name" value={form.name} error={errors.name} maxLength={100} onChange={(e) => setForm({ ...form, name: e.target.value })} />
      <div className="form-row">
        <SelectField
          label="Profile type"
          options={PROFILE_TYPES}
          value={form.profile_type}
          hint={profile.profile_type === 'CHILD' ? 'A child with development records must stay a child profile.' : undefined}
          onChange={(e) => setForm({ ...form, profile_type: e.target.value as ProfileType })}
        />
        <TextField
          label="Birth date"
          type="date"
          max={todayIso()}
          value={form.date_of_birth}
          error={errors.date_of_birth}
          onChange={(e) => setForm({ ...form, date_of_birth: e.target.value })}
        />
      </div>
      <div className="form-actions">
        <Button type="submit" variant="primary" busy={save.isPending}>
          Save
        </Button>
        <Button variant="ghost" onClick={onDone}>
          Cancel
        </Button>
      </div>
    </form>
  )
}

function AddRelationshipForm({ profileId }: { profileId: number }) {
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const [type, setType] = useState<RelationshipType>('FAMILY')
  const [startedAt, setStartedAt] = useState('')
  const create = useMutation({
    mutationFn: () => api.createRelationship({ profile_id: profileId, relationship_type: type, started_at: optionalDate(startedAt), ended_at: null }),
    onSuccess: (relationship) => {
      void queryClient.invalidateQueries({ queryKey: keys.relationships })
      navigate(`/relationships/${relationship.id}/members`)
    },
  })
  return (
    <form
      className="form"
      onSubmit={(e) => {
        e.preventDefault()
        create.mutate()
      }}
    >
      <p className="muted">
        A separate relationship has its own members, timeline and capsules — e.g. one for grandparents alongside the parents'.
      </p>
      <ErrorMessage error={create.error} />
      <div className="form-row form-row-end">
        <SelectField label="Type" options={RELATIONSHIP_TYPES} value={type} onChange={(e) => setType(e.target.value as RelationshipType)} />
        <TextField label="Started (optional)" type="date" value={startedAt} onChange={(e) => setStartedAt(e.target.value)} />
        <Button type="submit" busy={create.isPending}>
          Add relationship
        </Button>
      </div>
    </form>
  )
}

function ProfileDetail({ profile }: { profile: Profile }) {
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const relationships = useRelationships()
  const [editing, setEditing] = useState(false)
  const mine = (relationships.data ?? []).filter((r) => r.profile_id === profile.id)
  const remove = useMutation({
    mutationFn: () => api.deleteProfile(profile.id),
    onSuccess: () => {
      queryClient.removeQueries({ queryKey: keys.profile(profile.id) })
      void queryClient.invalidateQueries({ queryKey: keys.relationships })
      void queryClient.invalidateQueries({ queryKey: keys.profiles })
      navigate('/')
    },
  })

  return (
    <>
      <PageHeader
        back={<Link to="/">← My relationships</Link>}
        title={profile.name}
        subtitle={
          <>
            <Badge tone="muted">{label(profile.profile_type)}</Badge>
            {profile.date_of_birth && <> · born {formatDate(profile.date_of_birth)}</>} · you are <RoleBadge role={profile.role} />
          </>
        }
        actions={
          canManage(profile.role) &&
          !editing && (
            <Button variant="secondary" onClick={() => setEditing(true)}>
              Edit profile
            </Button>
          )
        }
      />
      {editing && (
        <section className="card" aria-label="Edit profile">
          <EditProfileForm profile={profile} onDone={() => setEditing(false)} />
        </section>
      )}

      <section className="card" aria-labelledby="profile-relationships">
        <h2 id="profile-relationships">Your relationships with {profile.name}</h2>
        {mine.length === 0 ? (
          <p className="muted">None.</p>
        ) : (
          <ul className="relationship-list">
            {mine.map((r) => (
              <li key={r.id}>
                <Link to={`/relationships/${r.id}`} className="relationship-link">
                  <span className="relationship-type">{label(r.relationship_type)}</span>
                  <span className="muted">{r.started_at ? `Since ${formatDate(r.started_at)}` : ''}</span>
                  <RoleBadge role={r.role} />
                </Link>
              </li>
            ))}
          </ul>
        )}
        {canManage(profile.role) && (
          <details className="disclosure">
            <summary>Add another relationship</summary>
            <AddRelationshipForm profileId={profile.id} />
          </details>
        )}
      </section>

      {profile.profile_type === 'CHILD' && <DevelopmentSection profile={profile} />}
      <MeasurementsSection profile={profile} />

      {isOwner(profile.role) && (
        <section className="card danger-zone" aria-labelledby="delete-profile">
          <h2 id="delete-profile">Delete profile</h2>
          <p>
            Deletes {profile.name} with every relationship, memory, capsule, development record and measurement. You must be the owner of
            every relationship with this profile.
          </p>
          <ErrorMessage error={remove.error} />
          <Button
            variant="danger"
            busy={remove.isPending}
            onClick={() => {
              if (window.confirm(`Delete ${profile.name} and everything about them? This cannot be undone.`)) remove.mutate()
            }}
          >
            Delete profile
          </Button>
        </section>
      )}
    </>
  )
}

export function ProfilePage() {
  const id = useIdParam('profileId')
  const profile = useProfile(id)
  if (id === null) return <ErrorMessage error={new Error('Invalid profile id.')} />
  if (profile.isPending) return <Loading label="Loading profile…" />
  if (profile.error) return <ErrorMessage error={profile.error} onRetry={() => void profile.refetch()} />
  return <ProfileDetail key={profile.data.id} profile={profile.data} />
}
