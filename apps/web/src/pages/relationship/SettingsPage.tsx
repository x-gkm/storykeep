import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState, type FormEvent } from 'react'
import { useNavigate } from 'react-router'
import { api } from '../../api/endpoints'
import { keys } from '../../api/queries'
import { RELATIONSHIP_TYPES, type RelationshipType } from '../../api/types'
import { Button, ErrorMessage, Notice, SelectField, TextField } from '../../components/ui'
import { canManage, isOwner } from '../../lib/roles'
import { optionalDate } from '../../lib/validation'
import { useRelationshipContext } from './context'

export function SettingsPage() {
  const { relationship } = useRelationshipContext()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [form, setForm] = useState({
    relationship_type: relationship.relationship_type,
    started_at: relationship.started_at ?? '',
    ended_at: relationship.ended_at ?? '',
  })
  const [dateError, setDateError] = useState<string>()

  const save = useMutation({
    mutationFn: () =>
      api.updateRelationship(relationship.id, {
        relationship_type: form.relationship_type,
        started_at: optionalDate(form.started_at),
        ended_at: optionalDate(form.ended_at),
      }),
    onSuccess: (updated) => {
      queryClient.setQueryData(keys.relationship(relationship.id), updated)
      void queryClient.invalidateQueries({ queryKey: keys.relationships })
    },
  })
  const remove = useMutation({
    mutationFn: () => api.deleteRelationship(relationship.id),
    onSuccess: () => {
      queryClient.removeQueries({ queryKey: keys.relationship(relationship.id) })
      void queryClient.invalidateQueries({ queryKey: keys.relationships })
      void queryClient.invalidateQueries({ queryKey: keys.profiles })
      navigate('/')
    },
  })

  if (!canManage(relationship.role)) return <Notice>Only owners and parents can change this relationship.</Notice>

  const submit = (event: FormEvent) => {
    event.preventDefault()
    const problem =
      form.started_at && form.ended_at && form.ended_at < form.started_at ? 'The end date must not be before the start date.' : undefined
    setDateError(problem)
    if (!problem) save.mutate()
  }

  return (
    <section aria-labelledby="settings-heading">
      <h2 id="settings-heading">Relationship settings</h2>
      <form className="card form form-narrow" onSubmit={submit} noValidate>
        <ErrorMessage error={save.error} />
        {save.isSuccess && <Notice>Saved.</Notice>}
        <SelectField
          label="Relationship type"
          options={RELATIONSHIP_TYPES}
          value={form.relationship_type}
          onChange={(e) => setForm({ ...form, relationship_type: e.target.value as RelationshipType })}
        />
        <div className="form-row">
          <TextField label="Started" type="date" value={form.started_at} onChange={(e) => setForm({ ...form, started_at: e.target.value })} />
          <TextField label="Ended" type="date" value={form.ended_at} error={dateError} onChange={(e) => setForm({ ...form, ended_at: e.target.value })} />
        </div>
        <div className="form-actions">
          <Button type="submit" variant="primary" busy={save.isPending}>
            Save
          </Button>
        </div>
      </form>

      {isOwner(relationship.role) && (
        <div className="card danger-zone">
          <h3>Delete relationship</h3>
          <p>
            Deletes this relationship with all its memories, time capsules and memberships. The profile itself stays if it has other
            relationships.
          </p>
          <ErrorMessage error={remove.error} />
          <Button
            variant="danger"
            busy={remove.isPending}
            onClick={() => {
              if (window.confirm(`Delete this relationship with ${relationship.profile_name} and everything in it? This cannot be undone.`)) {
                remove.mutate()
              }
            }}
          >
            Delete relationship
          </Button>
        </div>
      )}
    </section>
  )
}
