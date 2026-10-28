import type { CancellationImpact } from '~/types/admin-review'

/**
 * Plusieurs dossiers annulés ensemble : on additionne. Une personne inscrite à
 * deux d'entre eux compte deux fois, comme elle recevra deux avis.
 */
export function sumCancellationImpacts(impacts: CancellationImpact[]): CancellationImpact {
  return impacts.reduce(
    (total, impact) => ({
      published_sessions: total.published_sessions + impact.published_sessions,
      registrants: total.registrants + impact.registrants,
    }),
    { published_sessions: 0, registrants: 0 },
  )
}

/** Rien de publié : l'annulation ne change pas le programme, on n'en dit rien. */
export function touchesPublicProgramme(impact: CancellationImpact | null): impact is CancellationImpact {
  return impact !== null && impact.published_sessions > 0
}
