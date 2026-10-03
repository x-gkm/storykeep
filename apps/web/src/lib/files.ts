const MAX_FILE_BYTES = 50 * 1024 * 1024
const MAX_FILES = 20

/** File types the server accepts (it checks the content itself; this only filters the picker). */
export const ACCEPTED_TYPES =
  'image/jpeg,image/png,image/gif,image/webp,image/heic,image/heif,video/mp4,video/quicktime,video/webm,audio/mpeg,audio/mp4,audio/x-m4a,audio/ogg,audio/wav,audio/webm,application/pdf,.heic,.m4a,.mp3,.pdf'

/** Client-side checks mirroring the upload limits (50 MiB per file, 20 files per request, no empty files). */
export function validateFiles(files: readonly File[]): string | undefined {
  if (files.length > MAX_FILES) return `Upload at most ${MAX_FILES} files at a time.`
  const tooBig = files.find((file) => file.size > MAX_FILE_BYTES)
  if (tooBig) return `“${tooBig.name}” is larger than 50 MB.`
  const empty = files.find((file) => file.size === 0)
  if (empty) return `“${empty.name}” is empty.`
  return undefined
}
