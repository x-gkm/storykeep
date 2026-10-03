import type { CapsuleStatus } from '../../api/types'
import { Badge } from '../../components/ui'

const TONES = { LOCKED: 'neutral', AVAILABLE: 'warn', OPENED: 'accent', CANCELLED: 'muted' } as const
const TEXT: Record<CapsuleStatus, string> = { LOCKED: 'Locked', AVAILABLE: 'Ready to open', OPENED: 'Opened', CANCELLED: 'Cancelled' }

export function CapsuleStatusBadge({ status }: { status: CapsuleStatus }) {
  return <Badge tone={TONES[status]}>{TEXT[status]}</Badge>
}
