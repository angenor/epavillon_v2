import type { EditionTemporalState, PublicEditionRow } from '~/types/views'

const RANK: Record<EditionTemporalState, number> = { ongoing: 0, upcoming: 1, past: 2 }

/**
 * L'édition qu'ouvre `/programmations` sans paramètre : celle en cours, sinon
 * la plus proche à venir, sinon la dernière passée. L'état temporel vient de
 * l'API — l'horloge du serveur, pas celle du visiteur.
 */
export function defaultProgrammeEdition(
  editions: readonly PublicEditionRow[],
): PublicEditionRow | undefined {
  return [...editions].sort((a, b) => {
    if (a.temporal_state !== b.temporal_state) return RANK[a.temporal_state] - RANK[b.temporal_state]
    return a.temporal_state === 'upcoming'
      ? a.starts_at.localeCompare(b.starts_at)
      : b.starts_at.localeCompare(a.starts_at)
  })[0]
}
