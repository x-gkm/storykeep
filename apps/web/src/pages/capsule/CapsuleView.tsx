import type { ReactNode } from 'react'
import type { Capsule } from '../../api/types'
import { Countdown } from '../../components/Countdown'
import { MediaGallery } from '../../components/media'
import { Button } from '../../components/ui'
import { formatDateTime } from '../../lib/format'
import { CapsuleStatusBadge } from './CapsuleStatusBadge'

/**
 * Presents a capsule according to its status. The server never sends sealed content,
 * but this view also refuses to render `message`/`media` for any status other than
 * OPENED, so a locked capsule can't leak content through the UI either.
 */
export function CapsuleView({
  capsule,
  onOpen,
  opening = false,
  onElapsed,
  lockedActions,
}: {
  capsule: Capsule
  onOpen: () => void
  opening?: boolean
  onElapsed?: () => void
  /** Edit/cancel/upload controls shown while LOCKED (when the user may use them). */
  lockedActions?: ReactNode
}) {
  return (
    <div className="capsule-view">
      <div className="card">
        <div className="capsule-card-head">
          <CapsuleStatusBadge status={capsule.status} />
          <span className="muted">
            Unlock time: <time dateTime={capsule.unlock_at}>{formatDateTime(capsule.unlock_at)}</time>
          </span>
        </div>

        {capsule.status === 'LOCKED' && (
          <div className="capsule-sealed" data-testid="capsule-sealed">
            <svg className="capsule-lock-icon" viewBox="0 0 24 24" width="36" height="36" aria-hidden="true">
              <rect x="5" y="11" width="14" height="10" rx="2" fill="none" stroke="currentColor" strokeWidth="1.8" />
              <path d="M8 11V8a4 4 0 0 1 8 0v3" fill="none" stroke="currentColor" strokeWidth="1.8" />
            </svg>
            <p>
              <Countdown target={capsule.unlock_at} onElapsed={onElapsed} />
            </p>
            <p className="muted">
              The message{capsule.media_count > 0 ? ` and ${capsule.media_count} attachment${capsule.media_count === 1 ? '' : 's'}` : ''} stay
              sealed until then.
            </p>
          </div>
        )}

        {capsule.status === 'AVAILABLE' && (
          <div className="capsule-sealed">
            <p>The unlock time has passed. Opening it reveals the content to every member of this relationship.</p>
            <Button variant="primary" busy={opening} onClick={onOpen}>
              Open capsule
            </Button>
          </div>
        )}

        {capsule.status === 'CANCELLED' && (
          <div className="capsule-sealed">
            <p>This capsule was cancelled. Its content will never be shown.</p>
          </div>
        )}

        {capsule.status === 'OPENED' && (
          <div className="capsule-content" data-testid="capsule-content">
            {capsule.message ? <p className="prewrap capsule-message">{capsule.message}</p> : <p className="muted">This capsule has no message.</p>}
            {capsule.media && capsule.media.length > 0 && <MediaGallery media={capsule.media} />}
          </div>
        )}
      </div>
      {capsule.status === 'LOCKED' && lockedActions}
    </div>
  )
}
