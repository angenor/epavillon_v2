/**
 * Les thématiques de négociation : **le vocabulaire, et ce que la personne suit**.
 *
 * Tout est pur ici, comme pour l'accès : les règles qui comptent — au moins une
 * thématique, le récapitulatif, l'écran proposé une seule fois — se trompent
 * silencieusement, et un test doit pouvoir les prendre sans navigateur.
 *
 * **Les libellés viennent du vocabulaire et de nulle part ailleurs** : le corps
 * des suivis ne porte que des codes. Le libellé se résout dans la langue de la
 * personne, avec repli sur le français (FR-002) — et le code en dernier
 * recours, qui vaut mieux qu'une ligne vide.
 */
import type { MyThemes } from '~/types/negotiation'
import type { TaxonomyTerm } from '~/types/reference'
import type { I18nText } from '~/types/shared'

/** Le vocabulaire des filières de négociation. **Pas** celui du Pavillon. */
export const VOCABULAIRE_THEMATIQUES = 'negotiation_theme'

/** La clé de la file : une intention de thématiques, jamais deux. */
export const CLE_FILE_THEMATIQUES = 'mes-thematiques'

/** Ce qui se garde d'un terme : le code et ses libellés, rien d'administratif. */
export interface ThematiqueGardee {
  code: string
  label: I18nText
  sort_order: number
}

/**
 * Ce qui se garde des suivis : les codes, et l'empreinte de l'état lu.
 *
 * **L'empreinte fait partie de la valeur gardée**, et c'est délibéré : un choix
 * pris hors connexion la renvoie en `If-Match`, y compris après un
 * redémarrage du téléphone. Gardée ailleurs, elle se perdrait à la fermeture de
 * l'application, et le choix repartirait sans garde d'ancienneté.
 */
export interface EtatDesThematiques {
  codes: string[]
  /** Les suivies dont on est prévenu (3b) ; absent d'une garde d'avant. */
  notify?: string[]
  empreinte: string | null
}

export const AUCUNE_THEMATIQUE: EtatDesThematiques = { codes: [], notify: [], empreinte: null }

export function thematiquesGardees(termes: TaxonomyTerm[]): ThematiqueGardee[] {
  return termes
    .filter((t) => t.is_active)
    .sort((a, b) => a.sort_order - b.sort_order || a.code.localeCompare(b.code))
    .map((t) => ({ code: t.code, label: { ...t.label }, sort_order: t.sort_order }))
}

/** Le libellé dans la langue demandée, en français à défaut, le code sinon. */
export function libelleDe(thematique: ThematiqueGardee, locale: string): string {
  return thematique.label[locale] ?? thematique.label.fr ?? thematique.code
}

/** Les codes suivis, triés — l'ordre du vocabulaire n'est pas celui du serveur. */
export function etatDesThematiques(mes: MyThemes | null, empreinte: string | null): EtatDesThematiques {
  return {
    codes: [...new Set((mes?.themes ?? []).map((t) => t.code))].sort(),
    notify: [...new Set(mes?.notify ?? [])].sort(),
    empreinte,
  }
}

/** Réduit une sélection d'écran à ce que le `PUT` porte : des codes, triés, uniques. */
export function codesVoulus(codes: Iterable<string>): string[] {
  return [...new Set([...codes].map((c) => c.trim()).filter(Boolean))].sort()
}

/**
 * **Au moins une thématique** (FR-005). La validation se refuse à zéro, à la
 * première entrée comme à la modification — la maquette écrit « Une ou
 * plusieurs » et ne dessine pas le cas de zéro.
 */
export function choixValidable(codes: string[]): boolean {
  return codes.length > 0
}

/**
 * Le récapitulatif du pied : combien, et lesquelles. Les noms sont rendus dans
 * l'ordre du vocabulaire, pas dans celui où les cases ont été touchées — deux
 * personnes ayant le même choix lisent la même phrase.
 */
export function recapitulatif(
  codes: string[],
  vocabulaire: ThematiqueGardee[],
  locale: string,
): { nombre: number; noms: string[] } {
  const choisis = new Set(codes)
  return {
    nombre: choisis.size,
    noms: vocabulaire.filter((t) => choisis.has(t.code)).map((t) => libelleDe(t, locale)),
  }
}

/**
 * **L'écran de premier choix se propose une fois, et ne retient personne.**
 *
 * Il faut une personne connectée, un état lu — sinon on proposerait l'écran à
 * qui suit déjà des thématiques, le temps d'une lecture —, aucune thématique
 * suivie, et la proposition pas déjà faite sur cet appareil. La clé se pose au
 * moment de proposer : refuser, c'est aussi une réponse, et la redemander à
 * chaque ouverture enfermerait.
 */
export function propositionAFaire(quoi: {
  connectee: boolean
  pret: boolean
  nombreSuivi: number
  dejaProposee: boolean
}): boolean {
  return quoi.connectee && quoi.pret && quoi.nombreSuivi === 0 && !quoi.dejaProposee
}
