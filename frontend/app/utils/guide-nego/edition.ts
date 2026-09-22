/**
 * L'édition que Guide Négo accompagne — **celle qui se tient, sinon la
 * prochaine**, parmi les conférences climat.
 *
 * Aucune vue ne désigne une « édition en cours » unique (R11) : la règle est
 * écrite ici, et nulle part ailleurs. La série dit ce qu'est une COP climat —
 * jamais une liste de libellés recopiée dans un composant.
 */
import type { PublicEditionRow } from '~/types/views'

export interface EditionGardee {
  libelle: string
  enCours: boolean
}

export function editionDuGuide(
  editions: Pick<PublicEditionRow, 'series_kind' | 'temporal_state' | 'starts_at' | 'edition_label' | 'acronym'>[],
): EditionGardee | null {
  const retenue = editions
    .filter((e) => e.series_kind === 'cop_climate' && e.temporal_state !== 'past')
    .sort((a, b) => {
      if (a.temporal_state !== b.temporal_state) return a.temporal_state === 'ongoing' ? -1 : 1
      return a.starts_at.localeCompare(b.starts_at)
    })[0]
  const libelle = retenue?.edition_label ?? retenue?.acronym
  return retenue && libelle ? { libelle, enCours: retenue.temporal_state === 'ongoing' } : null
}
