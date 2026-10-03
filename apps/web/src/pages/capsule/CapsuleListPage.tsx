import { useQuery } from '@tanstack/react-query'
import { useState } from 'react'
import { Link } from 'react-router'
import { api } from '../../api/endpoints'
import { keys } from '../../api/queries'
import { CAPSULE_STATUSES, type Capsule, type CapsuleStatus } from '../../api/types'
import { Countdown } from '../../components/Countdown'
import { EmptyState, QueryView, SelectField } from '../../components/ui'
import { formatDateTime, label } from '../../lib/format'
import { canWrite } from '../../lib/roles'
import { useRelationshipContext } from '../relationship/context'
import { CapsuleStatusBadge } from './CapsuleStatusBadge'

function CapsuleCard({ capsule }: { capsule: Capsule }) {
  return (
    <li className="card capsule-card">
      <div className="capsule-card-head">
        <h3>
          <Link to={`/capsules/${capsule.id}`}>{capsule.title}</Link>
        </h3>
        <CapsuleStatusBadge status={capsule.status} />
      </div>
      <p className="muted">
        Unlock: <time dateTime={capsule.unlock_at}>{formatDateTime(capsule.unlock_at)}</time>
        {capsule.media_count > 0 && ` · ${capsule.media_count} attachment${capsule.media_count === 1 ? '' : 's'}`}
      </p>
      {capsule.status === 'LOCKED' && <Countdown target={capsule.unlock_at} />}
      {capsule.status === 'AVAILABLE' && <p className="capsule-ready">Ready to open</p>}
    </li>
  )
}

export function CapsuleListPage() {
  const { relationship } = useRelationshipContext()
  const [status, setStatus] = useState<CapsuleStatus | ''>('')
  const capsules = useQuery({
    queryKey: [...keys.capsules(relationship.id), status],
    queryFn: () => api.listCapsules(relationship.id, status || undefined),
  })
  return (
    <section aria-labelledby="capsules-heading">
      <div className="section-header">
        <h2 id="capsules-heading">Time capsules</h2>
        {canWrite(relationship.role) && (
          <Link to="new" className="btn btn-primary">
            New capsule
          </Link>
        )}
      </div>
      <p className="muted">
        A capsule's message and media stay sealed — for everyone, including its author — until its unlock time passes and a member opens it.
      </p>
      <div className="filters-inline">
        <SelectField
          label="Status"
          options={CAPSULE_STATUSES}
          emptyOption="All"
          format={label}
          value={status}
          onChange={(e) => setStatus(e.target.value as CapsuleStatus | '')}
        />
      </div>
      <QueryView query={capsules} loadingLabel="Loading capsules…">
        {(list) =>
          list.length === 0 ? (
            <EmptyState title={status ? `No ${label(status).toLowerCase()} capsules` : 'No time capsules yet'}>
              {!status && canWrite(relationship.role) && (
                <Link to="new" className="btn btn-primary">
                  Seal a message for the future
                </Link>
              )}
            </EmptyState>
          ) : (
            <ul className="capsule-list">
              {list.map((capsule) => (
                <CapsuleCard key={capsule.id} capsule={capsule} />
              ))}
            </ul>
          )
        }
      </QueryView>
    </section>
  )
}
