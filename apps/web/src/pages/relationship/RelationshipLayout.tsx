import { Link, NavLink, Outlet } from 'react-router'
import { useRelationship } from '../../api/queries'
import { Badge, ErrorMessage, Loading, PageHeader, RoleBadge } from '../../components/ui'
import { formatDate, label } from '../../lib/format'
import { useIdParam } from '../../lib/params'
import { canManage } from '../../lib/roles'
import type { RelationshipOutletContext } from './context'

export function RelationshipLayout() {
  const id = useIdParam('relationshipId')
  const query = useRelationship(id)
  if (id === null) return <ErrorMessage error={new Error('Invalid relationship id.')} />
  if (query.isPending) return <Loading />
  if (query.error) return <ErrorMessage error={query.error} onRetry={() => void query.refetch()} />
  const relationship = query.data
  const context: RelationshipOutletContext = { relationship }

  return (
    <>
      <PageHeader
        back={<Link to="/">← My relationships</Link>}
        title={relationship.profile_name}
        subtitle={
          <>
            <Badge tone="muted">{label(relationship.profile_type)}</Badge> {label(relationship.relationship_type)}
            {relationship.started_at && <> · since {formatDate(relationship.started_at)}</>}
            {relationship.ended_at && <> · ended {formatDate(relationship.ended_at)}</>} · you are <RoleBadge role={relationship.role} />
          </>
        }
        actions={
          <Link to={`/profiles/${relationship.profile_id}`} className="btn btn-secondary">
            Profile{relationship.profile_type === 'CHILD' ? ' & development' : ''}
          </Link>
        }
      />
      <nav className="tabs" aria-label="Relationship sections">
        <NavLink to="" end>
          Timeline
        </NavLink>
        <NavLink to="capsules">Time capsules</NavLink>
        <NavLink to="members">Members</NavLink>
        {canManage(relationship.role) && <NavLink to="settings">Settings</NavLink>}
      </nav>
      <Outlet context={context} />
    </>
  )
}
