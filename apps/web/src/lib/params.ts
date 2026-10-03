import { useParams } from 'react-router'

/** Reads a positive integer route parameter; `null` if missing or malformed. */
export function useIdParam(name: string): number | null {
  const raw = useParams()[name]
  if (!raw || !/^\d+$/.test(raw)) return null
  const id = Number(raw)
  return Number.isSafeInteger(id) && id > 0 ? id : null
}
