/**
 * Ce qui a changé dans le programme publié d'une édition : sa part de `useApi()`.
 * Route publique, sans session, comme `/schedule`.
 */
import type { ProgrammeChange } from '~/types/programme/changes'
import type { EventId } from '~/types/shared'
import type { ApiTransport } from './proposal-review'

const exemples = () => import('~/mocks/programme-changes')

export function createProgrammeChangesApi({ call }: Pick<ApiTransport, 'call'>) {
  return {
    /** Les quinze derniers jours, le plus récent d'abord ; vide si le programme n'est pas paru. */
    forEdition: (eventId: EventId) =>
      call<ProgrammeChange[]>(`/events/${eventId}/programme-changes`, async () =>
        (await exemples()).programmeChanges(eventId),
      ),
  }
}
