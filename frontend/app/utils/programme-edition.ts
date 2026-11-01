import type { EditionTemporalState, PublicEditionRow } from '~/types/views'
import type { AttachedImage } from '~/types/media'

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

/**
 * La photo d'une édition, 16:9 d'abord : celle de son bandeau, et la couverture
 * par défaut de ses activités sans image (`ActivityCoverFallback`).
 */
export function editionPhoto(edition: Pick<PublicEditionRow, 'cover' | 'banner'> | null | undefined): AttachedImage | null {
  return edition ? (edition.cover ?? edition.banner ?? null) : null
}
