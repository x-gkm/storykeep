import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { Link, useLocation, useNavigate } from 'react-router'
import { api } from '../../api/endpoints'
import { keys, useCapsule, useRelationship } from '../../api/queries'
import type { Capsule, Media, Relationship } from '../../api/types'
import { useCurrentUser } from '../../auth/context'
import { MediaUploader } from '../../components/media'
import { Button, ErrorMessage, Loading, Notice, PageHeader } from '../../components/ui'
import { formatBytes } from '../../lib/format'
import { useIdParam } from '../../lib/params'
import { canEditItem, canWrite } from '../../lib/roles'
import { CapsuleForm } from './CapsuleForm'
import { CapsuleView } from './CapsuleView'

function LockedControls({ capsule, relationship, onChanged }: { capsule: Capsule; relationship: Relationship; onChanged: (c?: Capsule) => void }) {
  const user = useCurrentUser()
  const [editing, setEditing] = useState(false)
  // Media uploaded during this visit. Sealed media can't be listed, but its uploader may remove it while locked.
  const [uploaded, setUploaded] = useState<Media[]>([])
  const mayEdit = canEditItem(relationship.role, capsule.created_by, user.id)

  const update = useMutation({
    mutationFn: (input: Parameters<typeof api.updateCapsule>[1]) => api.updateCapsule(capsule.id, input),
    onSuccess: (updated) => {
      setEditing(false)
      onChanged(updated)
    },
  })
  const cancel = useMutation({ mutationFn: () => api.cancelCapsule(capsule.id), onSuccess: (updated) => onChanged(updated) })
  const removeMedia = useMutation({
    mutationFn: (media: Media) => api.removeCapsuleMedia(capsule.id, media.id),
    onSuccess: (_, media) => {
      setUploaded((list) => list.filter((m) => m.id !== media.id))
      onChanged()
    },
  })

  return (
    <>
      {mayEdit && (
        <div className="card">
          <div className="section-header">
            <h2>Manage capsule</h2>
            <div className="page-actions">
              {!editing && <Button onClick={() => setEditing(true)}>Edit</Button>}
              <Button
                variant="danger"
                busy={cancel.isPending}
                onClick={() => {
                  if (window.confirm('Cancel this capsule? Its content will never be shown to anyone. This cannot be undone.')) cancel.mutate()
                }}
              >
                Cancel capsule
              </Button>
            </div>
          </div>
          <ErrorMessage error={cancel.error} />
          {editing && (
            <CapsuleForm
              mode="edit"
              initial={{ title: capsule.title, unlock_at: capsule.unlock_at }}
              submitLabel="Save changes"
              busy={update.isPending}
              error={update.error}
              onSubmit={(result) =>
                update.mutate({
                  title: result.title,
                  unlock_at: result.unlock_at,
                  ...(result.message !== undefined ? { message: result.message } : {}),
                })
              }
              cancel={
                <Button variant="ghost" onClick={() => setEditing(false)}>
                  Cancel
                </Button>
              }
            />
          )}
        </div>
      )}
      {canWrite(relationship.role) && (
        <div className="card">
          <h2>Add media to the capsule</h2>
          <p className="muted">Files are sealed with the capsule and can't be viewed until it is opened.</p>
          <MediaUploader
            upload={(files, onProgress) =>
              api.uploadCapsuleMedia(capsule.id, files, onProgress).then((media) => setUploaded((list) => [...list, ...media]))
            }
            onUploaded={() => onChanged()}
          />
          {uploaded.length > 0 && (
            <>
              <h3>Added just now</h3>
              <ErrorMessage error={removeMedia.error} />
              <ul className="file-list">
                {uploaded.map((media) => (
                  <li key={media.id}>
                    <span>
                      {media.file_name} <span className="muted">({formatBytes(media.file_size)})</span>
                    </span>
                    <Button small variant="ghost" busy={removeMedia.isPending && removeMedia.variables?.id === media.id} onClick={() => removeMedia.mutate(media)}>
                      Remove
                    </Button>
                  </li>
                ))}
              </ul>
            </>
          )}
        </div>
      )}
    </>
  )
}

function CapsuleDetail({ capsule, relationship }: { capsule: Capsule; relationship: Relationship }) {
  const user = useCurrentUser()
  const navigate = useNavigate()
  const location = useLocation()
  const notice = (location.state as { notice?: string } | null)?.notice
  const queryClient = useQueryClient()

  const setCapsule = (updated?: Capsule) => {
    if (updated) queryClient.setQueryData(keys.capsule(capsule.id), updated)
    else void queryClient.invalidateQueries({ queryKey: keys.capsule(capsule.id) })
    void queryClient.invalidateQueries({ queryKey: keys.capsules(relationship.id) })
  }

  const open = useMutation({ mutationFn: () => api.openCapsule(capsule.id), onSuccess: (opened) => setCapsule(opened) })
  const remove = useMutation({
    mutationFn: () => api.deleteCapsule(capsule.id),
    onSuccess: () => {
      queryClient.removeQueries({ queryKey: keys.capsule(capsule.id) })
      void queryClient.invalidateQueries({ queryKey: keys.capsules(relationship.id) })
      navigate(`/relationships/${relationship.id}/capsules`)
    },
  })
  const mayDelete = canEditItem(relationship.role, capsule.created_by, user.id)

  return (
    <>
      <PageHeader
        back={<Link to={`/relationships/${relationship.id}/capsules`}>← {relationship.profile_name}'s capsules</Link>}
        title={capsule.title}
        actions={
          mayDelete && (
            <Button
              variant="danger"
              busy={remove.isPending}
              onClick={() => {
                if (window.confirm('Delete this capsule permanently?')) remove.mutate()
              }}
            >
              Delete
            </Button>
          )
        }
      />
      {notice && <Notice>{notice}</Notice>}
      <ErrorMessage error={remove.error ?? open.error} />
      <CapsuleView
        capsule={capsule}
        opening={open.isPending}
        onOpen={() => open.mutate()}
        // The server computes AVAILABLE on read; refetch when the countdown ends.
        onElapsed={() => setCapsule()}
        lockedActions={<LockedControls capsule={capsule} relationship={relationship} onChanged={setCapsule} />}
      />
    </>
  )
}

export function CapsuleDetailPage() {
  const id = useIdParam('capsuleId')
  const capsule = useCapsule(id)
  const relationship = useRelationship(capsule.data?.relationship_id ?? null)
  if (id === null) return <ErrorMessage error={new Error('Invalid capsule id.')} />
  if (capsule.error) return <ErrorMessage error={capsule.error} onRetry={() => void capsule.refetch()} />
  if (relationship.error) return <ErrorMessage error={relationship.error} />
  if (!capsule.data || !relationship.data) return <Loading label="Loading capsule…" />
  return <CapsuleDetail capsule={capsule.data} relationship={relationship.data} />
}
