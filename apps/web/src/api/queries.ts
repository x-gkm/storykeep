// TanStack Query keys and hooks for the reads that several screens share.
import { QueryClient, useQuery } from '@tanstack/react-query'
import { isApiError } from './client'
import { api } from './endpoints'
import type { DevelopmentQuery, TimelineQuery } from './types'

export const keys = {
  me: ['me'] as const,
  profiles: ['profiles'] as const,
  profile: (id: number) => ['profiles', id] as const,
  relationships: ['relationships'] as const,
  relationship: (id: number) => ['relationships', id] as const,
  members: (relationshipId: number) => ['relationships', relationshipId, 'members'] as const,
  timeline: (relationshipId: number, query: TimelineQuery) => ['relationships', relationshipId, 'memories', query] as const,
  timelineAll: (relationshipId: number) => ['relationships', relationshipId, 'memories'] as const,
  relationshipTags: (relationshipId: number) => ['relationships', relationshipId, 'tags'] as const,
  capsules: (relationshipId: number) => ['relationships', relationshipId, 'capsules'] as const,
  memory: (id: number) => ['memories', id] as const,
  capsule: (id: number) => ['capsules', id] as const,
  development: (profileId: number, query: DevelopmentQuery) => ['profiles', profileId, 'development', query] as const,
  developmentAll: (profileId: number) => ['profiles', profileId, 'development'] as const,
  measurements: (profileId: number) => ['profiles', profileId, 'measurements'] as const,
  series: (profileId: number) => ['profiles', profileId, 'measurements', 'series'] as const,
}

export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 30_000,
        refetchOnWindowFocus: false,
        // Client errors (403/404/…) won't fix themselves; only retry network/server failures.
        retry: (count, error) => count < 2 && (!isApiError(error) || error.status === 0 || error.status >= 500),
      },
      mutations: { retry: false },
    },
  })
}

export const useRelationships = () => useQuery({ queryKey: keys.relationships, queryFn: api.listRelationships })
export const useProfiles = () => useQuery({ queryKey: keys.profiles, queryFn: api.listProfiles })
export const useRelationship = (id: number | null) =>
  useQuery({ queryKey: keys.relationship(id ?? 0), queryFn: () => api.getRelationship(id ?? 0), enabled: id !== null })
export const useProfile = (id: number | null) =>
  useQuery({ queryKey: keys.profile(id ?? 0), queryFn: () => api.getProfile(id ?? 0), enabled: id !== null })
export const useMembers = (relationshipId: number) =>
  useQuery({ queryKey: keys.members(relationshipId), queryFn: () => api.listMembers(relationshipId) })
export const useRelationshipTags = (relationshipId: number) =>
  useQuery({
    queryKey: keys.relationshipTags(relationshipId),
    queryFn: () => api.listRelationshipTags(relationshipId, { limit: 200 }),
  })
export const useMemory = (id: number | null) =>
  useQuery({ queryKey: keys.memory(id ?? 0), queryFn: () => api.getMemory(id ?? 0), enabled: id !== null })
export const useCapsule = (id: number | null) =>
  useQuery({ queryKey: keys.capsule(id ?? 0), queryFn: () => api.getCapsule(id ?? 0), enabled: id !== null })
