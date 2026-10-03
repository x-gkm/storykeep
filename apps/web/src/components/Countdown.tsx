import { useEffect, useRef, useState } from 'react'
import { formatCountdown } from '../lib/format'

/** Live countdown to `target`; calls `onElapsed` once when it reaches zero. */
export function Countdown({ target, onElapsed }: { target: string; onElapsed?: () => void }) {
  const targetMs = new Date(target).getTime()
  const [now, setNow] = useState(() => Date.now())
  const remaining = targetMs - now

  useEffect(() => {
    if (remaining <= 0) return
    // Tick every second when close, otherwise every 30 s.
    const interval = remaining < 3_600_000 ? 1000 : 30_000
    const timer = setTimeout(() => setNow(Date.now()), Math.min(interval, remaining))
    return () => clearTimeout(timer)
  }, [now, remaining])

  const elapsed = remaining <= 0
  const firedRef = useRef(false)
  useEffect(() => {
    if (elapsed && !firedRef.current) {
      firedRef.current = true
      onElapsed?.()
    }
  }, [elapsed, onElapsed])

  return (
    <span className="countdown">
      {remaining > 0 ? (
        <>
          Unlocks in <strong>{formatCountdown(remaining)}</strong>
        </>
      ) : (
        'Unlock time has passed'
      )}
    </span>
  )
}
