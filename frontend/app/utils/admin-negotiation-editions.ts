/**
 * Les COP climat qu'un écran d'import peut viser, et celle qu'il ouvre par
 * défaut : l'édition que sert Guide Négo, selon la règle unique d'`editionDuGuide`.
 */
import type { PublicEditionRow } from '~/types/views'
import type { SelectOption } from '~/types/ui'
import { editionDuGuide } from '~/utils/guide-nego/edition'

export interface EditionsDeCop {
  options: SelectOption[]
  parDefaut: string | null
}

export function editionsDeCop(editions: PublicEditionRow[], locale: string): EditionsDeCop {
  const cop = editions
    .filter((e) => e.series_kind === 'cop_climate')
    .sort((a, b) => b.starts_at.localeCompare(a.starts_at))
  const libelle = (e: PublicEditionRow) => e.edition_label ?? e.acronym ?? resolveI18nText(e.title, locale)
  const gardee = editionDuGuide(cop)?.libelle
  const parDefaut = cop.find((e) => (e.edition_label ?? e.acronym) === gardee) ?? cop[0]
  return {
    options: cop.map((e) => ({ value: e.slug, label: libelle(e) })),
    parDefaut: parDefaut?.slug ?? null,
  }
}
