import { useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { Link, useNavigate } from 'react-router'
import { api } from '../../api/endpoints'
import { keys } from '../../api/queries'
import { Notice } from '../../components/ui'
import { errorMessage } from '../../lib/errors'
import { canWrite } from '../../lib/roles'
import { useRelationshipContext } from '../relationship/context'
import { CapsuleForm, type CapsuleFormResult } from './CapsuleForm'

export function NewCapsulePage() {
  const { relationship } = useRelationshipContext()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<unknown>(null)
  const [progress, setProgress] = useState<number | null>(null)

  if (!canWrite(relationship.role)) return <Notice>Viewers can open capsules but not create them.</Notice>

  const submit = async (result: CapsuleFormResult) => {
    setBusy(true)
    setError(null)
    let capsuleId: number | null = null
    try {
      const capsule = await api.createCapsule(relationship.id, {
        title: result.title,
        message: result.message ?? null,
        unlock_at: result.unlock_at,
      })
      capsuleId = capsule.id
      if (result.files.length > 0) {
        setProgress(0)
        await api.uploadCapsuleMedia(capsule.id, result.files, setProgress)
      }
      void queryClient.invalidateQueries({ queryKey: keys.capsules(relationship.id) })
      navigate(`/capsules/${capsule.id}`)
    } catch (cause) {
      setBusy(false)
      setProgress(null)
      if (capsuleId !== null) {
        void queryClient.invalidateQueries({ queryKey: keys.capsules(relationship.id) })
        navigate(`/capsules/${capsuleId}`, {
          state: { notice: `The capsule was sealed, but the upload failed: ${errorMessage(cause)} You can upload again below.` },
        })
        return
      }
      setError(cause)
    }
  }

  return (
    <section aria-labelledby="new-capsule-heading">
      <h2 id="new-capsule-heading">New time capsule</h2>
      <CapsuleForm
        mode="create"
        submitLabel="Seal capsule"
        busy={busy}
        error={error}
        uploadProgress={progress}
        onSubmit={(result) => void submit(result)}
        cancel={
          <Link to={`/relationships/${relationship.id}/capsules`} className="btn btn-ghost">
            Cancel
          </Link>
        }
      />
    </section>
  )
}
