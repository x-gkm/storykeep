import { isApiError } from '../api/client'

/** A message suitable for showing to the user. Prefers the API's own `error.message`. */
export function errorMessage(error: unknown): string {
  if (isApiError(error)) {
    switch (error.status) {
      case 403:
        return `You don't have permission to do that (${error.message}).`
      case 404:
        return `Not found — it may have been deleted, or you don't have access (${error.message}).`
      case 413:
        return `Too large: ${error.message}`
      default:
        return error.message.charAt(0).toUpperCase() + error.message.slice(1)
    }
  }
  if (error instanceof Error) return error.message
  return 'Something went wrong.'
}

export function isNotFound(error: unknown): boolean {
  return isApiError(error) && error.status === 404
}
