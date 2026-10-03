import { useId, useState, type KeyboardEvent } from 'react'
import type { Tag } from '../api/types'

const MAX_TAGS = 20
const MAX_TAG_LENGTH = 50

/** Chip-style tag editor with suggestions from existing tags (case-insensitive de-duplication). */
export function TagInput({
  value,
  onChange,
  suggestions,
  error,
}: {
  value: readonly string[]
  onChange: (tags: string[]) => void
  suggestions: readonly Tag[]
  error?: string
}) {
  const id = useId()
  const listId = `${id}-suggestions`
  const [draft, setDraft] = useState('')
  const [localError, setLocalError] = useState<string>()

  const has = (name: string) => value.some((tag) => tag.toLowerCase() === name.toLowerCase())

  const add = (raw: string) => {
    const name = raw.trim()
    if (!name) return
    if (name.length > MAX_TAG_LENGTH) {
      setLocalError(`Tags can be at most ${MAX_TAG_LENGTH} characters.`)
      return
    }
    if (value.length >= MAX_TAGS) {
      setLocalError(`At most ${MAX_TAGS} tags.`)
      return
    }
    // Reuse the existing spelling when the tag already exists.
    const existing = suggestions.find((tag) => tag.name.toLowerCase() === name.toLowerCase())
    if (!has(name)) onChange([...value, existing?.name ?? name])
    setDraft('')
    setLocalError(undefined)
  }

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Enter' || event.key === ',') {
      event.preventDefault()
      add(draft)
    } else if (event.key === 'Backspace' && draft === '' && value.length > 0) {
      onChange(value.slice(0, -1))
    }
  }

  const available = suggestions.filter((tag) => !has(tag.name))
  const shown = error ?? localError

  return (
    <div className={`field${shown ? ' field-invalid' : ''}`}>
      <label htmlFor={id}>Tags</label>
      {value.length > 0 && (
        <ul className="chips" aria-label="Selected tags">
          {value.map((tag) => (
            <li key={tag} className="chip">
              {tag}
              <button type="button" aria-label={`Remove tag ${tag}`} onClick={() => onChange(value.filter((t) => t !== tag))}>
                ✕
              </button>
            </li>
          ))}
        </ul>
      )}
      <div className="inline-input">
        <input
          id={id}
          list={listId}
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={onKeyDown}
          placeholder="Type a tag and press Enter"
          aria-describedby={`${id}-hint`}
        />
        <button type="button" className="btn btn-secondary" onClick={() => add(draft)} disabled={!draft.trim()}>
          Add
        </button>
      </div>
      <datalist id={listId}>
        {available.map((tag) => (
          <option key={tag.name} value={tag.name} />
        ))}
      </datalist>
      {available.length > 0 && (
        <div className="tag-suggestions" aria-label="Suggested tags">
          {available.slice(0, 12).map((tag) => (
            <button key={tag.name} type="button" className="chip chip-suggestion" onClick={() => add(tag.name)}>
              + {tag.name}
            </button>
          ))}
        </div>
      )}
      <p className="field-hint" id={`${id}-hint`}>
        Up to {MAX_TAGS} tags.
      </p>
      {shown && <p className="field-error">{shown}</p>}
    </div>
  )
}
