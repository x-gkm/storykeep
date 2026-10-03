import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Link, useLocation, useNavigate } from 'react-router'
import { api } from '../../api/endpoints'
import { keys, useRelationshipTags } from '../../api/queries'
import type { Memory, MemoryInput, Relationship } from '../../api/types'
import { useCurrentUser } from '../../auth/context'
import { Notice, PageHeader } from '../../components/ui'
import { canEditItem } from '../../lib/roles'
import { MemoryLoader, MemoryMedia } from './MemoryDetailPage'
import { MemoryForm } from './MemoryForm'

function EditMemory({ memory, relationship }: { memory: Memory; relationship: Relationship }) {
  const user = useCurrentUser()
  const navigate = useNavigate()
  const location = useLocation()
  const notice = (location.state as { notice?: string } | null)?.notice
  const queryClient = useQueryClient()
  const tags = useRelationshipTags(relationship.id)
  const save = useMutation({
    mutationFn: (input: MemoryInput) => api.updateMemory(memory.id, input),
    onSuccess: (updated) => {
      queryClient.setQueryData(keys.memory(memory.id), updated)
      void queryClient.invalidateQueries({ queryKey: keys.timelineAll(relationship.id) })
      void queryClient.invalidateQueries({ queryKey: keys.relationshipTags(relationship.id) })
      navigate(`/memories/${memory.id}`)
    },
  })

  const header = (
    <PageHeader back={<Link to={`/memories/${memory.id}`}>← Back to memory</Link>} title={`Edit “${memory.title}”`} />
  )

  if (!canEditItem(relationship.role, memory.created_by.id, user.id)) {
    return (
      <>
        {header}
        <Notice>Only the memory's author or a relationship owner/parent can edit it.</Notice>
      </>
    )
  }

  return (
    <>
      {header}
      {notice && <Notice>{notice}</Notice>}
      <MemoryForm
        initial={{
          category: memory.category,
          title: memory.title,
          description: memory.description ?? '',
          memory_date: memory.memory_date,
          tags: memory.tags,
        }}
        tagSuggestions={tags.data ?? []}
        submitLabel="Save changes"
        onSubmit={(input) => save.mutate(input)}
        error={save.error}
        busy={save.isPending}
        cancel={
          <Link to={`/memories/${memory.id}`} className="btn btn-ghost">
            Cancel
          </Link>
        }
      />
      <MemoryMedia memory={memory} relationship={relationship} />
    </>
  )
}

export function EditMemoryPage() {
  return <MemoryLoader>{(memory, relationship) => <EditMemory memory={memory} relationship={relationship} />}</MemoryLoader>
}
