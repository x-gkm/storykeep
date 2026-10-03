import { useState, type FormEvent, type ReactNode } from 'react'
import { FilePicker, UploadProgress } from '../../components/media'
import { Button, ErrorMessage, TextArea, TextField } from '../../components/ui'
import { validateFiles } from '../../lib/files'
import { isoToLocalInput, localInputToIso } from '../../lib/format'
import { hasErrors, maxLength, requiredText, type Errors } from '../../lib/validation'

type Field = 'title' | 'message' | 'unlock_at' | 'files'

export interface CapsuleFormResult {
  title: string
  /** `undefined` = keep the sealed message unchanged (edit only). */
  message: string | null | undefined
  unlock_at: string
  files: File[]
}

const HUNDRED_YEARS_MS = 100 * 365.25 * 24 * 3600 * 1000

/**
 * Create/edit form. When editing, the existing message can't be shown (it is sealed),
 * so the user explicitly chooses to replace it.
 */
export function CapsuleForm({
  mode,
  initial,
  submitLabel,
  onSubmit,
  busy,
  error,
  uploadProgress = null,
  cancel,
}: {
  mode: 'create' | 'edit'
  initial?: { title: string; unlock_at: string }
  submitLabel: string
  onSubmit: (result: CapsuleFormResult) => void
  busy: boolean
  error: unknown
  uploadProgress?: number | null
  cancel: ReactNode
}) {
  const [title, setTitle] = useState(initial?.title ?? '')
  const [message, setMessage] = useState('')
  const [replaceMessage, setReplaceMessage] = useState(mode === 'create')
  const [unlockAt, setUnlockAt] = useState(initial ? isoToLocalInput(initial.unlock_at) : '')
  const [files, setFiles] = useState<File[]>([])
  const [errors, setErrors] = useState<Errors<Field>>({})

  const submit = (event: FormEvent) => {
    event.preventDefault()
    const unlockMs = unlockAt ? new Date(unlockAt).getTime() : Number.NaN
    const next: Errors<Field> = {
      title: requiredText(title, 200, 'Title'),
      message: replaceMessage ? maxLength(message, 20_000, 'Message') : undefined,
      unlock_at: !unlockAt
        ? 'Unlock time is required.'
        : Number.isNaN(unlockMs)
          ? 'Enter a valid date and time.'
          : unlockMs <= Date.now()
            ? 'The unlock time must be in the future.'
            : unlockMs > Date.now() + HUNDRED_YEARS_MS
              ? 'The unlock time can be at most 100 years ahead.'
              : undefined,
      files: validateFiles(files),
    }
    setErrors(next)
    if (hasErrors(next)) return
    onSubmit({
      title: title.trim(),
      message: replaceMessage ? message.trim() || null : undefined,
      unlock_at: localInputToIso(unlockAt),
      files,
    })
  }

  return (
    <form className="card form" onSubmit={submit} noValidate>
      <ErrorMessage error={error} />
      <TextField label="Title" value={title} maxLength={200} error={errors.title} onChange={(e) => setTitle(e.target.value)} />
      <TextField
        label="Unlock at"
        type="datetime-local"
        value={unlockAt}
        error={errors.unlock_at}
        hint={`Your local time (${Intl.DateTimeFormat().resolvedOptions().timeZone}). Nobody can read the capsule before then.`}
        onChange={(e) => setUnlockAt(e.target.value)}
      />
      {mode === 'edit' && (
        <label className="checkbox">
          <input type="checkbox" checked={replaceMessage} onChange={(e) => setReplaceMessage(e.target.checked)} />
          Replace the sealed message (the current one can't be shown while the capsule is locked)
        </label>
      )}
      {replaceMessage && (
        <TextArea
          label={mode === 'edit' ? 'New message (leave empty to remove it)' : 'Message (optional)'}
          rows={8}
          value={message}
          error={errors.message}
          onChange={(e) => setMessage(e.target.value)}
        />
      )}
      {mode === 'create' && <FilePicker files={files} onChange={setFiles} disabled={busy} label="Media to seal (optional)" />}
      <UploadProgress fraction={uploadProgress} />
      <div className="form-actions">
        <Button type="submit" variant="primary" busy={busy}>
          {submitLabel}
        </Button>
        {cancel}
      </div>
    </form>
  )
}
