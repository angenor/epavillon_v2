/**
 * Les quatre échéances d'une édition (frise de la page publique) : ouverture de
 * l'appel, clôture des dépôts, annonce des résultats, tenue de l'événement.
 *
 * L'ÉTAT DE CHAQUE ÉTAPE VIENT DES FAITS, PAS DU RANG DE LA PREMIÈRE DATE
 * FUTURE (corrigé le 02/11). L'ancienne règle rendait courante la première
 * étape non datée ou à venir, et toutes les suivantes « à venir » : une annonce
 * sans date bloquait la frise, et la CdP30, terminée, s'affichait à venir.
 *
 *   ouverture   franchie quand `opens_at` est passée ;
 *   clôture     franchie à l'échéance effective, ou dès que l'appel est clos
 *               (`closed`, `under_review`, `published`) ; en cours sinon ;
 *   résultats   annoncés si l'appel est `published` OU si le programme de
 *               l'édition est publié : publier une programmation, c'est avoir
 *               annoncé qui est retenu ; en cours dès la clôture ;
 *   événement   en cours entre ses deux dates, franchi après.
 *
 * Un fait vaut pour ce qui le précède : une étape atteinte coche toutes les
 * étapes antérieures, datées ou non. Un événement commencé a forcément vu ses
 * dépôts clos et ses résultats annoncés, quelle que soit la date saisie.
 */

import type { CallForProposals } from '~/types/event/call'
import type { EventEdition } from '~/types/event/edition'
import type { IsoDateTime } from '~/types/shared'
import type { StepState } from '~/types/ui'
import { effectiveDeadline } from './call.ts'

export type MilestoneKey = 'callOpens' | 'callCloses' | 'results' | 'event'

export interface Milestone {
  key: MilestoneKey
  at: IsoDateTime | null
  /** Date au jour, sans heure : `results_expected_at` est une colonne `date`. */
  dateOnly: boolean
  state: StepState
}

type MilestoneEdition = Pick<EventEdition, 'starts_at' | 'ends_at' | 'programme_published_at'>
type MilestoneCall = Pick<
  CallForProposals,
  'status' | 'opens_at' | 'closes_at' | 'extended_until' | 'results_expected_at'
>

const CLOSED_CALL_STATUSES: ReadonlyArray<CallForProposals['status']> = ['closed', 'under_review', 'published']

function isPast(at: string | null, now: number): boolean {
  if (!at) return false
  const instant = Date.parse(at)
  return Number.isFinite(instant) && instant <= now
}

export function editionMilestones(
  edition: MilestoneEdition,
  call: MilestoneCall | null,
  now: number,
): Milestone[] {
  const steps: Milestone[] = []
  const eventStarted = isPast(edition.starts_at, now)
  const eventEnded = isPast(edition.ends_at, now)

  if (call) {
    const cancelled = call.status === 'cancelled'
    const opened = isPast(call.opens_at, now)
    const closed =
      isPast(effectiveDeadline(call), now) ||
      (opened && CLOSED_CALL_STATUSES.includes(call.status))
    const announced = call.status === 'published' || isPast(edition.programme_published_at, now)

    steps.push({ key: 'callOpens', at: call.opens_at, dateOnly: false, state: opened ? 'done' : 'upcoming' })
    steps.push({
      key: 'callCloses',
      at: effectiveDeadline(call),
      dateOnly: false,
      state: cancelled ? 'skipped' : closed ? 'done' : opened ? 'current' : 'upcoming',
    })
    steps.push({
      key: 'results',
      // Midi UTC : aucune conversion de fuseau ne fait changer le jour saisi.
      at: call.results_expected_at ? `${call.results_expected_at}T12:00:00Z` : null,
      dateOnly: true,
      state: cancelled ? 'skipped' : announced ? 'done' : closed ? 'current' : 'upcoming',
    })
  }

  steps.push({
    key: 'event',
    at: edition.starts_at,
    dateOnly: false,
    state: eventEnded ? 'done' : eventStarted ? 'current' : 'upcoming',
  })

  const lastReached = steps.findLastIndex((step) => step.state === 'done' || step.state === 'current')
  return steps.map((step, index) =>
    index < lastReached && step.state !== 'skipped' ? { ...step, state: 'done' } : step,
  )
}
