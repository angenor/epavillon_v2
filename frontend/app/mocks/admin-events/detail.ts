/**
 * GESTION DES ÉVÉNEMENTS (A10) — les LECTURES : la composition des six onglets.
 *
 * Rien ici n'écrit. Ce fichier assemble ce que l'écran de détail affiche —
 * journées, fils, lieux et salles, canaux, appel et grille, comité — et le
 * réunit dans `editionDetail()`.
 *
 * ── UNE COMPOSITION, PAS DOUZE LECTURES ─────────────────────────────────────
 *
 * Les six onglets partent ensemble, et chaque écriture les fait tous recalculer
 * (voir `writes.ts`). Le coût est assumé : ajouter une salle change le nombre
 * d'activités plaçables, retirer un jour détache des séances, désactiver un canal
 * touche la règle du direct unique. Rendre seulement l'objet modifié laisserait
 * cinq onglets afficher des décomptes faux.
 *
 * ── CE QUI EST EN LECTURE SEULE, ET POURQUOI ────────────────────────────────
 *
 * `EditionTrack.session_count` compte les séances rattachées à un fil. Ce
 * rattachement est une décision ÉDITORIALE prise au planificateur (A9), dans
 * `programme.session_tracks` : aucune fonction de ce module ne l'écrit.
 */

import type {
  CommitteeCandidate,
  DayGenerationPlan,
  EditionCall,
  EditionChannel,
  EditionCommitteeMember,
  EditionCriterion,
  EditionDay,
  EditionDetail,
  EditionRoom,
  EditionTrack,
  EditionVenue,
} from '~/types/admin-events'
import type { CallForProposals, ReviewCriterion } from '~/types/event/call'
import type { EventEdition } from '~/types/event/edition'
import type { Uuid } from '~/types/shared'
import { allSessions, sessionTracks } from '../sessions'
import { reviews, reviewScores, reviewAssignments } from '../reviews'
import { people } from '../people'
import { taxonomyTerms } from '../reference'
import { attachedImage } from '../covers'
import { effectivePermissions } from '../permissions'
import {
  allProposals,
} from '../proposals'
import {
  badgesOf,
  callOf,
  calls,
  channelRows,
  criteria,
  datesBetween,
  days,
  editions,
  listRow,
  organizationById,
  periodOf,
  personById,
  reviewers,
  roomRows,
  tracks,
  venueRows,
} from './core'

// ---------------------------------------------------------------------------
// Onglet « Journées du calendrier »
// ---------------------------------------------------------------------------

export function dayGenerationPlan(edition: EventEdition): DayGenerationPlan {
  const period = periodOf(edition)
  const wanted = datesBetween(period.first_day, period.last_day)
  const existing = days.filter((d) => d.event_id === edition.id)
  const existingDates = new Set(existing.map((d) => d.day_date))

  return {
    to_create: wanted.filter((date) => !existingDates.has(date)),
    to_review: existing
      .filter((d) => !wanted.includes(d.day_date))
      .map((d) => ({
        id: d.id,
        day_date: d.day_date,
        session_count: allSessions.filter((s) => s.event_day_id === d.id).length,
      })),
    unchanged: existing.filter((d) => wanted.includes(d.day_date)).length,
  }
}

/** Le plan de génération, ANNONCÉ AVANT d'agir : on montre, on ne fait pas découvrir. */
export function planDayGeneration(eventId: Uuid): DayGenerationPlan | null {
  const edition = editions.find((e) => e.id === eventId)
  return edition ? dayGenerationPlan(edition) : null
}

export function editionDays(edition: EventEdition): EditionDay[] {
  const period = periodOf(edition)
  const wanted = new Set(datesBetween(period.first_day, period.last_day))

  return days
    .filter((d) => d.event_id === edition.id)
    .sort((a, b) => a.day_date.localeCompare(b.day_date))
    .map((d) => ({
      id: d.id,
      day_date: d.day_date,
      title: d.title,
      slug: d.slug,
      description: d.description,
      is_featured: d.is_featured,
      color_hex: d.color_hex,
      sort_order: d.sort_order,
      session_count: allSessions.filter((s) => s.event_day_id === d.id).length,
      is_outside_period: !wanted.has(d.day_date),
    }))
}

// ---------------------------------------------------------------------------
// Onglet « Journées spéciales »
// ---------------------------------------------------------------------------

export function editionTracks(eventId: Uuid): EditionTrack[] {
  return tracks
    .filter((t) => t.event_id === eventId)
    .sort((a, b) => a.sort_order - b.sort_order)
    .map((t) => ({
      id: t.id,
      code: t.code,
      slug: t.slug,
      kind: t.kind,
      title: t.title,
      subtitle: t.subtitle,
      description: t.description,
      starts_on: t.starts_on,
      ends_on: t.ends_on,
      color_hex: t.color_hex,
      curated_by: t.curated_by,
      curator_name: t.curated_by ? (personById.get(t.curated_by)?.display_name ?? null) : null,
      published_at: t.published_at,
      sort_order: t.sort_order,
      // Composée au planificateur (A9), lue ici.
      session_count: sessionTracks.filter((st) => st.track_id === t.id).length,
      themes: badgesOf('event', 'programme_tracks', t.id),
    }))
}

// ---------------------------------------------------------------------------
// Onglet « Lieux et salles »
// ---------------------------------------------------------------------------

export function editionVenues(eventId: Uuid): EditionVenue[] {
  return venueRows
    .filter((v) => v.event_id === eventId)
    .map((v) => ({
      id: v.id,
      name: v.name,
      kind: v.kind,
      address: v.address,
      map_url: v.map_url,
      rooms: roomRows
        .filter((r) => r.venue_id === v.id)
        .sort((a, b) => a.sort_order - b.sort_order)
        .map<EditionRoom>((r) => ({
          id: r.id,
          venue_id: r.venue_id,
          name: r.name,
          code: r.code,
          capacity: r.capacity,
          is_virtual: r.is_virtual,
          has_streaming: r.has_streaming,
          equipment: r.equipment,
          sort_order: r.sort_order,
          session_count: allSessions.filter((s) => s.room_id === r.id).length,
        })),
    }))
}

// ---------------------------------------------------------------------------
// Onglet « Canal de diffusion »
// ---------------------------------------------------------------------------

export function editionChannels(eventId: Uuid): EditionChannel[] {
  return channelRows
    .filter((c) => c.event_id === eventId || c.event_id === null)
    .map((c) => ({
      id: c.id,
      event_id: c.event_id,
      code: c.code,
      name: c.name,
      provider: c.provider,
      channel_ref: c.channel_ref,
      locale: c.locale,
      is_default: c.is_default,
      is_active: c.is_active,
      session_count: allSessions.filter((s) => s.broadcast_channel_id === c.id).length,
    }))
    // Le canal de l'édition d'abord, les canaux généraux de la plateforme ensuite.
    .sort((a, b) => Number(a.event_id === null) - Number(b.event_id === null))
}

// ---------------------------------------------------------------------------
// Onglet « Appel à propositions »
// ---------------------------------------------------------------------------

export function criterionRows(callId: Uuid): EditionCriterion[] {
  const submitted = new Set(reviews.map((r) => r.id))
  return criteria
    .filter((c) => c.call_id === callId)
    .sort((a, b) => a.sort_order - b.sort_order)
    .map((c) => ({
      id: c.id,
      code: c.code,
      label: c.label,
      description: c.description,
      max_score: c.max_score,
      weight: c.weight,
      is_knockout: c.is_knockout,
      sort_order: c.sort_order,
      score_count: reviewScores.filter((s) => s.criterion_id === c.id && submitted.has(s.review_id))
        .length,
    }))
}

export function editionCall(eventId: Uuid): EditionCall | null {
  const call = callOf(eventId)
  if (!call) return null

  const grid = criterionRows(call.id)
  const deadline = call.extended_until ?? call.closes_at
  const now = Date.now()

  return {
    ...call,
    effective_deadline: deadline,
    // `event.is_call_open()` : le statut ET la fenêtre. Un appel « ouvert » dont
    // l'échéance est passée n'est pas ouvert, et c'est la fonction de la base qui
    // tranche — pas le statut lu seul.
    is_open:
      call.status === 'open' && Date.parse(call.opens_at) <= now && now <= Date.parse(deadline),
    // `event.max_weighted_score()`
    max_weighted_score: grid.reduce((sum, c) => sum + c.max_score * c.weight, 0),
    proposal_count: allProposals.filter(
      (p) => p.call_id === call.id && p.status !== 'draft' && p.deleted_at === null,
    ).length,
    criteria: grid,
  }
}

// ---------------------------------------------------------------------------
// Onglet « Comité de sélection »
// ---------------------------------------------------------------------------

function canReview(personId: Uuid, eventId: Uuid): boolean {
  return effectivePermissions(personId).some(
    (entry) =>
      entry.permission_code === 'programme.review.write' &&
      (entry.scope_type === 'global' || (entry.scope_type === 'event' && entry.scope_id === eventId)),
  )
}

export function candidateOf(personId: Uuid, eventId: Uuid): CommitteeCandidate | null {
  const person = personById.get(personId)
  if (!person) return null
  const organization = person.primary_organization_id
    ? organizationById.get(person.primary_organization_id)
    : undefined

  return {
    person_id: person.id,
    full_name: person.display_name,
    email: person.primary_email,
    organization_name: organization?.legal_name ?? null,
    has_review_permission: canReview(person.id, eventId),
  }
}

export function committeeOfCall(callId: Uuid, eventId: Uuid): EditionCommitteeMember[] {
  const submittedByPerson = new Map<string, number>()
  for (const review of reviews) {
    if (review.submitted_at === null) continue
    submittedByPerson.set(review.reviewer_id, (submittedByPerson.get(review.reviewer_id) ?? 0) + 1)
  }

  return reviewers
    .filter((r) => r.call_id === callId)
    .map((r) => {
      const candidate = candidateOf(r.person_id, eventId)
      return {
        person_id: r.person_id,
        full_name: candidate?.full_name ?? '',
        email: candidate?.email ?? '',
        organization_name: candidate?.organization_name ?? null,
        is_lead: r.is_lead,
        workload_cap: r.workload_cap,
        added_at: r.added_at,
        // Déports exclus : un dossier dont on s'est retiré n'est plus une charge.
        assigned_count: reviewAssignments.filter(
          (a) => a.reviewer_id === r.person_id && a.recused_at === null,
        ).length,
        submitted_count: submittedByPerson.get(r.person_id) ?? 0,
        has_review_permission: candidate?.has_review_permission ?? false,
      }
    })
    .sort((a, b) => Number(b.is_lead) - Number(a.is_lead) || a.full_name.localeCompare(b.full_name, 'fr'))
}

// ---------------------------------------------------------------------------
// La composition entière
// ---------------------------------------------------------------------------

/**
 * Personnel de l'IFDD assignable — responsable d'un fil, membre du comité.
 *
 * Le critère est une PERMISSION, jamais un nom de rôle : quiconque détient
 * `event.event.manage` ou `programme.review.write`, globalement ou sur cette
 * édition, fait partie des personnes que l'équipe peut désigner. Une liste de
 * rôles écrite en dur aurait laissé de côté le premier rôle ajouté au catalogue.
 */
function staffCandidates(eventId: Uuid, codes: string[]): CommitteeCandidate[] {
  return people
    .filter((person) => person.status === 'active')
    .filter((person) =>
      effectivePermissions(person.id).some(
        (entry) =>
          codes.includes(entry.permission_code) &&
          (entry.scope_type === 'global' ||
            (entry.scope_type === 'event' && entry.scope_id === eventId)),
      ),
    )
    .map((person) => candidateOf(person.id, eventId))
    .filter((candidate): candidate is CommitteeCandidate => candidate !== null)
    .sort((a, b) => a.full_name.localeCompare(b.full_name, 'fr'))
}

export function editionDetail(eventId: Uuid): EditionDetail | null {
  const edition = editions.find((e) => e.id === eventId)
  if (!edition) return null

  const call = editionCall(eventId)
  const committee = call ? committeeOfCall(call.id, eventId) : []
  const seated = new Set(committee.map((m) => m.person_id))

  return {
    edition: listRow(edition),
    // Les deux textes longs, portés par le DÉTAIL et non par la ligne de liste :
    // c'est le formulaire de modification qui en a besoin, pas le tableau.
    description: edition.description,
    highlights: edition.highlights,
    period: periodOf(edition),
    images: {
      banner: attachedImage('event', 'events', edition.id, 'banner'),
      cover: attachedImage('event', 'events', edition.id, 'cover'),
      thumbnail: attachedImage('event', 'events', edition.id, 'thumbnail'),
    },
    days: editionDays(edition),
    tracks: editionTracks(eventId),
    venues: editionVenues(eventId),
    channels: editionChannels(eventId),
    call,
    committee,
    curators: staffCandidates(eventId, ['event.event.manage', 'programme.session.schedule']),
    committee_candidates: staffCandidates(eventId, ['programme.review.write']).filter(
      (candidate) => !seated.has(candidate.person_id),
    ),
    available_themes: taxonomyTerms
      .filter((term) => term.taxonomy_code === 'activity_theme' && term.is_active)
      .sort((a, b) => a.sort_order - b.sort_order)
      .map((term) => ({
        code: term.code,
        label: term.label,
        color: term.color_hex,
        icon: term.icon,
      })),
  }
}
