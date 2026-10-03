import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState, type FormEvent } from 'react'
import { useNavigate } from 'react-router'
import { api } from '../../api/endpoints'
import { keys, useMembers } from '../../api/queries'
import type { Member, Role } from '../../api/types'
import { useCurrentUser } from '../../auth/context'
import { Button, ErrorMessage, QueryView, RoleBadge, SelectField, TextField } from '../../components/ui'
import { formatDate, fullName, label } from '../../lib/format'
import { canManage, grantableRoles, isOwner } from '../../lib/roles'
import { email as checkEmail } from '../../lib/validation'
import { useRelationshipContext } from './context'

function AddMemberForm({ relationshipId, myRole }: { relationshipId: number; myRole: Role }) {
  const queryClient = useQueryClient()
  const roles = grantableRoles(myRole)
  const [email, setEmail] = useState('')
  const [role, setRole] = useState<Role>('MEMBER')
  const [emailError, setEmailError] = useState<string>()
  const add = useMutation({
    mutationFn: () => api.addMember(relationshipId, { email: email.trim(), role }),
    onSuccess: (members) => {
      queryClient.setQueryData(keys.members(relationshipId), members)
      setEmail('')
    },
  })
  const submit = (event: FormEvent) => {
    event.preventDefault()
    const problem = checkEmail(email)
    setEmailError(problem)
    if (!problem) add.mutate()
  }
  return (
    <form className="card form" onSubmit={submit} noValidate aria-labelledby="add-member-heading">
      <h3 id="add-member-heading">Add a member</h3>
      <p className="muted">They need a Storykeep account already. Invite them by the email they registered with.</p>
      <ErrorMessage error={add.error} />
      <div className="form-row form-row-end">
        <TextField label="Email" type="email" value={email} error={emailError} onChange={(e) => setEmail(e.target.value)} />
        <SelectField label="Role" options={roles} value={role} onChange={(e) => setRole(e.target.value as Role)} />
        <Button type="submit" variant="primary" busy={add.isPending}>
          Add
        </Button>
      </div>
      <p className="field-hint">
        Viewer: read only · Member: add memories &amp; content · Parent: also manage members · Owner: everything, including deletion.
      </p>
    </form>
  )
}

function MemberRow({ member, relationshipId, myRole, isMe }: { member: Member; relationshipId: number; myRole: Role; isMe: boolean }) {
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const [error, setError] = useState<unknown>(null)
  const change = useMutation({
    mutationFn: (role: Role) => api.updateMemberRole(relationshipId, member.user_id, role),
    onSuccess: (members) => {
      queryClient.setQueryData(keys.members(relationshipId), members)
      // My own role may have changed.
      void queryClient.invalidateQueries({ queryKey: keys.relationship(relationshipId) })
      void queryClient.invalidateQueries({ queryKey: keys.relationships })
    },
    onError: setError,
  })
  const remove = useMutation({
    mutationFn: () => api.removeMember(relationshipId, member.user_id),
    onSuccess: () => {
      if (isMe) {
        queryClient.removeQueries({ queryKey: keys.relationship(relationshipId) })
        void queryClient.invalidateQueries({ queryKey: keys.relationships })
        void queryClient.invalidateQueries({ queryKey: keys.profiles })
        navigate('/')
      } else {
        void queryClient.invalidateQueries({ queryKey: keys.members(relationshipId) })
      }
    },
    onError: setError,
  })

  // Managers may change roles; only an OWNER may touch an owner or grant OWNER.
  const mayChangeRole = canManage(myRole) && (member.role !== 'OWNER' || isOwner(myRole))
  const mayRemove = isMe || (canManage(myRole) && (member.role !== 'OWNER' || isOwner(myRole)))
  const roleOptions = grantableRoles(myRole)
  const options = roleOptions.includes(member.role) ? roleOptions : [member.role, ...roleOptions]

  return (
    <li className="member-row">
      <div className="member-info">
        <strong>
          {fullName(member)}
          {isMe && <span className="muted"> (you)</span>}
        </strong>
        <span className="muted">
          {member.email} · joined {formatDate(member.joined_at.slice(0, 10))}
        </span>
      </div>
      <div className="member-actions">
        {mayChangeRole ? (
          <SelectField
            label={`Role for ${fullName(member)}`}
            options={options}
            value={member.role}
            disabled={change.isPending}
            onChange={(e) => {
              setError(null)
              change.mutate(e.target.value as Role)
            }}
          />
        ) : (
          <RoleBadge role={member.role} />
        )}
        {mayRemove && (
          <Button
            small
            variant={isMe ? 'secondary' : 'danger'}
            busy={remove.isPending}
            onClick={() => {
              const question = isMe
                ? 'Leave this relationship? You will lose access to its memories and capsules.'
                : `Remove ${fullName(member)} (${label(member.role)}) from this relationship?`
              if (window.confirm(question)) {
                setError(null)
                remove.mutate()
              }
            }}
          >
            {isMe ? 'Leave' : 'Remove'}
          </Button>
        )}
      </div>
      {Boolean(error) && (
        <div className="member-error">
          <ErrorMessage error={error} />
        </div>
      )}
    </li>
  )
}

export function MembersPage() {
  const { relationship } = useRelationshipContext()
  const user = useCurrentUser()
  const members = useMembers(relationship.id)
  return (
    <section aria-labelledby="members-heading">
      <div className="section-header">
        <h2 id="members-heading">Members</h2>
      </div>
      <QueryView query={members} loadingLabel="Loading members…">
        {(list) => (
          <ul className="card member-list">
            {list.map((member) => (
              <MemberRow
                key={member.user_id}
                member={member}
                relationshipId={relationship.id}
                myRole={relationship.role}
                isMe={member.user_id === user.id}
              />
            ))}
          </ul>
        )}
      </QueryView>
      {canManage(relationship.role) && <AddMemberForm relationshipId={relationship.id} myRole={relationship.role} />}
    </section>
  )
}
