import type { AdminKnowledgeStatus } from '~/types/admin-negotiation-savoir'
import type { Intent } from '~/types/ui'

// « À revoir » reste servi au téléphone : jaune, il demande attention sans être un échec.

export const KNOWLEDGE_STATUS_INTENT: Record<AdminKnowledgeStatus, Intent> = {
  draft: 'neutral',
  published: 'success',
  to_review: 'warning',
}

export const KNOWLEDGE_STATUSES: AdminKnowledgeStatus[] = ['draft', 'published', 'to_review']

/** « Paris » : les dates du savoir se lisent à l'heure de la vérification, celle de l'API. */
export const KNOWLEDGE_TIMEZONE = 'Europe/Paris'
