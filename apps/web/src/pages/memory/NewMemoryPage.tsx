import { useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { Link, useNavigate } from 'react-router'
import { api } from '../../api/endpoints'
import { keys, useRelationshipTags } from '../../api/queries'
import type { MemoryInput } from '../../api/types'
import { Notice } from '../../components/ui'
import { errorMessage } from '../../lib/errors'
import { todayIso } from '../../lib/format'
import { canWrite } from '../../lib/roles'
import { useRelationshipContext } from '../relationship/context'
import { MemoryForm } from './MemoryForm'

export function NewMemoryPage() {
  const { relationship } = useRelationshipContext()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const tags = useRelationshipTags(relationship.id)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<unknown>(null)
  const [progress, setProgress] = useState<number | null>(null)

  if (!canWrite(relationship.role)) {
    return <Notice>Your role ({relationship.role.toLowerCase()}) can view memories but not add them.</Notice>
  }

  const submit = async (input: MemoryInput, files: File[]) => {
    setBusy(true)
    setError(null)
    let memoryId: number | null = null
    try {
      const memory = await api.createMemory(relationship.id, input)
      memoryId = memory.id
      if (files.length > 0) {
        setProgress(0)
        await api.uploadMemoryMedia(memory.id, files, setProgress)
      }
      void queryClient.invalidateQueries({ queryKey: keys.timelineAll(relationship.id) })
      void queryClient.invalidateQueries({ queryKey: keys.relationshipTags(relationship.id) })
      navigate(`/memories/${memory.id}`)
    } catch (cause) {
      setBusy(false)
      setProgress(null)
      if (memoryId !== null) {
        // The memory exists; only the upload failed. Don't create a duplicate on retry.
        void queryClient.invalidateQueries({ queryKey: keys.timelineAll(relationship.id) })
        navigate(`/memories/${memoryId}/edit`, {
          state: { notice: `The memory was saved, but the upload failed: ${errorMessage(cause)} You can try again here.` },
        })
        return
      }
      setError(cause)
    }
  }

  return (
    <section aria-labelledby="new-memory-heading">
      <h2 id="new-memory-heading">New memory</h2>
      <MemoryForm
        initial={{ category: 'GENERAL', title: '', description: '', memory_date: todayIso(), tags: [] }}
        tagSuggestions={tags.data ?? []}
        withFiles
        submitLabel="Save memory"
        onSubmit={(input, files) => void submit(input, files)}
        error={error}
        busy={busy}
        uploadProgress={progress}
        cancel={
          <Link to={`/relationships/${relationship.id}`} className="btn btn-ghost">
            Cancel
          </Link>
        }
      />
    </section>
  )
}
