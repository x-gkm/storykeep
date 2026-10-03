import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { useState, type FormEvent } from 'react'
import { Link, useSearchParams } from 'react-router'
import { api } from '../../api/endpoints'
import { keys, useRelationshipTags } from '../../api/queries'
import { MEMORY_CATEGORIES, type Memory } from '../../api/types'
import { MediaThumbs } from '../../components/media'
import { Badge, Button, EmptyState, ErrorMessage, Loading, SelectField, TextField } from '../../components/ui'
import { formatDate, formatDateTime, fullName, label } from '../../lib/format'
import { canWrite } from '../../lib/roles'
import { useRelationshipContext } from './context'
import {
  EMPTY_FILTERS,
  PAGE_SIZE,
  filtersToParams,
  readFilters,
  readPage,
  timelineQueryFromParams,
  validateFilters,
  type Filters,
} from './timelineParams'

function MemoryCard({ memory, onTag }: { memory: Memory; onTag: (tag: string) => void }) {
  return (
    <li className="timeline-item">
      <div className="timeline-date" aria-hidden="true">
        <span>{formatDate(memory.memory_date)}</span>
      </div>
      <article className="card memory-card">
        <header>
          <Badge tone="accent">{label(memory.category)}</Badge>
          <h3>
            <Link to={`/memories/${memory.id}`}>{memory.title}</Link>
          </h3>
          <p className="memory-dates">
            <span>
              <span className="muted">Happened</span> <time dateTime={memory.memory_date}>{formatDate(memory.memory_date)}</time>
            </span>
            <span>
              <span className="muted">Recorded</span> <time dateTime={memory.created_at}>{formatDateTime(memory.created_at)}</time> by{' '}
              {fullName(memory.created_by)}
            </span>
          </p>
        </header>
        {memory.description && <p className="memory-description">{memory.description}</p>}
        <MediaThumbs media={memory.media} to={`/memories/${memory.id}`} />
        {memory.tags.length > 0 && (
          <ul className="chips" aria-label="Tags">
            {memory.tags.map((tag) => (
              <li key={tag}>
                <button type="button" className="chip chip-button" onClick={() => onTag(tag)} title={`Show memories tagged ${tag}`}>
                  #{tag}
                </button>
              </li>
            ))}
          </ul>
        )}
      </article>
    </li>
  )
}

export function TimelinePage() {
  const { relationship } = useRelationshipContext()
  const [params, setParams] = useSearchParams()
  const applied = readFilters(params)
  const query = timelineQueryFromParams(params)
  const page = readPage(params)
  const [draft, setDraft] = useState<Filters>(applied)
  const [draftError, setDraftError] = useState<string>()
  const tags = useRelationshipTags(relationship.id)

  const timeline = useQuery({
    queryKey: keys.timeline(relationship.id, query),
    queryFn: () => api.listMemories(relationship.id, query),
    placeholderData: keepPreviousData,
  })

  const apply = (filters: Filters, nextPage = 1) => setParams(filtersToParams(filters, nextPage))

  const onSubmit = (event: FormEvent) => {
    event.preventDefault()
    const problem = validateFilters(draft)
    setDraftError(problem)
    if (!problem) apply(draft)
  }

  const clear = () => {
    setDraft(EMPTY_FILTERS)
    setDraftError(undefined)
    apply(EMPTY_FILTERS)
  }

  const filterByTag = (tag: string) => {
    const next = { ...applied, tag }
    setDraft(next)
    apply(next)
  }

  const isFiltered = Boolean(applied.from || applied.to || applied.category || applied.tag || applied.q)
  const total = timeline.data?.total ?? 0
  const pageCount = Math.max(1, Math.ceil(total / PAGE_SIZE))

  return (
    <section aria-labelledby="timeline-heading">
      <div className="section-header">
        <h2 id="timeline-heading">Timeline</h2>
        {canWrite(relationship.role) && (
          <Link to="memories/new" className="btn btn-primary">
            Add memory
          </Link>
        )}
      </div>

      <form className="card filters" onSubmit={onSubmit} aria-label="Filter memories" role="search">
        <TextField label="Search" type="search" placeholder="Title or description" value={draft.q} onChange={(e) => setDraft({ ...draft, q: e.target.value })} />
        <TextField label="From" type="date" value={draft.from} onChange={(e) => setDraft({ ...draft, from: e.target.value })} />
        <TextField label="To" type="date" value={draft.to} onChange={(e) => setDraft({ ...draft, to: e.target.value })} />
        <SelectField
          label="Category"
          options={MEMORY_CATEGORIES}
          emptyOption="Any category"
          value={draft.category}
          onChange={(e) => setDraft({ ...draft, category: e.target.value })}
        />
        <SelectField
          label="Tag"
          options={(tags.data ?? []).map((t) => t.name).concat(draft.tag && !tags.data?.some((t) => t.name === draft.tag) ? [draft.tag] : [])}
          format={(name) => name}
          emptyOption="Any tag"
          value={draft.tag}
          onChange={(e) => setDraft({ ...draft, tag: e.target.value })}
        />
        <SelectField
          label="Order"
          options={['desc', 'asc']}
          format={(v) => (v === 'desc' ? 'Newest first' : 'Oldest first')}
          value={draft.order}
          onChange={(e) => setDraft({ ...draft, order: e.target.value === 'asc' ? 'asc' : 'desc' })}
        />
        <div className="filters-actions">
          <Button type="submit" variant="primary">
            Apply
          </Button>
          {isFiltered && (
            <Button variant="ghost" onClick={clear}>
              Clear
            </Button>
          )}
        </div>
        {draftError && (
          <p className="field-error filters-error" role="alert">
            {draftError}
          </p>
        )}
      </form>

      {timeline.isPending ? (
        <Loading label="Loading memories…" />
      ) : timeline.error ? (
        <ErrorMessage error={timeline.error} onRetry={() => void timeline.refetch()} />
      ) : timeline.data.memories.length === 0 ? (
        <EmptyState title={isFiltered ? 'No memories match these filters' : 'No memories yet'}>
          {isFiltered ? (
            <Button variant="secondary" onClick={clear}>
              Clear filters
            </Button>
          ) : canWrite(relationship.role) ? (
            <Link to="memories/new" className="btn btn-primary">
              Add the first memory
            </Link>
          ) : (
            <p>Memories added by other members will appear here.</p>
          )}
        </EmptyState>
      ) : (
        <>
          <p className="muted result-count" aria-live="polite">
            {total} {total === 1 ? 'memory' : 'memories'}
            {isFiltered ? ' match' : ''} · sorted by the date they happened
          </p>
          <ol className="timeline" aria-busy={timeline.isFetching}>
            {timeline.data.memories.map((memory) => (
              <MemoryCard key={memory.id} memory={memory} onTag={filterByTag} />
            ))}
          </ol>
          {pageCount > 1 && (
            <nav className="pagination" aria-label="Timeline pages">
              <Button disabled={page <= 1} onClick={() => apply(applied, page - 1)}>
                ← Previous
              </Button>
              <span>
                Page {page} of {pageCount}
              </span>
              <Button disabled={page >= pageCount} onClick={() => apply(applied, page + 1)}>
                Next →
              </Button>
            </nav>
          )}
        </>
      )}
    </section>
  )
}
