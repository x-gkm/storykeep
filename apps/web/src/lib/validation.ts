// Client-side checks mirroring the API's rules, for fast feedback. The server stays the authority.
import { todayIso } from './format'

export type Errors<K extends string> = Partial<Record<K, string>>

export function hasErrors<K extends string>(errors: Errors<K>): boolean {
  return Object.values(errors).some(Boolean)
}

export function required(value: string, name = 'This field'): string | undefined {
  return value.trim() === '' ? `${name} is required.` : undefined
}

export function maxLength(value: string, max: number, name = 'This field'): string | undefined {
  return value.trim().length > max ? `${name} must be at most ${max} characters.` : undefined
}

export function requiredText(value: string, max: number, name: string): string | undefined {
  return required(value, name) ?? maxLength(value, max, name)
}

const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]+$/

export function email(value: string): string | undefined {
  if (value.trim() === '') return 'Email is required.'
  return EMAIL.test(value.trim()) ? undefined : 'Enter a valid email address.'
}

export function password(value: string, name = 'Password'): string | undefined {
  if (value.length < 8) return `${name} must be at least 8 characters.`
  if (value.length > 128) return `${name} must be at most 128 characters.`
  return undefined
}

/** `YYYY-MM-DD` dates compare correctly as strings. */
export function notInFuture(date: string, name = 'Date'): string | undefined {
  return date && date > todayIso() ? `${name} can't be in the future.` : undefined
}

export function notBefore(date: string, min: string | null | undefined, message: string): string | undefined {
  return date && min && date < min ? message : undefined
}

/** Measurement value: > 0, < 10 000 000, at most 3 decimal places. */
export function measurementValue(raw: string): string | undefined {
  if (raw.trim() === '') return 'Value is required.'
  const value = Number(raw)
  if (!Number.isFinite(value)) return 'Enter a number.'
  if (value <= 0) return 'Value must be greater than 0.'
  if (value >= 10_000_000) return 'Value must be less than 10,000,000.'
  const decimals = raw.trim().split('.')[1]
  if (decimals && decimals.length > 3) return 'Use at most 3 decimal places.'
  return undefined
}

/** Blank optional text → null, otherwise trimmed. */
export function optionalText(value: string): string | null {
  const trimmed = value.trim()
  return trimmed === '' ? null : trimmed
}

export function optionalDate(value: string): string | null {
  return value === '' ? null : value
}
