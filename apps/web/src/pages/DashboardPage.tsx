import { Link } from 'react-router'
import { useRelationships } from '../api/queries'
import type { Relationship } from '../api/types'
import { useCurrentUser } from '../auth/context'
import { Badge, EmptyState, PageHeader, QueryView, RoleBadge } from '../components/ui'
import { formatDate, label } from '../lib/format'

interface ProfileGroup {
  profileId: number
  name: string
  type: Relationship['profile_type']
  relationships: Relationship[]
}

function groupByProfile(relationships: readonly Relationship[]): ProfileGroup[] {
  const groups = new Map<number, ProfileGroup>()
  for (const relationship of relationships) {
    let group = groups.get(relationship.profile_id)
    if (!group) {
      group = {
        profileId: relationship.profile_id,
        name: relationship.profile_name,
        type: relationship.profile_type,
        relationships: [],
      }
      groups.set(relationship.profile_id, group)
    }
    group.relationships.push(relationship)
  }
  return [...groups.values()]
}

export function DashboardPage() {
  const user = useCurrentUser()
  const relationships = useRelationships()
  return (
    <>
      <PageHeader
        title="My relationships"
        subtitle={`Welcome back, ${user.first_name}.`}
        actions={
          <Link to="/profiles/new" className="btn btn-primary">
            New profile
          </Link>
        }
      />
      <QueryView query={relationships} loadingLabel="Loading your relationships…">
        {(list) =>
          list.length === 0 ? (
            <EmptyState title="No relationships yet">
              <p>Create a profile for a child, pet, friend or anyone else to start keeping memories together.</p>
              <Link to="/profiles/new" className="btn btn-primary">
                Create your first profile
              </Link>
            </EmptyState>
          ) : (
            <div className="profile-groups">
              {groupByProfile(list).map((group) => (
                <section key={group.profileId} className="card profile-group" aria-labelledby={`profile-${group.profileId}`}>
                  <div className="card-header">
                    <div>
                      <h2 id={`profile-${group.profileId}`}>
                        <Link to={`/profiles/${group.profileId}`}>{group.name}</Link>
                      </h2>
                      <Badge tone="muted">{label(group.type)}</Badge>
                    </div>
                    <Link to={`/profiles/${group.profileId}`} className="btn btn-ghost btn-small">
                      Profile
                    </Link>
                  </div>
                  <ul className="relationship-list">
                    {group.relationships.map((relationship) => (
                      <li key={relationship.id}>
                        <Link to={`/relationships/${relationship.id}`} className="relationship-link">
                          <span className="relationship-type">{label(relationship.relationship_type)}</span>
                          <span className="muted">
                            {relationship.started_at ? `Since ${formatDate(relationship.started_at)}` : 'No start date'}
                            {relationship.ended_at ? ` · ended ${formatDate(relationship.ended_at)}` : ''}
                          </span>
                          <RoleBadge role={relationship.role} />
                        </Link>
                      </li>
                    ))}
                  </ul>
                </section>
              ))}
            </div>
          )
        }
      </QueryView>
    </>
  )
}
