import { useId, useState, type MouseEvent } from 'react'
import type { SeriesPoint } from '../api/types'
import { formatDate } from '../lib/format'

const WIDTH = 640
const HEIGHT = 260
const MARGIN = { top: 16, right: 20, bottom: 32, left: 52 }

function toTime(date: string): number {
  const [y, m, d] = date.split('-').map(Number)
  return Date.UTC(y ?? 1970, (m ?? 1) - 1, d ?? 1)
}

/** "Nice" round tick values covering [min, max]. */
function niceTicks(min: number, max: number, count = 5): number[] {
  if (min === max) {
    const pad = Math.abs(min) * 0.1 || 1
    min -= pad
    max += pad
  }
  const rawStep = (max - min) / count
  const magnitude = 10 ** Math.floor(Math.log10(rawStep))
  const step = [1, 2, 2.5, 5, 10].map((f) => f * magnitude).find((s) => s >= rawStep) ?? rawStep
  const start = Math.floor(min / step) * step
  const ticks: number[] = []
  for (let v = start; v <= max + step * 0.5; v += step) ticks.push(Number(v.toFixed(6)))
  if (ticks[ticks.length - 1]! < max) ticks.push(ticks[ticks.length - 1]! + step)
  return ticks
}

function formatValue(value: number): string {
  return Number.isInteger(value) ? String(value) : value.toFixed(value < 10 ? 2 : 1).replace(/\.?0+$/, '')
}

/**
 * A single-series line chart (one measurement type, one unit — never a dual axis).
 * Hovering anywhere over the plot shows the nearest point; the measurement table
 * beside it is the accessible, non-visual equivalent.
 */
export function LineChart({ title, unit, points }: { title: string; unit: string; points: readonly SeriesPoint[] }) {
  const titleId = useId()
  const [active, setActive] = useState<number | null>(null)
  if (points.length === 0) return null

  const times = points.map((p) => toTime(p.date))
  let tMin = Math.min(...times)
  let tMax = Math.max(...times)
  if (tMin === tMax) {
    tMin -= 86_400_000 * 15
    tMax += 86_400_000 * 15
  }
  const values = points.map((p) => p.value)
  const yTicks = niceTicks(Math.min(...values), Math.max(...values))
  const yMin = yTicks[0]!
  const yMax = yTicks[yTicks.length - 1]!

  const plotW = WIDTH - MARGIN.left - MARGIN.right
  const plotH = HEIGHT - MARGIN.top - MARGIN.bottom
  const x = (t: number) => MARGIN.left + ((t - tMin) / (tMax - tMin)) * plotW
  const y = (v: number) => MARGIN.top + plotH - ((v - yMin) / (yMax - yMin || 1)) * plotH

  const coords = points.map((p, i) => ({ x: x(times[i]!), y: y(p.value), point: p }))
  const path = coords.map((c, i) => `${i === 0 ? 'M' : 'L'}${c.x.toFixed(1)},${c.y.toFixed(1)}`).join(' ')

  const xTickCount = Math.min(5, Math.max(2, points.length))
  const xTicks = Array.from({ length: xTickCount }, (_, i) => tMin + ((tMax - tMin) * i) / (xTickCount - 1))
  const dateLabel = (t: number) =>
    new Date(t).toLocaleDateString(undefined, { year: '2-digit', month: 'short', timeZone: 'UTC' })

  const onMove = (event: MouseEvent<SVGRectElement>) => {
    const svg = event.currentTarget.ownerSVGElement
    if (!svg) return
    const rect = svg.getBoundingClientRect()
    const px = ((event.clientX - rect.left) / rect.width) * WIDTH
    let best = 0
    coords.forEach((c, i) => {
      if (Math.abs(c.x - px) < Math.abs(coords[best]!.x - px)) best = i
    })
    setActive(best)
  }

  const activeCoord = active !== null ? coords[active] : undefined
  const first = points[0]!
  const last = points[points.length - 1]!

  return (
    <figure className="chart">
      <figcaption id={titleId} className="chart-title">
        {title} <span className="muted">({unit})</span>
      </figcaption>
      <svg
        viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
        role="img"
        aria-labelledby={titleId}
        aria-describedby={`${titleId}-desc`}
        className="chart-svg"
      >
        <desc id={`${titleId}-desc`}>
          {points.length} measurements from {formatDate(first.date)} ({formatValue(first.value)} {unit}) to{' '}
          {formatDate(last.date)} ({formatValue(last.value)} {unit}).
        </desc>
        {yTicks.map((tick) => (
          <g key={tick}>
            <line className="chart-grid" x1={MARGIN.left} x2={WIDTH - MARGIN.right} y1={y(tick)} y2={y(tick)} />
            <text className="chart-axis-label" x={MARGIN.left - 8} y={y(tick)} dy="0.32em" textAnchor="end">
              {formatValue(tick)}
            </text>
          </g>
        ))}
        {xTicks.map((tick, i) => (
          <text
            key={tick}
            className="chart-axis-label"
            x={x(tick)}
            y={HEIGHT - 8}
            textAnchor={i === 0 ? 'start' : i === xTicks.length - 1 ? 'end' : 'middle'}
          >
            {dateLabel(tick)}
          </text>
        ))}
        <line className="chart-baseline" x1={MARGIN.left} x2={WIDTH - MARGIN.right} y1={MARGIN.top + plotH} y2={MARGIN.top + plotH} />
        {coords.length > 1 && <path className="chart-line" d={path} />}
        {activeCoord && (
          <line className="chart-crosshair" x1={activeCoord.x} x2={activeCoord.x} y1={MARGIN.top} y2={MARGIN.top + plotH} />
        )}
        {coords.map((c, i) => (
          <circle key={`${c.point.date}-${i}`} className={`chart-point${i === active ? ' is-active' : ''}`} cx={c.x} cy={c.y} r={i === active ? 6 : 4} />
        ))}
        <rect
          className="chart-hit"
          x={MARGIN.left - 10}
          y={MARGIN.top}
          width={plotW + 20}
          height={plotH}
          onMouseMove={onMove}
          onMouseLeave={() => setActive(null)}
        />
        {activeCoord && (
          <g
            className="chart-tooltip"
            transform={`translate(${Math.min(Math.max(activeCoord.x - 70, MARGIN.left), WIDTH - MARGIN.right - 140)},${Math.max(activeCoord.y - 52, 2)})`}
            pointerEvents="none"
          >
            <rect width={140} height={42} rx={6} />
            <text x={10} y={17} className="chart-tooltip-value">
              {formatValue(activeCoord.point.value)} {unit}
            </text>
            <text x={10} y={33} className="chart-tooltip-date">
              {formatDate(activeCoord.point.date)}
            </text>
          </g>
        )}
      </svg>
    </figure>
  )
}
