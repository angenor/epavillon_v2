/**
 * Les deux vues du modèle, reconstituées à partir des données simulées.
 *
 * `programme.v_public_schedule` et `programme.v_proposal_dashboard` répondent
 * chacune à un écran entier en une requête. Les écrans doivent les consommer
 * TELLES QUELLES : leurs colonnes portent d'autres noms que les tables
 * (`organization_name`, `room_name`) et des valeurs déjà calculées (état
 * temporel, rang, revues manquantes). Recomposer la jointure côté application
 * produirait deux implémentations divergentes sur les cas limites.
 *
 * Ici, ces lignes sont DÉRIVÉES des mocks plutôt qu'écrites à la main : c'est ce
 * que fait la base, et c'est ce qui garantit qu'aucune valeur ne contredise sa
 * source. Le jour où l'API répond réellement, ce fichier disparaît sans qu'un
 * écran change.
 *
 * `temporal_state` est calculé par rapport à l'instant courant, comme la vue
 * SQL. Les séances de novembre 2027 sont donc « à venir » tant que cette date
 * n'est pas passée.
 */

import type {
  ProposalDashboardRow,
  PublicScheduleRow,
  ScheduleThemeBadge,
  ScheduleTrackBadge,
  TemporalState,
} from '~/types/views'
import { callsForProposals } from './calls'
import { countries } from './reference'
import { organizations } from './org'
import { programmeTracks } from './tracks'
import { rooms } from './rooms'
import { entityTerms, taxonomyTerms } from './reference'
import { allProposals, proposalComments, proposalSpeakers } from './proposals'
import { allSessions, sessionTracks } from './sessions'
import { attachedImage } from './covers'
import { registrations } from './registrations'
import { reviewAssignments } from './reviews'

const organizationById = new Map(organizations.map((o) => [o.id, o]))
const countryById = new Map(countries.map((c) => [c.id, c]))
const roomById = new Map(rooms.map((r) => [r.id, r]))
const trackById = new Map(programmeTracks.map((t) => [t.id, t]))
const termById = new Map(taxonomyTerms.map((t) => [t.id, t]))

/** Reproduit le calcul d'état temporel de la vue, dans le même ordre de tests. */
function temporalState(session: (typeof allSessions)[number], now: number): TemporalState {
  if (session.status === 'cancelled') return 'cancelled'
  if (session.status === 'postponed') return 'postponed'
  if (Date.parse(session.ends_at) < now) return 'past'
  if (Date.parse(session.starts_at) <= now) return 'ongoing'
  return 'upcoming'
}

/**
 * `programme.v_public_schedule` — la vue ne retient que les séances publiées.
 * Recalculée à l'appel : l'état temporel dépend de l'instant présent.
 */
export function publicSchedule(): PublicScheduleRow[] {
  const now = Date.now()

  return allSessions
    .filter((s) => s.published_at !== null)
    .map((s) => {
      const organization = s.organization_id ? organizationById.get(s.organization_id) : undefined
      const country = organization?.country_id ? countryById.get(organization.country_id) : undefined
      const room = s.room_id ? roomById.get(s.room_id) : undefined

      /**
       * `reference.term_badges()` rejouée : code, libellé et couleur, dans
       * l'ordre de `entity_terms.sort_order` puis de celui du terme. Les termes
       * inactifs sont écartés, comme en base.
       */
      const themes: ScheduleThemeBadge[] = entityTerms
        .filter((link) => link.entity_table === 'sessions' && link.entity_id === s.id)
        .map((link) => ({ link, term: termById.get(link.term_id) }))
        .filter((pair) => pair.term !== undefined && pair.term.is_active && pair.term.taxonomy_code === 'activity_theme')
        .sort((a, b) => a.link.sort_order - b.link.sort_order || a.term!.sort_order - b.term!.sort_order)
        .map((pair) => ({
          code: pair.term!.code,
          label: pair.term!.label,
          color: pair.term!.color_hex,
          icon: pair.term!.icon,
        }))

      const tracks: ScheduleTrackBadge[] = sessionTracks
        .filter((link) => link.session_id === s.id)
        .map((link) => trackById.get(link.track_id))
        .filter((track) => track !== undefined && track.published_at !== null)
        .map((track) => ({
          slug: track!.slug,
          title: track!.title,
          color: track!.color_hex,
          kind: track!.kind,
        }))

      return {
        id: s.id,
        event_id: s.event_id,
        event_day_id: s.event_day_id,
        proposal_id: s.proposal_id,
        slug: s.slug,
        title: s.title,
        summary: s.summary,
        starts_at: s.starts_at,
        ends_at: s.ends_at,
        timezone: s.timezone,
        format: s.format,
        status: s.status,
        room_id: s.room_id,
        room_name: room?.name ?? null,
        organization_id: s.organization_id,
        organization_name: organization?.legal_name ?? null,
        organization_acronym: organization?.acronym ?? null,
        organization_country_code: country?.iso2 ?? null,
        organization_country: country?.name ?? null,
        is_streamed: s.is_streamed,
        broadcast_channel_id: s.broadcast_channel_id,
        capacity: s.capacity,
        tracks,
        // Reproduit le `COALESCE` de la vue : couverture de la séance, à défaut
        // celle de la proposition d'origine. Le repli est la règle — l'image est
        // jointe au DÉPÔT, et personne n'en téléverse une seconde après coup.
        cover:
          attachedImage('programme', 'sessions', s.id) ??
          attachedImage('programme', 'proposals', s.proposal_id),
        temporal_state: temporalState(s, now),
        // Inscrits et présents, annulations exclues.
        registered_count: registrations.filter((r) => r.session_id === s.id && r.status !== 'cancelled').length,
        // Deux colonnes, deux usages : les codes filtrent, les pastilles
        // s'affichent. Ne jamais reconstruire un libellé depuis un code.
        theme_codes: themes.map((theme) => theme.code),
        themes,
      }
    })
    .sort((a, b) => a.starts_at.localeCompare(b.starts_at))
}

/**
 * `programme.v_proposal_dashboard` — la liste du back-office, dossiers
 * supprimés exclus.
 *
 * RAPPEL : cette liste se filtre TOUJOURS par le périmètre d'administration de
 * la personne connectée, y compris quand l'utilisateur forge une URL. La vue ne
 * porte pas ce filtre — c'est `useApi()` puis l'API qui l'appliquent.
 *
 * `title` expose le titre multilingue brut, `title_text` sa version résolue en
 * français : deux colonnes, deux usages, jamais le même nom pour deux types.
 */
export function proposalDashboard(): ProposalDashboardRow[] {
  const rows = allProposals
    .filter((p) => p.deleted_at === null)
    .map((p) => {
      const call = callsForProposals.find((c) => c.id === p.call_id)
      const requiredReviews = call?.required_reviews ?? null

      return {
        id: p.id,
        reference_code: p.reference_code,
        event_id: p.event_id,
        call_id: p.call_id,
        organization_id: p.organization_id,
        organization_name: organizationById.get(p.organization_id)?.legal_name ?? '',
        title: p.title,
        title_text: p.title.fr,
        status: p.status,
        submitted_at: p.submitted_at,
        weighted_score: p.weighted_score,
        average_score: p.average_score,
        is_knocked_out: p.is_knocked_out,
        review_count: p.review_count,
        required_reviews: requiredReviews,
        reviews_missing:
          requiredReviews === null ? null : Math.max(0, requiredReviews - p.review_count),
        assigned_reviewers: reviewAssignments.filter(
          (a) => a.proposal_id === p.id && a.recused_at === null,
        ).length,
        open_change_requests: proposalComments.filter(
          (c) => c.proposal_id === p.id && c.is_change_request && c.resolved_at === null && c.deleted_at === null,
        ).length,
        speaker_count: proposalSpeakers.filter((s) => s.proposal_id === p.id).length,
        // Rang provisoire : recalculé juste après, une fois l'ensemble trié.
        event_rank: 0,
      }
    })

  // Rang au sein de l'édition, par note pondérée décroissante. Les dossiers non
  // notés viennent après, sans rang significatif.
  const ranked = [...rows].sort((a, b) => (b.weighted_score ?? -1) - (a.weighted_score ?? -1))
  ranked.forEach((row, index) => {
    row.event_rank = index + 1
  })

  return rows
}
