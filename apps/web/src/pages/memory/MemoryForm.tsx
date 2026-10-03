import { useState, type FormEvent, type ReactNode } from 'react'
import { MEMORY_CATEGORIES, type MemoryCategory, type MemoryInput, type Tag } from '../../api/types'
import { FilePicker, UploadProgress } from '../../components/media'
import { TagInput } from '../../components/TagInput'
import { Button, ErrorMessage, SelectField, TextArea, TextField } from '../../components/ui'
import { validateFiles } from '../../lib/files'
import { hasErrors, maxLength, optionalText, required, requiredText, type Errors } from '../../lib/validation'

type Field = 'title' | 'description' | 'memory_date' | 'tags' | 'files'

export interface MemoryFormValues {
  category: MemoryCategory
  title: string
  description: string
  memory_date: string
  tags: string[]
}

export function MemoryForm({
  initial,
  tagSuggestions,
  withFiles = false,
  submitLabel,
  onSubmit,
  error,
  busy,
  uploadProgress = null,
  cancel,
}: {
  initial: MemoryFormValues
  tagSuggestions: readonly Tag[]
  /** Show a file picker (for the create flow; uploads happen after the memory exists). */
  withFiles?: boolean
  submitLabel: string
  onSubmit: (input: MemoryInput, files: File[]) => void
  error: unknown
  busy: boolean
  uploadProgress?: number | null
  cancel: ReactNode
}) {
  const [values, setValues] = useState<MemoryFormValues>(initial)
  const [files, setFiles] = useState<File[]>([])
  const [errors, setErrors] = useState<Errors<Field>>({})

  const submit = (event: FormEvent) => {
    event.preventDefault()
    const next: Errors<Field> = {
      title: requiredText(values.title, 200, 'Title'),
      description: maxLength(values.description, 10_000, 'Description'),
      memory_date: required(values.memory_date, 'Date'),
      tags: values.tags.length > 20 ? 'At most 20 tags.' : undefined,
      files: withFiles ? validateFiles(files) : undefined,
    }
    setErrors(next)
    if (hasErrors(next)) return
    onSubmit(
      {
        category: values.category,
        title: values.title.trim(),
        description: optionalText(values.description),
        memory_date: values.memory_date,
        tags: values.tags,
      },
      files,
    )
  }

  return (
    <form className="card form" onSubmit={submit} noValidate>
      <ErrorMessage error={error} />
      <TextField
        label="Title"
        value={values.title}
        maxLength={200}
        error={errors.title}
        onChange={(e) => setValues({ ...values, title: e.target.value })}
      />
      <div className="form-row">
        <TextField
          label="When did it happen?"
          type="date"
          value={values.memory_date}
          error={errors.memory_date}
          hint="The event date — it orders the timeline."
          onChange={(e) => setValues({ ...values, memory_date: e.target.value })}
        />
        <SelectField
          label="Category"
          options={MEMORY_CATEGORIES}
          value={values.category}
          onChange={(e) => setValues({ ...values, category: e.target.value as MemoryCategory })}
        />
      </div>
      <TextArea
        label="Description (optional)"
        rows={6}
        value={values.description}
        error={errors.description}
        onChange={(e) => setValues({ ...values, description: e.target.value })}
      />
      <TagInput value={values.tags} onChange={(tags) => setValues({ ...values, tags })} suggestions={tagSuggestions} error={errors.tags} />
      {withFiles && <FilePicker files={files} onChange={setFiles} disabled={busy} label="Photos, videos, audio or documents (optional)" />}
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
