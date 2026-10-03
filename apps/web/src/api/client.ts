// Low-level HTTP layer: token storage, the shared error type and the request helpers.
// Every API call goes through `request`, `upload` or `fetchBlob` so that the bearer token
// is attached and errors / expired sessions are handled in one place.

const TOKEN_KEY = 'storykeep.token'

/**
 * The bearer token lives in localStorage so a session survives reloads and new tabs.
 * Trade-off: any script running on the page (i.e. an XSS bug) can read it. An httpOnly
 * cookie would avoid that but needs backend support (cookie auth + CSRF protection).
 */
export const tokenStore = {
  get(): string | null {
    try {
      return localStorage.getItem(TOKEN_KEY)
    } catch {
      return null
    }
  },
  set(token: string): void {
    try {
      localStorage.setItem(TOKEN_KEY, token)
    } catch {
      // Storage unavailable (private mode): the session lasts until reload.
    }
  },
  clear(): void {
    try {
      localStorage.removeItem(TOKEN_KEY)
    } catch {
      // ignore
    }
  },
}

/** Every failed call rejects with this. `status` is 0 for network failures. */
export class ApiError extends Error {
  readonly status: number
  readonly code: string

  constructor(status: number, code: string, message: string) {
    super(message)
    this.name = 'ApiError'
    this.status = status
    this.code = code
  }
}

export function isApiError(error: unknown): error is ApiError {
  return error instanceof ApiError
}

type UnauthorizedListener = () => void
const unauthorizedListeners = new Set<UnauthorizedListener>()

/** Called whenever the server rejects our token (401 `unauthorized`). Returns an unsubscribe function. */
export function onUnauthorized(listener: UnauthorizedListener): () => void {
  unauthorizedListeners.add(listener)
  return () => {
    unauthorizedListeners.delete(listener)
  }
}

function handleUnauthorized(error: ApiError, sentToken: boolean): void {
  // `invalid_credentials` (wrong password on login / password change) is also a 401,
  // but it doesn't mean the session is gone.
  if (error.status === 401 && error.code === 'unauthorized' && sentToken) {
    tokenStore.clear()
    unauthorizedListeners.forEach((listener) => listener())
  }
}

function isErrorBody(value: unknown): value is { error: { code: string; message: string } } {
  if (typeof value !== 'object' || value === null || !('error' in value)) return false
  const inner = (value as { error: unknown }).error
  return (
    typeof inner === 'object' &&
    inner !== null &&
    typeof (inner as { code?: unknown }).code === 'string' &&
    typeof (inner as { message?: unknown }).message === 'string'
  )
}

/** Builds an ApiError from a status and a (possibly non-JSON) response body. */
export function parseError(status: number, bodyText: string): ApiError {
  try {
    const parsed: unknown = JSON.parse(bodyText)
    if (isErrorBody(parsed)) return new ApiError(status, parsed.error.code, parsed.error.message)
  } catch {
    // not JSON
  }
  return new ApiError(status, 'unknown', bodyText.trim() || `Request failed with status ${status}`)
}

type QueryValue = string | number | undefined | null
export type Query = Record<string, QueryValue>

/** Serialises a query object, skipping empty values. */
export function buildQuery(query: Query | undefined): string {
  if (!query) return ''
  const params = new URLSearchParams()
  for (const [key, value] of Object.entries(query)) {
    if (value === undefined || value === null || value === '') continue
    params.set(key, String(value))
  }
  const text = params.toString()
  return text ? `?${text}` : ''
}

export interface RequestOptions {
  method?: 'GET' | 'POST' | 'PUT' | 'DELETE'
  body?: unknown
  query?: Query
  signal?: AbortSignal
}

/** JSON request against `/api/...`. Resolves to the parsed body, or `undefined` for 204. */
export async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const token = tokenStore.get()
  const headers: Record<string, string> = { Accept: 'application/json' }
  if (token) headers.Authorization = `Bearer ${token}`
  let body: string | undefined
  if (options.body !== undefined) {
    headers['Content-Type'] = 'application/json'
    body = JSON.stringify(options.body)
  }

  let response: Response
  try {
    response = await fetch(path + buildQuery(options.query), {
      method: options.method ?? 'GET',
      headers,
      body,
      signal: options.signal,
    })
  } catch (cause) {
    if (cause instanceof DOMException && cause.name === 'AbortError') throw cause
    throw new ApiError(0, 'network_error', 'Could not reach the server. Check your connection and try again.')
  }

  const text = await response.text()
  if (!response.ok) {
    const error = parseError(response.status, text)
    handleUnauthorized(error, Boolean(token))
    throw error
  }
  if (response.status === 204 || text === '') return undefined as T
  return JSON.parse(text) as T
}

/** Downloads a protected file (e.g. `/api/media/1/content`) with the token. */
export async function fetchBlob(path: string, signal?: AbortSignal): Promise<Blob> {
  const token = tokenStore.get()
  let response: Response
  try {
    response = await fetch(path, { headers: token ? { Authorization: `Bearer ${token}` } : {}, signal })
  } catch (cause) {
    if (cause instanceof DOMException && cause.name === 'AbortError') throw cause
    throw new ApiError(0, 'network_error', 'Could not reach the server.')
  }
  if (!response.ok) {
    const error = parseError(response.status, await response.text())
    handleUnauthorized(error, Boolean(token))
    throw error
  }
  return response.blob()
}

/**
 * Multipart upload with one `file` part per file. Uses XHR because fetch can't report
 * upload progress. `onProgress` receives a fraction between 0 and 1.
 */
export function upload<T>(path: string, files: readonly File[], onProgress?: (fraction: number) => void): Promise<T> {
  const token = tokenStore.get()
  const form = new FormData()
  for (const file of files) form.append('file', file, file.name)

  return new Promise<T>((resolve, reject) => {
    const xhr = new XMLHttpRequest()
    xhr.open('POST', path)
    xhr.setRequestHeader('Accept', 'application/json')
    if (token) xhr.setRequestHeader('Authorization', `Bearer ${token}`)
    if (onProgress) {
      xhr.upload.onprogress = (event) => {
        if (event.lengthComputable && event.total > 0) onProgress(event.loaded / event.total)
      }
    }
    xhr.onload = () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        onProgress?.(1)
        resolve(JSON.parse(xhr.responseText) as T)
        return
      }
      const error = parseError(xhr.status, xhr.responseText)
      handleUnauthorized(error, Boolean(token))
      reject(error)
    }
    xhr.onerror = () => reject(new ApiError(0, 'network_error', 'Upload failed: could not reach the server.'))
    xhr.send(form)
  })
}
