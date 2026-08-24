/**
 * GESTION DES ÉVÉNEMENTS (A10) — l'APPEL À PROPOSITIONS et son COMITÉ.
 *
 * Détaché de `writes.ts` pour tenir le garde-fou de mille lignes de `CLAUDE.md`,
 * et sur la seule ligne de partage qui vaille : ces écritures demandent
 * `event.call.manage` quand les autres demandent `event.event.manage`, et un
 * chargé de programmation compose ses journées spéciales sans jamais toucher à
 * la grille d'évaluation.
 *
 * MÊMES RÈGLES QUE `writes.ts` : les contraintes de `060_events.sql` refusent
 * ici comme elles refuseront en base — `ck_calls_window`, `ck_calls_extension`,
 * `ck_calls_speakers`, `ck_calls_duration_bounds`, `ck_calls_daily_window`,
 * `ux_calls_one_per_event`, `ux_calls_code`, `ux_review_criteria` —, et les
 * écritures mutent les tableaux partagés de `core.ts`.
 */

import type {
  CallFormError,
  CallSaveResult,
  CommitteePayload,
  CommitteeSaveResult,
  EditionCallPayload,
  EditionCriterion,
} from '~/types/admin-events'
import type { CallForProposals } from '~/types/event/call'
import type { Uuid } from '~/types/shared'
import { ApiRequestError } from '~/utils/api-error'
import { seedDefaultCriteria } from '../criteria'
import { calls, criteria, newId, reviewers } from './core'
import { committeeOfCall, criterionRows, editionCall } from './detail'

function validateCall(payload: EditionCallPayload): CallFormError[] {
  const errors: CallFormError[] = []
  const push = (code: CallFormError['code'], field: string | null, index: number | null = null) =>
    errors.push({ code, field, criterion_index: index })

  if (!payload.title.fr?.trim()) push('required', 'title')
  if (!payload.code.trim()) push('required', 'code')

  // `ck_calls_window`
  if (Date.parse(payload.closes_at) <= Date.parse(payload.opens_at)) push('window', 'closes_at')
  // `ck_calls_extension` — une prolongation dépasse toujours la clôture annoncée.
  if (payload.extended_until && Date.parse(payload.extended_until) <= Date.parse(payload.closes_at)) {
    push('extension', 'extended_until')
  }
  // `ck_calls_speakers`
  if (payload.max_speakers < payload.min_speakers) push('speakers', 'max_speakers')
  // `ck_calls_duration_bounds` — bornes, puis durée par défaut PROPOSABLE.
  if (
    payload.min_duration_minutes < 15 ||
    payload.min_duration_minutes > 600 ||
    payload.max_duration_minutes < payload.min_duration_minutes ||
    payload.max_duration_minutes > 600 ||
    payload.default_duration_minutes < payload.min_duration_minutes ||
    payload.default_duration_minutes > payload.max_duration_minutes
  ) {
    push('duration_bounds', 'default_duration_minutes')
  }
  // `ck_calls_daily_window`
  if (payload.daily_end_time <= payload.daily_start_time) push('daily_window', 'daily_end_time')

  // `ux_calls_one_per_event` — la cardinalité 0..1, tenue par la base.
  const rival = calls.find(
    (c) => c.event_id === payload.event_id && c.status !== 'cancelled' && c.id !== payload.id,
  )
  if (rival) push('already_exists', null)

  // `ux_calls_code`
  if (calls.some((c) => c.event_id === payload.event_id && c.code === payload.code && c.id !== payload.id)) {
    push('code_taken', 'code')
  }

  // Une grille vide n'évalue rien : `refresh_proposal_score()` ne pourrait poser
  // aucune note, et le comité se retrouverait devant une fiche sans critère.
  if (payload.criteria.length === 0) push('criteria_empty', 'criteria')

  const seen = new Set<string>()
  payload.criteria.forEach((criterion, index) => {
    if (!criterion.code.trim() || !criterion.label.fr?.trim()) push('required', 'criteria', index)
    if (seen.has(criterion.code)) push('criterion_code_duplicate', 'criteria', index)
    seen.add(criterion.code)
  })

  return errors
}

/**
 * ENREGISTRER L'APPEL ET SA GRILLE.
 *
 * LA GRILLE PART AVEC L'APPEL. Deux enregistrements distincts laisseraient
 * exister un appel sans critère, ce qu'aucun écran ne pourrait ensuite évaluer.
 *
 * UN CRITÈRE DÉJÀ NOTÉ NE SE RETIRE PAS : `programme.review_scores` le référence
 * en `ON DELETE CASCADE`, et le refus sort en 422. Son barème, lui, se change —
 * la réponse porte alors `scores_affected` pour que l'écran dise que les
 * moyennes vont bouger. La v1 laissait modifier un barème sans rien annoncer, et
 * les notes affichées cessaient de correspondre à la grille.
 */
export function saveCall(payload: EditionCallPayload, actorId: Uuid | null): CallSaveResult {
  const errors = validateCall(payload)
  if (errors.length > 0) return { ok: false, call: null, errors, scores_affected: false }

  // Avant toute mutation, comme la transaction de l'API qui n'écrit rien
  // lorsqu'elle refuse.
  if (payload.id) refuseScoredRemoval(payload.id, payload.criteria)

  const now = new Date().toISOString()
  const existing = payload.id ? calls.find((c) => c.id === payload.id) : undefined

  const call: CallForProposals =
    existing ??
    {
      id: newId('7030'),
      event_id: payload.event_id,
      code: payload.code,
      title: payload.title,
      description: null,
      status: 'draft',
      opens_at: payload.opens_at,
      closes_at: payload.closes_at,
      extended_until: null,
      results_expected_at: null,
      max_proposals_per_organization: null,
      requires_verified_organization: false,
      min_speakers: 1,
      max_speakers: 10,
      default_duration_minutes: 60,
      min_duration_minutes: 45,
      max_duration_minutes: 150,
      daily_start_time: '09:00:00',
      daily_end_time: '17:00:00',
      allowed_formats: ['online', 'in_person', 'hybrid'],
      required_reviews: 2,
      blind_review: true,
      guidelines_url: null,
      created_by: actorId,
      created_at: now,
      updated_at: now,
    }

  Object.assign(call, {
    code: payload.code,
    title: payload.title,
    description: payload.description,
    status: payload.status,
    opens_at: payload.opens_at,
    closes_at: payload.closes_at,
    extended_until: payload.extended_until,
    results_expected_at: payload.results_expected_at,
    max_proposals_per_organization: payload.max_proposals_per_organization,
    requires_verified_organization: payload.requires_verified_organization,
    min_speakers: payload.min_speakers,
    max_speakers: payload.max_speakers,
    default_duration_minutes: payload.default_duration_minutes,
    min_duration_minutes: payload.min_duration_minutes,
    max_duration_minutes: payload.max_duration_minutes,
    daily_start_time: payload.daily_start_time,
    daily_end_time: payload.daily_end_time,
    allowed_formats: payload.allowed_formats,
    required_reviews: payload.required_reviews,
    blind_review: payload.blind_review,
    guidelines_url: payload.guidelines_url,
    updated_at: now,
  })

  const isCreation = existing === undefined
  if (isCreation) calls.push(call)

  const affected = applyCriteria(call.id, payload.criteria)

  return { ok: true, call: editionCall(payload.event_id), errors: [], scores_affected: affected }
}

/**
 * LE REFUS QUI SAUVE LES NOTES — 422 `EVENT_CRITERION_HAS_SCORES`.
 *
 * Il sort en erreur HTTP et non dans `errors` : `CallErrorCode` n'a aucune
 * variante pour l'exprimer, et le message NOMME le critère — « ce critère porte
 * des notes » sans dire lequel oblige à ouvrir la base pour savoir quoi garder.
 */
function refuseScoredRemoval(callId: Uuid, wanted: EditionCriterion[]): void {
  const keptCodes = new Set(wanted.map((entry) => entry.code))
  const removed = criterionRows(callId).find(
    (criterion) => !keptCodes.has(criterion.code) && criterion.score_count > 0,
  )
  if (!removed) return

  const name = removed.label.fr ?? removed.label.en ?? removed.code
  throw new ApiRequestError(
    {
      code: 'EVENT_CRITERION_HAS_SCORES',
      message: `Le critère « ${name} » porte déjà ${removed.score_count} note(s) : le retirer effacerait l'argumentaire des évaluations rendues.`,
      field: 'criteria',
    },
    422,
  )
}

/**
 * Remplace la grille d'un appel, **par CODE** et non par identifiant — c'est
 * ainsi que l'API rapproche les deux grilles. Renommer un code n'est donc pas
 * une modification mais un retrait suivi d'un ajout.
 *
 * Rend vrai quand un critère CONSERVÉ voit son barème ou son poids changer alors
 * qu'il porte déjà des notes : les moyennes déjà calculées vont bouger. Le
 * retrait, lui, ne passe plus par là — il est refusé.
 */
function applyCriteria(callId: Uuid, wanted: EditionCriterion[]): boolean {
  const before = criterionRows(callId)
  const keptCodes = new Set(wanted.map((entry) => entry.code))
  let affected = false

  for (let i = criteria.length - 1; i >= 0; i -= 1) {
    const criterion = criteria[i]!
    if (criterion.call_id !== callId || keptCodes.has(criterion.code)) continue
    criteria.splice(i, 1)
  }

  wanted.forEach((entry, index) => {
    const sort_order = (index + 1) * 10
    const existing = criteria.find((c) => c.call_id === callId && c.code === entry.code)
    if (existing) {
      // Les barèmes sont des `numeric(5,2)` : les comparer au flottant près
      // ferait signaler un changement là où la base n'en verrait aucun.
      const scaleChanged =
        Math.round(existing.max_score * 100) !== Math.round(entry.max_score * 100) ||
        Math.round(existing.weight * 100) !== Math.round(entry.weight * 100)
      if (scaleChanged && (before.find((c) => c.code === entry.code)?.score_count ?? 0) > 0) {
        affected = true
      }
      Object.assign(existing, {
        label: entry.label,
        description: entry.description,
        max_score: entry.max_score,
        weight: entry.weight,
        is_knockout: entry.is_knockout,
        sort_order,
      })
      return
    }
    criteria.push({
      id: newId('7031'),
      call_id: callId,
      code: entry.code,
      label: entry.label,
      description: entry.description,
      max_score: entry.max_score,
      weight: entry.weight,
      is_knockout: entry.is_knockout,
      sort_order,
    })
  })

  return affected
}

/**
 * LA GRILLE PAR DÉFAUT — `event.seed_default_criteria()`.
 *
 * Proposée à la création d'un appel, et rien de plus : six critères alignés sur
 * les usages de l'IFDD, modifiables appel par appel. Le formulaire s'ouvre ainsi
 * pré-rempli plutôt que devant six lignes vides — la grille par défaut existe en
 * base précisément pour cela.
 */
export function defaultCriteriaGrid(): EditionCriterion[] {
  return seedDefaultCriteria('', 0).map((criterion, index) => ({
    // Aucun identifiant : ces lignes ne sont pas encore en base.
    id: null,
    code: criterion.code,
    label: criterion.label,
    description: criterion.description,
    max_score: criterion.max_score,
    weight: criterion.weight,
    is_knockout: criterion.is_knockout,
    sort_order: (index + 1) * 10,
    score_count: 0,
  }))
}

// ---------------------------------------------------------------------------
// 7. LE COMITÉ DE SÉLECTION
// ---------------------------------------------------------------------------

/**
 * ENREGISTRER LA COMPOSITION DU COMITÉ, D'UN SEUL GESTE.
 *
 * La liste envoyée REMPLACE la précédente : ajouts, retraits et plafonds partent
 * ensemble. Un comité se compose en le regardant en entier — répartir la charge
 * suppose de voir tout le monde —, et trois appels séparés auraient permis
 * d'enregistrer un plafond sur quelqu'un qu'on vient de retirer.
 *
 * SIÉGER N'ACCORDE AUCUN DROIT : `event.call_reviewers` dit la composition,
 * `identity.role_assignments` dit l'accès. La réponse rend donc, pour chaque
 * membre, s'il détient bien `programme.review.write` sur l'édition — sans quoi on
 * confierait des dossiers à quelqu'un qui ne peut pas les ouvrir.
 */
export function saveCommittee(payload: CommitteePayload, eventId: Uuid): CommitteeSaveResult {
  const before = committeeOfCall(payload.call_id, eventId)
  const wanted = new Map(payload.members.map((m) => [m.person_id, m]))
  const now = new Date().toISOString()

  const removed = before
    .filter((member) => !wanted.has(member.person_id) && member.assigned_count > 0)
    .map((member) => ({ full_name: member.full_name, assigned_count: member.assigned_count }))

  for (let i = reviewers.length - 1; i >= 0; i -= 1) {
    const row = reviewers[i]!
    if (row.call_id === payload.call_id && !wanted.has(row.person_id)) reviewers.splice(i, 1)
  }

  for (const member of payload.members) {
    const existing = reviewers.find(
      (r) => r.call_id === payload.call_id && r.person_id === member.person_id,
    )
    if (existing) {
      existing.is_lead = member.is_lead
      existing.workload_cap = member.workload_cap
      continue
    }
    reviewers.push({
      call_id: payload.call_id,
      person_id: member.person_id,
      is_lead: member.is_lead,
      workload_cap: member.workload_cap,
      added_at: now,
    })
  }

  return {
    ok: true,
    members: committeeOfCall(payload.call_id, eventId),
    removed_with_assignments: removed,
  }
}
