import { useOutletContext } from 'react-router'
import type { Relationship } from '../../api/types'

export interface RelationshipOutletContext {
  relationship: Relationship
}

export function useRelationshipContext(): RelationshipOutletContext {
  return useOutletContext<RelationshipOutletContext>()
}
