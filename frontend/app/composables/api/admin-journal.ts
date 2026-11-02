/**
 * JOURNAL DU BACK-OFFICE (`/admin/journal`) : sa part de `useApi()`.
 *
 * Toutes les routes exigent `identity.platform.journal` en portée globale ; les
 * deux relances exigent en plus `identity.platform.maintenance`. L'écran ne fait
 * que décider d'afficher le bouton : le refus, s'il vient, est celui de l'API.
 *
 * L'export CSV n'est pas ici : un fichier se télécharge par un lien
 * (`useApiUrl`), pas par un appel JSON.
 */

import type {
  JournalChangeFilters,
  JournalChangePage,
  JournalEntryFilters,
  JournalEntryPage,
  JournalFailures,
  JournalRequestTrace,
  JournalSummary,
} from '~/types/admin-journal'
import type { Uuid } from '~/types/shared'
import { journalQuery } from '~/utils/admin-journal'
import type { ApiTransport } from './proposal-review'

type JournalApiDeps = Pick<ApiTransport, 'call' | 'send'>

export function createAdminJournalApi({ call, send }: JournalApiDeps) {
  return {
    summary: (): Promise<JournalSummary> =>
      call('/admin/journal/summary', (m) => m.journalSummary()),

    entries: (filters: JournalEntryFilters): Promise<JournalEntryPage> =>
      call('/admin/journal/entries', (m) => m.journalEntryPage(filters), journalQuery(filters)),

    /** Listes vides, et non 404, quand rien ne porte ce code. */
    trace: (requestId: string): Promise<JournalRequestTrace> =>
      call(`/admin/journal/requests/${encodeURIComponent(requestId)}`, (m) => m.journalTrace(requestId)),

    changes: (filters: JournalChangeFilters): Promise<JournalChangePage> =>
      call('/admin/journal/changes', (m) => m.journalChangePage(filters), journalQuery(filters)),

    failures: (): Promise<JournalFailures> =>
      call('/admin/journal/failures', (m) => m.journalFailures()),

    /** 409 si le travail n'est plus en échec : le message de l'API le dit. */
    retryJob: (jobId: Uuid): Promise<void> =>
      send(`/admin/journal/jobs/${jobId}/retry`, {}, (m) => m.retryJournalJob(jobId), 'POST'),

    /** 409 si l'événement a déjà été relayé. */
    retryEvent: (eventId: Uuid): Promise<void> =>
      send(`/admin/journal/events/${eventId}/retry`, {}, (m) => m.retryJournalEvent(eventId), 'POST'),
  }
}
