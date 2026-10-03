import { useId, useRef, useState, type ChangeEvent } from 'react'
import { Link } from 'react-router'
import type { Media } from '../api/types'
import { errorMessage } from '../lib/errors'
import { ACCEPTED_TYPES, validateFiles } from '../lib/files'
import { formatBytes } from '../lib/format'
import { downloadProtected, useObjectUrl } from '../lib/useObjectUrl'
import { Button, ErrorMessage } from './ui'

function MediaPreview({ media }: { media: Media }) {
  const needsBlob = media.media_type !== 'DOCUMENT'
  const { url, error, loading } = useObjectUrl(needsBlob ? media.content_url : null)

  if (media.media_type === 'DOCUMENT') {
    return <DocumentLink media={media} />
  }
  if (loading) return <div className="media-placeholder" aria-label={`Loading ${media.file_name}`} />
  if (error || !url) return <div className="media-placeholder media-failed">Couldn't load {media.file_name}</div>

  switch (media.media_type) {
    case 'IMAGE':
      return (
        <a href={url} target="_blank" rel="noopener noreferrer" className="media-image-link">
          <img src={url} alt={media.file_name} loading="lazy" />
        </a>
      )
    case 'VIDEO':
      return <video src={url} controls preload="metadata" aria-label={media.file_name} />
    case 'AUDIO':
      return <audio src={url} controls preload="metadata" aria-label={media.file_name} />
  }
}

function DocumentLink({ media }: { media: Media }) {
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<unknown>(null)
  return (
    <div className="media-document">
      <span className="media-doc-icon" aria-hidden="true">
        PDF
      </span>
      <Button
        small
        busy={busy}
        onClick={() => {
          setBusy(true)
          setError(null)
          downloadProtected(media.content_url, media.file_name)
            .catch(setError)
            .finally(() => setBusy(false))
        }}
      >
        Download
      </Button>
      {Boolean(error) && <span className="field-error">{errorMessage(error)}</span>}
    </div>
  )
}

function Thumb({ media }: { media: Media }) {
  const { url } = useObjectUrl(media.content_url)
  return url ? <img src={url} alt={media.file_name} loading="lazy" /> : <span className="thumb-placeholder" aria-hidden="true" />
}

/** Compact preview for timeline cards: up to four image thumbnails plus a count of the rest. */
export function MediaThumbs({ media, to }: { media: readonly Media[]; to: string }) {
  if (media.length === 0) return null
  const images = media.filter((m) => m.media_type === 'IMAGE').slice(0, 4)
  const rest = media.length - images.length
  return (
    <Link to={to} className="thumbs" aria-label={`${media.length} attachment${media.length === 1 ? '' : 's'} — open memory`}>
      {images.map((item) => (
        <Thumb key={item.id} media={item} />
      ))}
      {rest > 0 && (
        <span className="thumbs-more">
          {images.length > 0 ? '+' : ''}
          {rest} {images.length > 0 ? 'more' : `attachment${rest === 1 ? '' : 's'}`}
        </span>
      )}
    </Link>
  )
}

export function MediaGallery({
  media,
  onRemove,
  canRemove = false,
}: {
  media: readonly Media[]
  onRemove?: (media: Media) => Promise<unknown>
  canRemove?: boolean
}) {
  const [removing, setRemoving] = useState<number | null>(null)
  const [error, setError] = useState<unknown>(null)
  if (media.length === 0) return null
  return (
    <div>
      <ErrorMessage error={error} />
      <ul className="media-grid" aria-label="Attached media">
        {media.map((item) => (
          <li key={item.id} className={`media-item media-${item.media_type.toLowerCase()}`}>
            <MediaPreview media={item} />
            <div className="media-meta">
              <span className="media-name" title={item.file_name}>
                {item.file_name}
              </span>
              <span className="muted">{formatBytes(item.file_size)}</span>
              {canRemove && onRemove && (
                <Button
                  small
                  variant="ghost"
                  busy={removing === item.id}
                  aria-label={`Remove ${item.file_name}`}
                  onClick={() => {
                    if (!window.confirm(`Remove “${item.file_name}”?`)) return
                    setRemoving(item.id)
                    setError(null)
                    onRemove(item)
                      .catch(setError)
                      .finally(() => setRemoving(null))
                  }}
                >
                  Remove
                </Button>
              )}
            </div>
          </li>
        ))}
      </ul>
    </div>
  )
}

/** A multi-file picker with a list of the chosen files. Controlled by the parent. */
export function FilePicker({
  files,
  onChange,
  label = 'Add files',
  disabled,
}: {
  files: readonly File[]
  onChange: (files: File[]) => void
  label?: string
  disabled?: boolean
}) {
  const id = useId()
  const inputRef = useRef<HTMLInputElement>(null)
  const problem = validateFiles(files)
  return (
    <div className="field">
      <label htmlFor={id}>{label}</label>
      <input
        ref={inputRef}
        id={id}
        type="file"
        multiple
        accept={ACCEPTED_TYPES}
        disabled={disabled}
        onChange={(event: ChangeEvent<HTMLInputElement>) => {
          const chosen = Array.from(event.target.files ?? [])
          onChange([...files, ...chosen])
          event.target.value = ''
        }}
      />
      <p className="field-hint">Images, video, audio or PDF — up to 50 MB each, 20 per upload.</p>
      {files.length > 0 && (
        <ul className="file-list">
          {files.map((file, index) => (
            <li key={`${file.name}-${index}`}>
              <span>
                {file.name} <span className="muted">({formatBytes(file.size)})</span>
              </span>
              <button
                type="button"
                className="btn btn-ghost btn-small"
                disabled={disabled}
                aria-label={`Don't upload ${file.name}`}
                onClick={() => onChange(files.filter((_, i) => i !== index))}
              >
                ✕
              </button>
            </li>
          ))}
        </ul>
      )}
      {problem && <p className="field-error">{problem}</p>}
    </div>
  )
}

export function UploadProgress({ fraction }: { fraction: number | null }) {
  if (fraction === null) return null
  const percent = Math.round(fraction * 100)
  return (
    <div className="upload-progress">
      <progress max={100} value={percent} aria-label="Upload progress" />
      <span>{percent < 100 ? `Uploading… ${percent}%` : 'Processing…'}</span>
    </div>
  )
}

/** File picker + upload button for an existing memory or capsule. */
export function MediaUploader({
  upload,
  onUploaded,
}: {
  upload: (files: File[], onProgress: (fraction: number) => void) => Promise<unknown>
  onUploaded?: () => void
}) {
  const [files, setFiles] = useState<File[]>([])
  const [progress, setProgress] = useState<number | null>(null)
  const [error, setError] = useState<unknown>(null)
  const busy = progress !== null
  return (
    <div className="uploader">
      <FilePicker files={files} onChange={setFiles} disabled={busy} label="Upload media" />
      <UploadProgress fraction={progress} />
      <ErrorMessage error={error} />
      <Button
        variant="primary"
        disabled={files.length === 0 || Boolean(validateFiles(files))}
        busy={busy}
        onClick={() => {
          setError(null)
          setProgress(0)
          upload(files, setProgress)
            .then(() => {
              setFiles([])
              onUploaded?.()
            })
            .catch(setError)
            .finally(() => setProgress(null))
        }}
      >
        Upload {files.length > 0 ? `${files.length} file${files.length === 1 ? '' : 's'}` : ''}
      </Button>
    </div>
  )
}
