import { useMutation, useQueryClient } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { Link, useNavigate } from 'react-router'
import { api } from '../../api/endpoints'
import { keys, useMemory, useRelationship } from '../../api/queries'
import type { Media, Memory, Relationship } from '../../api/types'
import { useCurrentUser } from '../../auth/context'
import { MediaGallery, MediaUploader } from '../../components/media'
import { Badge, Button, ErrorMessage, Loading, PageHeader } from '../../components/ui'
import { formatDate, formatDateTime, fullName, label } from '../../lib/format'
import { useIdParam } from '../../lib/params'
import { canEditItem, canWrite } from '../../lib/roles'

/** Loads a memory and the caller's role in its relationship. */
function useMemoryAndRelationship(id: number | null) {
  const memory = useMemory(id)
  const relationship = useRelationship(memory.data?.relationship_id ?? null)
  return { memory, relationship }
}

export function MemoryLoader({ children }: { children: (memory: Memory, relationship: Relationship) => ReactNode }) {
  const id = useIdParam('memoryId')
  const { memory, relationship } = useMemoryAndRelationship(id)
  if (id === null) return <ErrorMessage error={new Error('Invalid memory id.')} />
  if (memory.error) return <ErrorMessage error={memory.error} onRetry={() => void memory.refetch()} />
  if (relationship.error) return <ErrorMessage error={relationship.error} onRetry={() => void relationship.refetch()} />
  if (!memory.data || !relationship.data) return <Loading label="Loading memory…" />
  return <>{children(memory.data, relationship.data)}</>
}

/** Shared media management: remove (when allowed) and upload more. */
export function MemoryMedia({ memory, relationship }: { memory: Memory; relationship: Relationship }) {
  const user = useCurrentUser()
  const queryClient = useQueryClient()
  const refresh = () => {
    void queryClient.invalidateQueries({ queryKey: keys.memory(memory.id) })
    void queryClient.invalidateQueries({ queryKey: keys.timelineAll(relationship.id) })
  }
  // The server allows removal by the uploader (with write access) or by managers.
  const mayRemove = (media: Media) => canEditItem(relationship.role, media.uploaded_by.id, user.id)
  return (
    <section aria-labelledby="media-heading" className="card">
      <h2 id="media-heading">Media</h2>
      {memory.media.length === 0 && <p className="muted">No media attached.</p>}
      <MediaGallery
        media={memory.media}
        canRemove={mayRemove}
        onRemove={(media: Media) => api.removeMemoryMedia(memory.id, media.id).then(refresh)}
      />
      {canWrite(relationship.role) && (
        <MediaUploader upload={(files, onProgress) => api.uploadMemoryMedia(memory.id, files, onProgress)} onUploaded={refresh} />
      )}
    </section>
  )
}

function MemoryDetail({ memory, relationship }: { memory: Memory; relationship: Relationship }) {
  const user = useCurrentUser()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const editable = canEditItem(relationship.role, memory.created_by.id, user.id)
  const remove = useMutation({
    mutationFn: () => api.deleteMemory(memory.id),
    onSuccess: () => {
      queryClient.removeQueries({ queryKey: keys.memory(memory.id) })
      void queryClient.invalidateQueries({ queryKey: keys.timelineAll(relationship.id) })
      void queryClient.invalidateQueries({ queryKey: keys.relationshipTags(relationship.id) })
      navigate(`/relationships/${relationship.id}`)
    },
  })

  return (
    <>
      <PageHeader
        back={<Link to={`/relationships/${relationship.id}`}>← {relationship.profile_name}'s timeline</Link>}
        title={memory.title}
        subtitle={<Badge tone="accent">{label(memory.category)}</Badge>}
        actions={
          editable && (
            <>
              <Link to={`/memories/${memory.id}/edit`} className="btn btn-secondary">
                Edit
              </Link>
              <Button
                variant="danger"
                busy={remove.isPending}
                onClick={() => {
                  if (window.confirm('Delete this memory? Its media links and tags are removed too.')) remove.mutate()
                }}
              >
                Delete
              </Button>
            </>
          )
        }
      />
      <ErrorMessage error={remove.error} />
      <div className="card memory-detail">
        <dl className="facts">
          <div>
            <dt>Happened on</dt>
            <dd>
              <time dateTime={memory.memory_date}>{formatDate(memory.memory_date)}</time>
            </dd>
          </div>
          <div>
            <dt>Recorded</dt>
            <dd>
              <time dateTime={memory.created_at}>{formatDateTime(memory.created_at)}</time> by {fullName(memory.created_by)}
            </dd>
          </div>
          {memory.updated_at !== memory.created_at && (
            <div>
              <dt>Last edited</dt>
              <dd>
                <time dateTime={memory.updated_at}>{formatDateTime(memory.updated_at)}</time>
              </dd>
            </div>
          )}
        </dl>
        {memory.description ? <p className="memory-description prewrap">{memory.description}</p> : <p className="muted">No description.</p>}
        {memory.tags.length > 0 && (
          <ul className="chips" aria-label="Tags">
            {memory.tags.map((tag) => (
              <li key={tag}>
                <Link className="chip chip-button" to={`/relationships/${relationship.id}?tag=${encodeURIComponent(tag)}`}>
                  #{tag}
                </Link>
              </li>
            ))}
          </ul>
        )}
      </div>
      <MemoryMedia memory={memory} relationship={relationship} />
    </>
  )
}

export function MemoryDetailPage() {
  return <MemoryLoader>{(memory, relationship) => <MemoryDetail memory={memory} relationship={relationship} />}</MemoryLoader>
}
