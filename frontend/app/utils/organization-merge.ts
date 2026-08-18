/**
 * L'ARITHMÉTIQUE DE LA FUSION (A11) — fonctions PURES.
 *
 * Rien ici n'appelle le réseau et rien n'affiche : ce fichier porte les règles
 * que l'écran de fusion applique entre deux frappes — quels champs restent à
 * trancher, quelle valeur sera écrite, combien de lignes se déplacent, et si le
 * nom saisi en confirmation est bien celui de la fiche absorbée.
 *
 * POURQUOI À PART. L'écran de fusion est le seul du back-office dont un clic
 * déplace des rattachements dans tous les modules à la fois. Ces règles doivent
 * pouvoir se relire d'une traite, sans traverser un composant : c'est le même
 * parti qu'`utils/review-scoring.ts`, qui rejoue `refresh_proposal_score()`.
 *
 * LE SENS DE LA FUSION N'EST PAS L'ORDRE DE LA PAIRE. `duplicate_candidates`
 * range `left_id` avant `right_id` par contrainte d'unicité
 * (`ck_duplicate_candidates_ordered`) : cet ordre est technique. Qui absorbe qui
 * est une décision humaine, et l'écran doit pouvoir l'inverser sans rien perdre
 * — d'où `suggestAbsorbingSide()`, qui propose sans imposer.
 */

import type {
  DuplicateSide,
  MergeField,
  MergeFieldComparison,
  MergeSideKey,
  MergeTransferLine,
} from '~/types/admin-organizations'

/**
 * Ordre d'affichage des champs comparés. Il suit ce qui IDENTIFIE une
 * organisation avant ce qui la joint : on tranche d'abord le nom et le sigle,
 * les coordonnées ensuite.
 */
export const MERGE_FIELDS: MergeField[] = [
  'legal_name',
  'acronym',
  'slug',
  'organization_type_code',
  'country_id',
  'city',
  'description',
  'website',
  'contact_email',
  'contact_phone',
]

/**
 * Choix par défaut, champ par champ.
 *
 * DEUX RÈGLES, ET UNE SEULE DEMANDE UN ARBITRAGE HUMAIN :
 *   · un seul côté renseigné → on prend celui-là, y compris quand c'est la fiche
 *     absorbée. Compléter la cible avec ce que la source apporte est tout
 *     l'intérêt de la manœuvre : la fiche en doublon porte souvent le téléphone
 *     ou le site que l'autre n'a jamais eus ;
 *   · les deux renseignés et différents → aucun choix par défaut. L'écran laisse
 *     le champ ouvert, et le décompte des arbitrages restants empêche de valider
 *     à l'aveugle.
 * Les champs identiques ne figurent dans aucun des deux cas : il n'y a rien à
 * choisir entre deux valeurs égales.
 */
export function defaultMergeChoices(
  comparisons: MergeFieldComparison[],
): Partial<Record<MergeField, MergeSideKey>> {
  const choices: Partial<Record<MergeField, MergeSideKey>> = {}

  for (const comparison of comparisons) {
    if (!comparison.differs) continue
    if (comparison.filled === 'source') choices[comparison.field] = 'source'
    else if (comparison.filled === 'target') choices[comparison.field] = 'target'
  }
  return choices
}

/** Champs réellement en conflit : les deux fiches portent une valeur, différente. */
export function contestedFields(comparisons: MergeFieldComparison[]): MergeFieldComparison[] {
  return comparisons.filter((comparison) => comparison.differs && comparison.filled === 'both')
}

/** Champs en conflit qu'aucun choix ne tranche encore. */
export function unresolvedFields(
  comparisons: MergeFieldComparison[],
  choices: Partial<Record<MergeField, MergeSideKey>>,
): MergeField[] {
  return contestedFields(comparisons)
    .filter((comparison) => choices[comparison.field] === undefined)
    .map((comparison) => comparison.field)
}

/**
 * Valeur qui sera écrite sur la fiche absorbante.
 *
 * SANS CHOIX, C'EST LA CIBLE QUI L'EMPORTE — et non la source, ni la valeur « la
 * plus remplie ». C'est elle qui survit à la fusion : ne rien décider ne doit
 * rien écraser. Le contrat de `MergePayload.field_choices` dit la même chose, et
 * l'API applique la même règle.
 */
export function resolvedMergeValue(
  comparison: MergeFieldComparison,
  choices: Partial<Record<MergeField, MergeSideKey>>,
): { value: unknown; label: unknown; side: MergeSideKey } {
  const side = choices[comparison.field] ?? 'target'
  return side === 'source'
    ? { value: comparison.source_value, label: comparison.source_label, side }
    : { value: comparison.target_value, label: comparison.target_label, side }
}

/** Champs dont la valeur retenue vient de la fiche absorbée : ce que la fusion apporte. */
export function fieldsTakenFromSource(
  comparisons: MergeFieldComparison[],
  choices: Partial<Record<MergeField, MergeSideKey>>,
): MergeField[] {
  return comparisons
    .filter((comparison) => comparison.differs && choices[comparison.field] === 'source')
    .map((comparison) => comparison.field)
}

/** Totaux du décompte de transfert, tous modules confondus. */
export function transferTotals(lines: MergeTransferLine[]): {
  reassigned: number
  deduped: number
  deleted: number
  touched: number
} {
  const totals = lines.reduce(
    (acc, line) => ({
      reassigned: acc.reassigned + line.reassigned,
      deduped: acc.deduped + line.deduped,
      deleted: acc.deleted + line.deleted,
    }),
    { reassigned: 0, deduped: 0, deleted: 0 },
  )

  return { ...totals, touched: totals.reassigned + totals.deduped + totals.deleted }
}

/** Lignes du registre qui déplacent ou suppriment quelque chose — les seules à montrer. */
export function significantTransfers(lines: MergeTransferLine[]): MergeTransferLine[] {
  return lines.filter((line) => line.reassigned + line.deduped + line.deleted > 0)
}

/**
 * Forme comparable d'un nom saisi : minuscules, sans accents ni ponctuation,
 * espaces réduits. C'est `platform.normalize_label()`, et la confirmation s'y
 * mesure — exiger l'accent exact d'« Observatoire du Sahel pour l'énergie
 * durable » transformerait un garde-fou en épreuve de dactylographie.
 */
function normalizeName(value: string): string {
  return value
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, ' ')
    .trim()
}

/**
 * Le nom saisi désigne-t-il bien la fiche ABSORBÉE ?
 *
 * C'est la confirmation exigée par le prompt, et elle porte sur la fiche qui
 * DISPARAÎT de la liste — pas sur celle qui survit. Recopier le nom de la fiche
 * absorbante serait le geste d'une personne qui a lu l'écran à l'envers, et c'est
 * exactement ce que ce contrôle doit attraper.
 *
 * Le sigle est accepté : « OSED » désigne la fiche aussi sûrement que son nom
 * légal — c'est la règle métier n° 1 du projet, et la refuser ici la
 * contredirait sur le seul écran qui existe pour la faire respecter.
 */
export function isMergeConfirmationValid(input: string, source: DuplicateSide): boolean {
  const typed = normalizeName(input)
  if (typed.length === 0) return false

  return (
    typed === normalizeName(source.legal_name) ||
    (source.acronym !== null && typed === normalizeName(source.acronym))
  )
}

/**
 * Laquelle des deux fiches devrait absorber l'autre ?
 *
 * PROPOSITION, JAMAIS DÉCISION : l'écran présente le sens suggéré et l'inverse
 * d'un clic. Trois signaux, dans cet ordre, et le premier qui départage tranche :
 *   1. le SCEAU de l'IFDD — une fiche vérifiée absorbe, elle n'est pas absorbée.
 *      Perdre un sceau à l'occasion d'une fusion est une erreur qu'on ne
 *      découvre que le jour où le public cherche l'organisation ;
 *   2. le score de confiance, qui résume domaine vérifié, complétude et membres ;
 *   3. l'ancienneté — à égalité, la fiche la plus ancienne est celle dont les
 *      identifiants ont le plus circulé.
 */
export function suggestAbsorbingSide(left: DuplicateSide, right: DuplicateSide): DuplicateSide {
  const leftVerified = left.verified_at !== null
  const rightVerified = right.verified_at !== null
  if (leftVerified !== rightVerified) return leftVerified ? left : right

  if (left.trust_score !== right.trust_score) {
    return left.trust_score > right.trust_score ? left : right
  }
  return left.created_at <= right.created_at ? left : right
}
