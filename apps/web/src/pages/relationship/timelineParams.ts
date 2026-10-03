import { MEMORY_CATEGORIES, type MemoryCategory, type TimelineQuery } from '../../api/types'

export const PAGE_SIZE = 20

/** Timeline filters as kept in the URL (so they survive reloads and can be shared). */
export interface Filters {
  from: string
  to: string
  category: string
  tag: string
  q: string
  order: 'asc' | 'desc'
}

export const EMPTY_FILTERS: Filters = { from: '', to: '', category: '', tag: '', q: '', order: 'desc' }

export function readFilters(params: URLSearchParams): Filters {
  const category = params.get('category') ?? ''
  return {
    from: params.get('from') ?? '',
    to: params.get('to') ?? '',
    category: (MEMORY_CATEGORIES as readonly string[]).includes(category) ? category : '',
    tag: params.get('tag') ?? '',
    q: params.get('q') ?? '',
    order: params.get('order') === 'asc' ? 'asc' : 'desc',
  }
}

export function readPage(params: URLSearchParams): number {
  return Math.max(1, Number.parseInt(params.get('page') ?? '1', 10) || 1)
}

/** Filters + page → URL search params (empty values omitted). */
export function filtersToParams(filters: Filters, page = 1): URLSearchParams {
  const next = new URLSearchParams()
  if (filters.from) next.set('from', filters.from)
  if (filters.to) next.set('to', filters.to)
  if (filters.category) next.set('category', filters.category)
  if (filters.tag) next.set('tag', filters.tag)
  if (filters.q.trim()) next.set('q', filters.q.trim())
  if (filters.order === 'asc') next.set('order', 'asc')
  if (page > 1) next.set('page', String(page))
  return next
}

/** URL search params → timeline API query (page → limit/offset). */
export function timelineQueryFromParams(params: URLSearchParams): TimelineQuery {
  const filters = readFilters(params)
  return {
    from: filters.from || undefined,
    to: filters.to || undefined,
    category: (filters.category || undefined) as MemoryCategory | undefined,
    tag: filters.tag || undefined,
    q: filters.q.trim() || undefined,
    order: filters.order,
    limit: PAGE_SIZE,
    offset: (readPage(params) - 1) * PAGE_SIZE,
  }
}

/** Mirrors the API's rules for the filter values. */
export function validateFilters(filters: Filters): string | undefined {
  if (filters.from && filters.to && filters.to < filters.from) return 'The end date must not be before the start date.'
  if (filters.q.trim().length > 200) return 'Search text must be at most 200 characters.'
  return undefined
}
