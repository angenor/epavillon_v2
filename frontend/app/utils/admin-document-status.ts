import type { AdminDocumentState, ExtractionStatus } from '~/types/admin-negotiation-documents'
import type { Intent } from '~/types/ui'

// Une seule table pour la liste, la fiche et l'aperçu : « en file » est jaune partout.

export const DOCUMENT_STATE_INTENT: Record<AdminDocumentState, Intent> = {
  draft: 'neutral',
  published: 'success',
  unpublished: 'neutral',
}

export const EXTRACTION_STATUS_INTENT: Record<ExtractionStatus, Intent> = {
  pending: 'warning',
  extracting: 'warning',
  ready: 'success',
  failed: 'danger',
}
