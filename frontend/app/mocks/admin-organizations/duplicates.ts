/**
 * LA FILE DES DOUBLONS PRÉSUMÉS ET L'APERÇU DE FUSION (A11).
 *
 * Deux lectures, et la seconde est le cœur de l'écran le plus délicat du jalon.
 *
 * ── CE QUE LA FILE MONTRE ───────────────────────────────────────────────────
 *
 * `org.duplicate_candidates`, tel que le worker le remplit : une paire, un
 * score, des motifs. L'ordre est celui de l'index partiel
 * `ix_duplicate_candidates_pending (score DESC) WHERE reviewed_at IS NULL` — la
 * paire la plus ressemblante d'abord, ce que demande le prompt.
 *
 * ── CE QUE L'APERÇU CALCULE, ET POURQUOI IL NE PEUT PAS ÊTRE ÉCRIT À LA MAIN ─
 *
 * Le décompte de transfert se construit en parcourant
 * `org.organization_references` — le registre —, table par table, exactement
 * comme le fait la boucle `FOR v_ref IN SELECT * FROM org.organization_references`
 * de `merge_organizations()`. Trois sorts, jamais confondus :
 *   · `reassign` sans `dedupe_on` → toutes les lignes basculent ;
 *   · `reassign` avec `dedupe_on` → les lignes dont la valeur existe déjà côté
 *     cible sont SUPPRIMÉES avant la bascule (la même personne adhérente des
 *     deux fiches), le reste bascule ;
 *   · `delete` → la ligne disparaît.
 * Un chiffre unique « 14 éléments transférés » mentirait sur les trois.
 *
 * LE DÉCOMPTE N'EST PAS SYMÉTRIQUE. Fusionner A dans B ne déplace pas le même
 * nombre de lignes que B dans A : ce qui est dédoublonné dépend de ce que la
 * CIBLE porte déjà. C'est pourquoi l'aperçu se recalcule à chaque inversion du
 * sens, et pourquoi l'écran ne mémorise pas un décompte.
 *
 * CE QUE CE FICHIER NE SAIT PAS COMPTER, ET LE DIT : les tables des modules hors
 * jalon — `media`, `publication`, `training` — n'ont pas de données simulées.
 * Leurs lignes de registre sont donc à zéro, ce qui est exact pour ce jeu de
 * données et le resterait pour une plateforme neuve. Le jour où ces modules
 * vivent, le même code les comptera sans modification : c'est tout l'intérêt de
 * lire le registre plutôt que d'énumérer des tables.
 */

import type {
  DuplicatePair,
  DuplicateQueueScreen,
  DuplicateReason,
  DuplicateSide,
  MergeField,
  MergeFieldComparison,
  MergePreview,
  MergeTransferLine,
  MergeWarning,
} from '~/types/admin-organizations'
import type { I18nText } from '~/types/shared'
import { duplicateCandidates, organizationReferences } from '../org'
import { people } from '../people'
import { allProposals, proposalOrganizations, proposalSpeakers } from '../proposals'
import { allSessions, sessionOrganizations } from '../sessions'
import { registrations } from '../registrations'
import { eventSeries } from '../event'
import { incidents } from '../incidents'
import { countryOf, duplicateSideOf, organizationTypeTerm, personName } from './core'
import {
  effectiveDomains,
  effectiveMemberships,
  effectiveNames,
  effectiveOrganizations,
  mergedOrganizationIds,
  resolveOrganizationId,
  settledDecisionOf,
} from './session'

// ---------------------------------------------------------------------------
// La file
// ---------------------------------------------------------------------------

/** Une paire, ses deux fiches résolues et son arbitrage éventuel. */
function pairOf(candidate: (typeof duplicateCandidates)[number]): DuplicatePair | null {
  const left = duplicateSideOf(candidate.left_id)
  const right = duplicateSideOf(candidate.right_id)
  if (!left || !right) return null

  // Une décision prise pendant la session de démonstration prime sur celle du
  // jeu de données : c'est le journal d'écritures, décrit dans `writes.ts`.
  const settled = settledDecisionOf(candidate.id)

  return {
    id: candidate.id,
    score: candidate.score,
    reasons: candidate.reasons as DuplicateReason[],
    detected_at: candidate.detected_at,
    reviewed_at: settled?.reviewed_at ?? candidate.reviewed_at,
    reviewed_by: settled?.reviewed_by ?? candidate.reviewed_by,
    reviewed_by_name: personName(settled?.reviewed_by ?? candidate.reviewed_by),
    decision: settled?.decision ?? candidate.decision,
    left,
    right,
  }
}

/**
 * LA FILE, ET CE QUI EN EST SORTI.
 *
 * Les paires arbitrées restent accessibles : « ce ne sont pas des doublons » est
 * une décision humaine, et une décision humaine se reprend. Les ranger hors de
 * la file active plutôt que les effacer, c'est la différence entre un outil de
 * travail et un bouton qui fait disparaître les choses.
 */
export function duplicateQueue(): DuplicateQueueScreen {
  const pairs = duplicateCandidates
    .map(pairOf)
    .filter((pair): pair is DuplicatePair => pair !== null)

  return {
    pending: pairs
      .filter((pair) => pair.reviewed_at === null)
      .sort((a, b) => b.score - a.score),
    settled: pairs
      .filter((pair) => pair.reviewed_at !== null)
      .sort((a, b) => (b.reviewed_at ?? '').localeCompare(a.reviewed_at ?? '')),
  }
}

/** Paires non arbitrées où une fiche apparaît — le lien de la liste vers la file. */
export function pendingDuplicatesOf(organizationId: string): DuplicatePair[] {
  return duplicateQueue().pending.filter(
    (pair) => pair.left.organization_id === organizationId || pair.right.organization_id === organizationId,
  )
}

// ---------------------------------------------------------------------------
// La comparaison champ par champ
// ---------------------------------------------------------------------------

/** Les champs comparés — colonnes saisies par un humain, pas les colonnes générées. */
const COMPARED_FIELDS: MergeField[] = [
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

/** Valeur absente : nulle, chaîne blanche, ou document multilingue sans traduction. */
function isEmpty(value: unknown): boolean {
  if (value === null || value === undefined) return true
  if (typeof value === 'string') return value.trim().length === 0
  if (typeof value === 'object') return Object.keys(value as object).length === 0
  return false
}

/**
 * Les deux valeurs sont-elles la même chose ? Un `i18n_text` se compare
 * traduction par traduction : deux descriptions dont seule la version anglaise
 * diffère SONT différentes, et l'écran doit le montrer.
 */
function sameValue(a: unknown, b: unknown): boolean {
  if (a === b) return true
  if (isEmpty(a) && isEmpty(b)) return true
  if (typeof a === 'object' && typeof b === 'object' && a !== null && b !== null) {
    return JSON.stringify(a) === JSON.stringify(b)
  }
  return false
}

/** Libellé lisible d'un code : type d'organisation, pays. Nul ailleurs. */
function labelFor(field: MergeField, value: unknown): I18nText | string | null {
  if (value === null || value === undefined) return null
  if (field === 'organization_type_code') return organizationTypeTerm(String(value)).label
  if (field === 'country_id') return countryOf(String(value)).name
  return null
}

function comparisonsOf(sourceId: string, targetId: string): MergeFieldComparison[] {
  const fiches = effectiveOrganizations()
  const source = fiches.find((o) => o.id === sourceId)
  const target = fiches.find((o) => o.id === targetId)
  if (!source || !target) return []

  return COMPARED_FIELDS.map((field) => {
    const sourceValue = source[field] ?? null
    const targetValue = target[field] ?? null
    const sourceFilled = !isEmpty(sourceValue)
    const targetFilled = !isEmpty(targetValue)

    return {
      field,
      source_value: sourceValue,
      target_value: targetValue,
      differs: !sameValue(sourceValue, targetValue),
      filled:
        sourceFilled && targetFilled
          ? 'both'
          : sourceFilled
            ? 'source'
            : targetFilled
              ? 'target'
              : 'none',
      source_label: labelFor(field, sourceValue),
      target_label: labelFor(field, targetValue),
    }
  })
}

// ---------------------------------------------------------------------------
// Le décompte de transfert, lu dans le registre
// ---------------------------------------------------------------------------

/**
 * Les lignes d'une table qui pointent vers une organisation, telles que ce jeu
 * de données les connaît.
 *
 * La clé est `schéma.table.colonne`, exactement celle du registre et celle que
 * `merge_log.rows_reassigned` emploie. Chaque entrée rend, pour une organisation,
 * les lignes concernées et leur valeur de dédoublonnage éventuelle — la colonne
 * qui forme une unicité AVEC l'organisation.
 */
/** La ligne appartient-elle à cette fiche, redirection de fusion comprise ? */
function belongsTo(referenced: string, organizationId: string): boolean {
  return resolveOrganizationId(referenced) === organizationId
}

const REFERENCE_ROWS: Record<string, (organizationId: string) => { key: string | null }[]> = {
  'org.organization_names.organization_id': (id) =>
    effectiveNames().filter((n) => n.organization_id === id).map(() => ({ key: null })),
  'org.organization_domains.organization_id': (id) =>
    effectiveDomains()
      .filter((d) => d.organization_id === id)
      .map((d) => ({ key: d.domain })),
  'org.memberships.organization_id': (id) =>
    effectiveMemberships().filter((m) => m.organization_id === id).map((m) => ({ key: m.person_id })),
  'identity.people.primary_organization_id': (id) =>
    people
      .filter((p) => p.primary_organization_id !== null && belongsTo(p.primary_organization_id, id))
      .map(() => ({ key: null })),
  'event.event_series.organizer_organization_id': (id) =>
    eventSeries
      .filter((s) => s.organizer_organization_id !== null && belongsTo(s.organizer_organization_id, id))
      .map(() => ({ key: null })),
  'programme.proposals.organization_id': (id) =>
    allProposals.filter((p) => belongsTo(p.organization_id, id)).map(() => ({ key: null })),
  'programme.proposal_speakers.organization_id': (id) =>
    proposalSpeakers
      .filter((s) => s.organization_id !== null && belongsTo(s.organization_id, id))
      .map(() => ({ key: null })),
  'programme.proposal_organizations.organization_id': (id) =>
    proposalOrganizations
      .filter((l) => belongsTo(l.organization_id, id))
      .map((l) => ({ key: l.proposal_id })),
  'programme.sessions.organization_id': (id) =>
    allSessions
      .filter((s) => s.organization_id !== null && belongsTo(s.organization_id, id))
      .map(() => ({ key: null })),
  'programme.registrations.organization_id': (id) =>
    registrations
      .filter((r) => r.organization_id !== null && belongsTo(r.organization_id, id))
      .map(() => ({ key: null })),
  'programme.session_organizations.organization_id': (id) =>
    sessionOrganizations
      .filter((l) => belongsTo(l.organization_id, id))
      .map((l) => ({ key: l.session_id })),
  'live.incidents.organization_id': (id) =>
    incidents
      .filter((i) => i.organization_id !== null && belongsTo(i.organization_id, id))
      .map(() => ({ key: null })),
}

/** Clé de registre d'une ligne — la même chaîne que `merge_log.rows_reassigned`. */
export function referenceKey(reference: {
  ref_schema: string
  ref_table: string
  ref_column: string
}): string {
  return `${reference.ref_schema}.${reference.ref_table}.${reference.ref_column}`
}

/**
 * LE DÉCOMPTE, ligne de registre par ligne de registre.
 *
 * C'est la traduction fidèle de la boucle de `merge_organizations()` : pour
 * chaque référence déclarée, on compte ce qui bascule, ce qui saute parce que la
 * cible le porte déjà, et ce que la stratégie `delete` emporte. Les tables sans
 * données simulées rendent zéro partout, ce qui est le comportement juste — pas
 * une omission.
 */
export function transfersFor(sourceId: string, targetId: string): MergeTransferLine[] {
  return organizationReferences.map((reference) => {
    const key = referenceKey(reference)
    const rowsOf = REFERENCE_ROWS[key]
    const sourceRows = rowsOf ? rowsOf(sourceId) : []
    const targetRows = rowsOf ? rowsOf(targetId) : []

    if (reference.strategy === 'delete') {
      return {
        ...reference,
        strategy: reference.strategy,
        dedupe_on: [...reference.dedupe_on],
        reassigned: 0,
        deduped: 0,
        deleted: sourceRows.length,
      }
    }

    if (reference.dedupe_on.length === 0) {
      return {
        ...reference,
        strategy: reference.strategy,
        dedupe_on: [],
        reassigned: sourceRows.length,
        deduped: 0,
        deleted: 0,
      }
    }

    // `dedupe_on` : les lignes de la source dont la valeur existe déjà côté
    // cible seraient des doublons après bascule. La base les supprime AVANT,
    // sans quoi l'unicité ferait échouer la fusion entière.
    const targetKeys = new Set(targetRows.map((row) => row.key))
    const deduped = sourceRows.filter((row) => targetKeys.has(row.key)).length

    return {
      ...reference,
      strategy: reference.strategy,
      dedupe_on: [...reference.dedupe_on],
      reassigned: sourceRows.length - deduped,
      deduped,
      deleted: 0,
    }
  })
}

// ---------------------------------------------------------------------------
// Les avertissements
// ---------------------------------------------------------------------------

/**
 * CE QUI DOIT ARRÊTER LA MAIN, sans jamais bloquer le geste.
 *
 * Aucun de ces cas n'est une erreur : fusionner une fiche vérifiée dans une
 * fiche qui ne l'est pas peut être exactement ce qu'il faut faire, si c'est la
 * seconde que tout le monde utilise. Mais personne ne doit le découvrir après
 * coup — perdre un sceau ou vingt dossiers à l'occasion d'une fusion se
 * remarque des mois plus tard, quand le public cherche l'organisation.
 */
function warningsFor(source: DuplicateSide, target: DuplicateSide): MergeWarning[] {
  const warnings: MergeWarning[] = []

  if (source.verified_at !== null && target.verified_at === null) {
    warnings.push({ code: 'source_is_verified', values: { name: source.legal_name } })
  }
  if (source.verified_at === null && target.verified_at === null) {
    warnings.push({ code: 'target_not_verified' })
  }
  if (source.proposal_count > target.proposal_count) {
    warnings.push({
      code: 'source_has_more_activity',
      values: { source: source.proposal_count, target: target.proposal_count },
    })
  }
  if (
    effectiveDomains().some(
      (d) => d.organization_id === source.organization_id && d.verified_at !== null,
    )
  ) {
    warnings.push({ code: 'source_has_verified_domain' })
  }
  if (source.country_id !== target.country_id) {
    warnings.push({ code: 'different_countries' })
  }
  if (source.organization_type_code !== target.organization_type_code) {
    warnings.push({ code: 'different_types' })
  }
  return warnings
}

// ---------------------------------------------------------------------------
// L'aperçu complet
// ---------------------------------------------------------------------------

/**
 * TOUT L'ÉCRAN DE FUSION POUR UN SENS DONNÉ.
 *
 * `null` si l'une des fiches est introuvable, ou si la source est DÉJÀ
 * fusionnée : `tg_forbid_merge_chains` interdit les chaînes A → B → C, et
 * proposer un écran qui ne pourra pas valider serait une impasse.
 */
export function mergePreview(sourceId: string, targetId: string, pairId: string | null): MergePreview | null {
  if (sourceId === targetId) return null

  const source = duplicateSideOf(sourceId)
  const target = duplicateSideOf(targetId)
  if (!source || !target) return null
  // Une fiche déjà absorbée ne peut être ni source ni cible :
  // `tg_forbid_merge_chains` refuse A → B → C, « cibler la fiche finale ».
  const merged = mergedOrganizationIds()
  if (target.status === 'merged' || merged.has(targetId)) return null
  if (source.status === 'merged' || merged.has(sourceId)) return null

  const names = effectiveNames()
  const domains = effectiveDomains()
  const targetNames = new Set(
    names
      .filter((n) => n.organization_id === targetId)
      .map((n) => `${n.name.toLowerCase()}|${n.kind}`),
  )
  const targetDomains = new Set(
    domains.filter((d) => d.organization_id === targetId).map((d) => d.domain),
  )

  return {
    source,
    target,
    pair_id: pairId,
    comparisons: comparisonsOf(sourceId, targetId),
    transfers: transfersFor(sourceId, targetId),
    // Étape 1 de `merge_organizations()`, traitée hors registre : les
    // dénominations de la source deviennent des variantes de la cible, et une
    // recherche sur l'ancien nom continue de trouver la bonne fiche. Celles que
    // la cible porte déjà ne sont pas dupliquées.
    transferred_names: names
      .filter((n) => n.organization_id === sourceId)
      .map((n) => ({
        name: n.name,
        kind: n.kind,
        is_confirmed: n.is_confirmed,
        already_present: targetNames.has(`${n.name.toLowerCase()}|${n.kind}`),
      })),
    transferred_domains: domains
      .filter((d) => d.organization_id === sourceId)
      .map((d) => ({
        domain: d.domain,
        verified_at: d.verified_at,
        already_present: targetDomains.has(d.domain),
      })),
    warnings: warningsFor(source, target),
  }
}
