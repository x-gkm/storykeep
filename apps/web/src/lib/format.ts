import type { Author } from '../api/types'

const LABELS: Record<string, string> = {
  PARENT_CHILD: 'Parent & child',
  OWNER_PET: 'Owner & pet',
  FIRST_TIME: 'First time',
  SOCIAL_EMOTIONAL: 'Social & emotional',
  HEAD_CIRCUMFERENCE: 'Head circumference',
}

/** `"FIRST_TIME"` → `"First time"`. */
export function label(value: string): string {
  const known = LABELS[value]
  if (known) return known
  const lower = value.toLowerCase().replace(/_/g, ' ')
  return lower.charAt(0).toUpperCase() + lower.slice(1)
}

/** Formats a `YYYY-MM-DD` date without timezone shifts. */
export function formatDate(date: string | null | undefined): string {
  if (!date) return '—'
  const [y, m, d] = date.split('-').map(Number)
  if (!y || !m || !d) return date
  return new Date(Date.UTC(y, m - 1, d)).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    timeZone: 'UTC',
  })
}

/** Formats an RFC 3339 timestamp in the viewer's local time. */
export function formatDateTime(timestamp: string | null | undefined): string {
  if (!timestamp) return '—'
  const date = new Date(timestamp)
  if (Number.isNaN(date.getTime())) return timestamp
  return date.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })
}

export function fullName(person: Pick<Author, 'first_name' | 'last_name'>): string {
  return `${person.first_name} ${person.last_name}`.trim()
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  const units = ['KB', 'MB', 'GB']
  let value = bytes / 1024
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit += 1
  }
  return `${value.toFixed(value < 10 ? 1 : 0)} ${units[unit]}`
}

/** Today's date as `YYYY-MM-DD` in the viewer's time zone. */
export function todayIso(): string {
  const now = new Date()
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`
}

/** "3 days 4 hours" style countdown text. */
export function formatCountdown(ms: number): string {
  if (ms <= 0) return 'now'
  const totalMinutes = Math.floor(ms / 60_000)
  const days = Math.floor(totalMinutes / 1440)
  const hours = Math.floor((totalMinutes % 1440) / 60)
  const minutes = totalMinutes % 60
  const seconds = Math.floor((ms % 60_000) / 1000)
  const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? '' : 's'}`
  if (days >= 365) {
    const years = Math.floor(days / 365)
    return `${plural(years, 'year')} ${plural(days % 365, 'day')}`
  }
  if (days > 0) return `${plural(days, 'day')} ${plural(hours, 'hour')}`
  if (hours > 0) return `${plural(hours, 'hour')} ${plural(minutes, 'minute')}`
  if (minutes > 0) return `${plural(minutes, 'minute')} ${plural(seconds, 'second')}`
  return plural(seconds, 'second')
}

/** Converts an `<input type="datetime-local">` value (local time) to an RFC 3339 UTC timestamp. */
export function localInputToIso(value: string): string {
  return new Date(value).toISOString()
}

/** Converts an RFC 3339 timestamp to an `<input type="datetime-local">` value in local time. */
export function isoToLocalInput(timestamp: string): string {
  const date = new Date(timestamp)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`
}
