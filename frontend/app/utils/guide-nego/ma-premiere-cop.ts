/**
 * Le parcours « Ma première COP » sur le téléphone : l'avancement compté sur les
 * étapes publiées, où mène un lien « Lire : … », et quand dire que les coches ont
 * changé ailleurs (R7).
 */
import type { PathwayGroup, PathwayLink, PathwayStep } from '../../types/negotiation-savoir.ts'

export interface Avancement {
  faites: number
  total: number
  parGroupe: Record<string, { faites: number; total: number }>
  prochaine: { etape: PathwayStep; groupe: PathwayGroup } | null
}

/** Une coche d'étape retirée n'est pas dans `groupes` : elle ne compte pas. */
export function avancement(groupes: readonly PathwayGroup[], coches: ReadonlySet<string>): Avancement {
  const resultat: Avancement = { faites: 0, total: 0, parGroupe: {}, prochaine: null }
  for (const groupe of groupes) {
    const faites = groupe.steps.filter((s) => coches.has(s.id)).length
    resultat.parGroupe[groupe.id] = { faites, total: groupe.steps.length }
    resultat.faites += faites
    resultat.total += groupe.steps.length
    const libre = resultat.prochaine ? null : groupe.steps.find((s) => !coches.has(s.id))
    if (libre) resultat.prochaine = { etape: libre, groupe }
  }
  return resultat
}

/** Un document à sa page, sinon à sa section, sinon sa fiche ; un terme par son slug. */
export function destinationDeLEtape(lien: PathwayLink, slugDe: (id: string) => string | null): string | null {
  switch (lien.kind) {
    case 'document': {
      const base = `/guide-nego/ressources/documents/${lien.target_id}`
      if (lien.page) return `${base}/lire?page=${lien.page}`
      if (lien.section) return `${base}/lire?section=${encodeURIComponent(lien.section)}`
      return base
    }
    case 'faq':
      return `/guide-nego/ressources/faq/${lien.target_id}`
    case 'glossary': {
      const slug = slugDe(lien.target_id)
      return slug ? `/guide-nego/lexique/${slug}` : null
    }
  }
}

/**
 * La relecture du compte a-t-elle changé ce que l'écran montrait ? Les étapes encore
 * dans la file n'en disent rien : l'écran y montre déjà le geste de ce téléphone.
 */
export function changeesAilleurs(
  avant: readonly string[],
  apres: readonly string[],
  enFile: Iterable<string>,
  publiees: ReadonlySet<string> | null,
): boolean {
  const ignorees = new Set(enFile)
  const compte = (id: string) => !ignorees.has(id) && (!publiees || publiees.has(id))
  const a = new Set(avant.filter(compte))
  const b = new Set(apres.filter(compte))
  return a.size !== b.size || [...a].some((id) => !b.has(id))
}
