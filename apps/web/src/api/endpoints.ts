// One function per documented endpoint (docs/api/*.md).
import { request, upload, type Query } from './client'
import type {
  Capsule,
  CapsuleStatus,
  ChangePasswordInput,
  CreateCapsuleInput,
  CreateProfileInput,
  CreateProfileResult,
  CreateRelationshipInput,
  DevelopmentQuery,
  DevelopmentRecord,
  DevelopmentRecordInput,
  LoginInput,
  Measurement,
  MeasurementInput,
  MeasurementQuery,
  MeasurementSeries,
  MeasurementType,
  Media,
  Member,
  Memory,
  MemoryInput,
  Profile,
  Reference,
  RegisterInput,
  Relationship,
  Role,
  Session,
  Tag,
  TimelinePage,
  TimelineQuery,
  UpdateCapsuleInput,
  UpdateMeInput,
  UpdateProfileInput,
  UpdateRelationshipInput,
  User,
} from './types'

type Progress = (fraction: number) => void

export const api = {
  // Reference data
  getReference: () => request<Reference>('/api/reference'),

  // Auth & users
  register: (input: RegisterInput) => request<Session>('/api/auth/register', { method: 'POST', body: input }),
  login: (input: LoginInput) => request<Session>('/api/auth/login', { method: 'POST', body: input }),
  logout: () => request<void>('/api/auth/logout', { method: 'POST' }),
  getMe: () => request<User>('/api/users/me'),
  updateMe: (input: UpdateMeInput) => request<User>('/api/users/me', { method: 'PUT', body: input }),
  changePassword: (input: ChangePasswordInput) =>
    request<void>('/api/users/me/password', { method: 'PUT', body: input }),

  // Profiles
  listProfiles: () => request<Profile[]>('/api/profiles'),
  createProfile: (input: CreateProfileInput) =>
    request<CreateProfileResult>('/api/profiles', { method: 'POST', body: input }),
  getProfile: (id: number) => request<Profile>(`/api/profiles/${id}`),
  updateProfile: (id: number, input: UpdateProfileInput) =>
    request<Profile>(`/api/profiles/${id}`, { method: 'PUT', body: input }),
  deleteProfile: (id: number) => request<void>(`/api/profiles/${id}`, { method: 'DELETE' }),

  // Relationships
  listRelationships: () => request<Relationship[]>('/api/relationships'),
  createRelationship: (input: CreateRelationshipInput) =>
    request<Relationship>('/api/relationships', { method: 'POST', body: input }),
  getRelationship: (id: number) => request<Relationship>(`/api/relationships/${id}`),
  updateRelationship: (id: number, input: UpdateRelationshipInput) =>
    request<Relationship>(`/api/relationships/${id}`, { method: 'PUT', body: input }),
  deleteRelationship: (id: number) => request<void>(`/api/relationships/${id}`, { method: 'DELETE' }),

  // Members
  listMembers: (relationshipId: number) => request<Member[]>(`/api/relationships/${relationshipId}/members`),
  addMember: (relationshipId: number, input: { email: string; role: Role }) =>
    request<Member[]>(`/api/relationships/${relationshipId}/members`, { method: 'POST', body: input }),
  updateMemberRole: (relationshipId: number, userId: number, role: Role) =>
    request<Member[]>(`/api/relationships/${relationshipId}/members/${userId}`, { method: 'PUT', body: { role } }),
  removeMember: (relationshipId: number, userId: number) =>
    request<void>(`/api/relationships/${relationshipId}/members/${userId}`, { method: 'DELETE' }),

  // Memories & timeline
  listMemories: (relationshipId: number, query: TimelineQuery = {}) =>
    request<TimelinePage>(`/api/relationships/${relationshipId}/memories`, { query: { ...query } }),
  createMemory: (relationshipId: number, input: MemoryInput) =>
    request<Memory>(`/api/relationships/${relationshipId}/memories`, { method: 'POST', body: input }),
  getMemory: (id: number) => request<Memory>(`/api/memories/${id}`),
  updateMemory: (id: number, input: MemoryInput) =>
    request<Memory>(`/api/memories/${id}`, { method: 'PUT', body: input }),
  deleteMemory: (id: number) => request<void>(`/api/memories/${id}`, { method: 'DELETE' }),

  // Tags
  listTags: (query: { q?: string; limit?: number } = {}) => request<Tag[]>('/api/tags', { query }),
  listRelationshipTags: (relationshipId: number, query: { q?: string; limit?: number } = {}) =>
    request<Tag[]>(`/api/relationships/${relationshipId}/tags`, { query }),
  createTag: (name: string) => request<Tag>('/api/tags', { method: 'POST', body: { name } }),

  // Media
  uploadMemoryMedia: (memoryId: number, files: readonly File[], onProgress?: Progress) =>
    upload<Media[]>(`/api/memories/${memoryId}/media`, files, onProgress),
  removeMemoryMedia: (memoryId: number, mediaId: number) =>
    request<void>(`/api/memories/${memoryId}/media/${mediaId}`, { method: 'DELETE' }),
  uploadCapsuleMedia: (capsuleId: number, files: readonly File[], onProgress?: Progress) =>
    upload<Media[]>(`/api/capsules/${capsuleId}/media`, files, onProgress),
  removeCapsuleMedia: (capsuleId: number, mediaId: number) =>
    request<void>(`/api/capsules/${capsuleId}/media/${mediaId}`, { method: 'DELETE' }),
  getMedia: (id: number) => request<Media>(`/api/media/${id}`),

  // Development records
  listDevelopmentRecords: (profileId: number, query: DevelopmentQuery = {}) =>
    request<DevelopmentRecord[]>(`/api/profiles/${profileId}/development-records`, { query: { ...query } }),
  createDevelopmentRecord: (profileId: number, input: DevelopmentRecordInput) =>
    request<DevelopmentRecord>(`/api/profiles/${profileId}/development-records`, { method: 'POST', body: input }),
  getDevelopmentRecord: (id: number) => request<DevelopmentRecord>(`/api/development-records/${id}`),
  updateDevelopmentRecord: (id: number, input: DevelopmentRecordInput) =>
    request<DevelopmentRecord>(`/api/development-records/${id}`, { method: 'PUT', body: input }),
  deleteDevelopmentRecord: (id: number) => request<void>(`/api/development-records/${id}`, { method: 'DELETE' }),

  // Measurements
  listMeasurements: (profileId: number, query: MeasurementQuery = {}) =>
    request<Measurement[]>(`/api/profiles/${profileId}/measurements`, { query: { ...query } }),
  createMeasurement: (profileId: number, input: MeasurementInput) =>
    request<Measurement>(`/api/profiles/${profileId}/measurements`, { method: 'POST', body: input }),
  getMeasurement: (id: number) => request<Measurement>(`/api/measurements/${id}`),
  updateMeasurement: (id: number, input: MeasurementInput) =>
    request<Measurement>(`/api/measurements/${id}`, { method: 'PUT', body: input }),
  deleteMeasurement: (id: number) => request<void>(`/api/measurements/${id}`, { method: 'DELETE' }),
  /** All series (one per type with data). */
  listMeasurementSeries: (profileId: number, query: Omit<MeasurementQuery, 'type'> = {}) =>
    request<MeasurementSeries[]>(`/api/profiles/${profileId}/measurements/series`, { query: { ...query } }),
  /** A single series for one type. */
  getMeasurementSeries: (profileId: number, type: MeasurementType, query: Omit<MeasurementQuery, 'type'> = {}) =>
    request<MeasurementSeries>(`/api/profiles/${profileId}/measurements/series`, {
      query: { ...query, type } satisfies Query,
    }),

  // Time capsules
  listCapsules: (relationshipId: number, status?: CapsuleStatus) =>
    request<Capsule[]>(`/api/relationships/${relationshipId}/capsules`, { query: { status } }),
  createCapsule: (relationshipId: number, input: CreateCapsuleInput) =>
    request<Capsule>(`/api/relationships/${relationshipId}/capsules`, { method: 'POST', body: input }),
  getCapsule: (id: number) => request<Capsule>(`/api/capsules/${id}`),
  updateCapsule: (id: number, input: UpdateCapsuleInput) =>
    request<Capsule>(`/api/capsules/${id}`, { method: 'PUT', body: input }),
  cancelCapsule: (id: number) => request<Capsule>(`/api/capsules/${id}/cancel`, { method: 'POST' }),
  openCapsule: (id: number) => request<Capsule>(`/api/capsules/${id}/open`, { method: 'POST' }),
  deleteCapsule: (id: number) => request<void>(`/api/capsules/${id}`, { method: 'DELETE' }),
}
