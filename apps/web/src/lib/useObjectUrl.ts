import { useEffect, useState } from 'react'
import { fetchBlob } from '../api/client'

interface ObjectUrlState {
  url: string | null
  error: unknown
}

/**
 * Media content needs the Authorization header, so `<img src="/api/media/1/content">`
 * can't load it. Fetch it with the token and expose a blob: URL instead; the URL is
 * revoked when the component unmounts or the source changes.
 */
export function useObjectUrl(path: string | null): ObjectUrlState & { loading: boolean } {
  const [state, setState] = useState<ObjectUrlState & { path: string | null }>({ path: null, url: null, error: null })

  useEffect(() => {
    if (!path) return
    const controller = new AbortController()
    let objectUrl: string | null = null
    fetchBlob(path, controller.signal)
      .then((blob) => {
        objectUrl = URL.createObjectURL(blob)
        setState({ path, url: objectUrl, error: null })
      })
      .catch((error: unknown) => {
        if (!controller.signal.aborted) setState({ path, url: null, error })
      })
    return () => {
      controller.abort()
      if (objectUrl) URL.revokeObjectURL(objectUrl)
    }
  }, [path])

  const current = state.path === path
  return {
    url: current ? state.url : null,
    error: current ? state.error : null,
    loading: Boolean(path) && !current,
  }
}

/** Downloads a protected file and hands it to the browser (open in a new tab or save). */
export async function downloadProtected(path: string, fileName: string): Promise<void> {
  const blob = await fetchBlob(path)
  const url = URL.createObjectURL(blob)
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = fileName
  anchor.rel = 'noopener'
  document.body.appendChild(anchor)
  anchor.click()
  anchor.remove()
  // Give the browser time to start the download before revoking.
  setTimeout(() => URL.revokeObjectURL(url), 60_000)
}
