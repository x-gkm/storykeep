// DTOs mirroring docs/api/*.md. Lookup values are exchanged by name.

export const PROFILE_TYPES = ['CHILD', 'PET', 'PERSON', 'OTHER'] as const
export type ProfileType = (typeof PROFILE_TYPES)[number]

export const RELATIONSHIP_TYPES = ['PARENT_CHILD', 'OWNER_PET', 'FRIEND', 'FAMILY', 'PARTNER', 'OTHER'] as const
export type RelationshipType = (typeof RELATIONSHIP_TYPES)[number]

export const ROLES = ['OWNER', 'PARENT', 'MEMBER', 'VIEWER'] as const
export type Role = (typeof ROLES)[number]

export const MEMORY_CATEGORIES = ['GENERAL', 'MILESTONE', 'BIRTHDAY', 'HOLIDAY', 'TRAVEL', 'FIRST_TIME', 'EVERYDAY'] as const
export type MemoryCategory = (typeof MEMORY_CATEGORIES)[number]

export const MEDIA_TYPES = ['IMAGE', 'VIDEO', 'AUDIO', 'DOCUMENT'] as const
export type MediaType = (typeof MEDIA_TYPES)[number]

export const DEVELOPMENT_DOMAINS = ['PHYSICAL', 'MOTOR', 'LANGUAGE', 'COGNITIVE', 'SOCIAL_EMOTIONAL'] as const
export type DevelopmentDomain = (typeof DEVELOPMENT_DOMAINS)[number]

export const MEASUREMENT_TYPES = ['HEIGHT', 'WEIGHT', 'HEAD_CIRCUMFERENCE'] as const
export type MeasurementType = (typeof MEASUREMENT_TYPES)[number]
export const MEASUREMENT_UNITS: Record<MeasurementType, string> = { HEIGHT: 'cm', WEIGHT: 'kg', HEAD_CIRCUMFERENCE: 'cm' }

export const CAPSULE_STATUSES = ['LOCKED', 'AVAILABLE', 'OPENED', 'CANCELLED'] as const
export type CapsuleStatus = (typeof CAPSULE_STATUSES)[number]

/** `YYYY-MM-DD` */
export type IsoDate = string
/** RFC 3339 timestamp in UTC */
export type Timestamp = string

export interface Reference {
  profile_types: ProfileType[]
  relationship_types: RelationshipType[]
  relationship_roles: Role[]
  memory_categories: MemoryCategory[]
  media_types: MediaType[]
  development_domains: DevelopmentDomain[]
  measurement_types: { name: MeasurementType; unit: string }[]
  time_capsule_statuses: CapsuleStatus[]
}

// ---- Auth & users ----

export interface User {
  id: number
  email: string
  first_name: string
  last_name: string
  date_of_birth: IsoDate | null
  created_at: Timestamp
  updated_at: Timestamp
}

export interface Session {
  token: string
  expires_at: Timestamp
  user: User
}

export interface RegisterInput {
  email: string
  password: string
  first_name: string
  last_name: string
  date_of_birth: IsoDate | null
}

export interface LoginInput {
  email: string
  password: string
}

export interface UpdateMeInput {
  first_name: string
  last_name: string
  date_of_birth: IsoDate | null
}

export interface ChangePasswordInput {
  current_password: string
  new_password: string
}

// ---- Profiles, relationships, members ----

export interface Profile {
  id: number
  profile_type: ProfileType
  name: string
  date_of_birth: IsoDate | null
  created_at: Timestamp
  updated_at: Timestamp
  /** Your strongest role across your relationships with the profile. */
  role: Role
}

export interface Relationship {
  id: number
  profile_id: number
  profile_name: string
  profile_type: ProfileType
  relationship_type: RelationshipType
  started_at: IsoDate | null
  ended_at: IsoDate | null
  created_at: Timestamp
  /** Your role in this relationship. */
  role: Role
}

export interface Member {
  user_id: number
  email: string
  first_name: string
  last_name: string
  role: Role
  joined_at: Timestamp
}

export interface CreateProfileInput {
  profile_type: ProfileType
  name: string
  date_of_birth: IsoDate | null
  relationship_type: RelationshipType
  started_at: IsoDate | null
}

export interface CreateProfileResult {
  profile: Profile
  relationship: Relationship
}

export interface UpdateProfileInput {
  profile_type: ProfileType
  name: string
  date_of_birth: IsoDate | null
}

export interface CreateRelationshipInput {
  profile_id: number
  relationship_type: RelationshipType
  started_at: IsoDate | null
  ended_at: IsoDate | null
}

export interface UpdateRelationshipInput {
  relationship_type: RelationshipType
  started_at: IsoDate | null
  ended_at: IsoDate | null
}

// ---- Memories, timeline, tags ----

export interface Author {
  id: number
  first_name: string
  last_name: string
}

export interface Media {
  id: number
  media_type: MediaType
  file_name: string
  mime_type: string
  file_size: number
  uploaded_by: Author
  created_at: Timestamp
  content_url: string
}

export interface Memory {
  id: number
  relationship_id: number
  category: MemoryCategory
  title: string
  description: string | null
  /** When the event happened. */
  memory_date: IsoDate
  created_by: Author
  /** When the memory was recorded. */
  created_at: Timestamp
  updated_at: Timestamp
  tags: string[]
  media: Media[]
}

export interface MemoryInput {
  category: MemoryCategory
  title: string
  description: string | null
  memory_date: IsoDate
  tags: string[]
}

export interface TimelineQuery {
  from?: IsoDate
  to?: IsoDate
  category?: MemoryCategory
  tag?: string
  q?: string
  order?: 'asc' | 'desc'
  limit?: number
  offset?: number
}

export interface TimelinePage {
  memories: Memory[]
  total: number
  limit: number
  offset: number
}

export interface Tag {
  name: string
  memory_count: number
}

// ---- Development & measurements ----

export interface Observation {
  id: number
  domain: DevelopmentDomain
  observation: string
  created_at: Timestamp
}

export interface DevelopmentRecord {
  id: number
  profile_id: number
  record_date: IsoDate
  notes: string | null
  created_by: Author
  created_at: Timestamp
  observations: Observation[]
}

export interface DevelopmentRecordInput {
  record_date: IsoDate
  notes: string | null
  observations: { domain: DevelopmentDomain; observation: string }[]
}

export interface DevelopmentQuery {
  from?: IsoDate
  to?: IsoDate
  domain?: DevelopmentDomain
}

export interface Measurement {
  id: number
  profile_id: number
  measurement_type: MeasurementType
  unit: string
  value: number
  measurement_date: IsoDate
  created_by: Author
  created_at: Timestamp
}

export interface MeasurementInput {
  measurement_type: MeasurementType
  value: number
  measurement_date: IsoDate
}

export interface MeasurementQuery {
  type?: MeasurementType
  from?: IsoDate
  to?: IsoDate
}

export interface SeriesPoint {
  date: IsoDate
  value: number
}

export interface MeasurementSeries {
  measurement_type: MeasurementType
  unit: string
  points: SeriesPoint[]
}

// ---- Time capsules ----

export interface Capsule {
  id: number
  relationship_id: number
  title: string
  status: CapsuleStatus
  unlock_at: Timestamp
  /** User id of the creator. */
  created_by: number
  created_at: Timestamp
  updated_at: Timestamp
  media_count: number
  /** Only present when `status` is `OPENED`. */
  message?: string | null
  /** Only present when `status` is `OPENED`. */
  media?: Media[]
}

export interface CreateCapsuleInput {
  title: string
  message: string | null
  unlock_at: Timestamp
}

/** Partial update: omitted fields are kept; `message: null` clears it. */
export interface UpdateCapsuleInput {
  title?: string
  message?: string | null
  unlock_at?: Timestamp
}
