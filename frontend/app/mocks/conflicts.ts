/**
 * Reconstitution de `programme.detect_conflicts()` et de
 * `programme.publication_readiness()` sur les données simulées.
 *
 * AUCUN DE CES CONFLITS N'EMPÊCHE QUOI QUE CE SOIT. Ils alimentent le bandeau
 * d'alerte du planificateur (A9) et l'écran d'arbitrage ; l'équipe déplace ses
 * blocs et passe par des états incohérents, c'est la nature du travail. Le seul
 * garde-fou dur se situe à la PUBLICATION du programme.
 *
 * Les cinq familles sont celles de la fonction SQL, dans le même ordre :
 *
 *   venue_capacity  un seul stand : deux séances simultanées de l'édition ;
 *   broadcast       un seul direct, tous événements confondus ;
 *   room            une salle physique réservée deux fois ;
 *   speaker         un intervenant attendu à deux endroits ;
 *   organization    une organisation programmée deux fois en même temps.
 *
 * Les séances `completed`, `postponed` et `cancelled` sont hors du calcul :
 * seules comptent celles qui occupent réellement une ressource.
 */

import type { ConflictSeverity, PublicationReadinessIssue, ScheduleConflict } from '~/types/programme/session'
import type { Session } from '~/types/programme/session'
import { allSessions, sessionOrganizations, sessionSpeakers } from './sessions'
import { organizations } from './org'
import { people } from './people'
import { rooms } from './rooms'
import { broadcastChannels } from './rooms'
import { events } from './event'

const organizationById = new Map(organizations.map((o) => [o.id, o]))
const personById = new Map(people.map((p) => [p.id, p]))
const roomById = new Map(rooms.map((r) => [r.id, r]))
const channelById = new Map(broadcastChannels.map((c) => [c.id, c]))

/** Deux créneaux se recouvrent-ils ? Bornes `[début, fin)`, comme `tstzrange`. */
function overlaps(a: Session, b: Session): boolean {
  return Date.parse(a.starts_at) < Date.parse(b.ends_at) && Date.parse(b.starts_at) < Date.parse(a.ends_at)
}

/** Intersection des deux créneaux, au format `tstzrange` sérialisé. */
function overlapRange(a: Session, b: Session): string {
  const start = Date.parse(a.starts_at) > Date.parse(b.starts_at) ? a.starts_at : b.starts_at
  const end = Date.parse(a.ends_at) < Date.parse(b.ends_at) ? a.ends_at : b.ends_at
  return `["${start}","${end}")`
}

function conflict(
  severity: ConflictSeverity,
  kind: ScheduleConflict['conflict_kind'],
  subjectId: string | null,
  subjectLabel: string | null,
  a: Session,
  b: Session,
): ScheduleConflict {
  return {
    severity,
    conflict_kind: kind,
    subject_id: subjectId,
    subject_label: subjectLabel,
    session_a: a.id,
    session_a_title: a.title.fr,
    session_b: b.id,
    session_b_title: b.title.fr,
    overlap: overlapRange(a, b),
  }
}

/** Équivalent de `programme.detect_conflicts(event_id)`. */
export function detectConflicts(eventId: string): ScheduleConflict[] {
  const active = allSessions.filter(
    (s) => s.event_id === eventId && ['planned', 'scheduled', 'live'].includes(s.status),
  )
  const event = events.find((e) => e.id === eventId)
  const found: ScheduleConflict[] = []

  for (let i = 0; i < active.length; i++) {
    for (let j = i + 1; j < active.length; j++) {
      const a = active[i]!
      const b = active[j]!
      if (!overlaps(a, b)) continue

      // 1. Un seul stand.
      found.push(conflict('blocking', 'venue_capacity', eventId, "Stand unique de l'événement", a, b))

      // 2. Un seul direct.
      if (a.is_streamed && b.is_streamed && a.broadcast_channel_id === b.broadcast_channel_id) {
        const channel = a.broadcast_channel_id ? channelById.get(a.broadcast_channel_id) : undefined
        found.push(conflict('blocking', 'broadcast', a.broadcast_channel_id, channel?.name.fr ?? null, a, b))
      }

      // 3. Salle physique réservée deux fois. Une salle virtuelle accepte les
      //    créneaux simultanés : elle ne produit aucun conflit.
      if (a.room_id && a.room_id === b.room_id && a.enforce_room_exclusivity) {
        found.push(conflict('blocking', 'room', a.room_id, roomById.get(a.room_id)?.name.fr ?? null, a, b))
      }

      // 4. Intervenant attendu à deux endroits.
      const speakersOf = (s: Session) =>
        new Set(sessionSpeakers.filter((sp) => sp.session_id === s.id).map((sp) => sp.person_id))
      const sharedSpeakers = [...speakersOf(a)].filter((id) => speakersOf(b).has(id))
      for (const personId of sharedSpeakers) {
        found.push(
          conflict('warning', 'speaker', personId, personById.get(personId)?.display_name ?? null, a, b),
        )
      }

      // 5. Organisation programmée deux fois.
      const orgsOf = (s: Session) =>
        new Set(sessionOrganizations.filter((so) => so.session_id === s.id).map((so) => so.organization_id))
      const sharedOrgs = [...orgsOf(a)].filter((id) => orgsOf(b).has(id))
      for (const orgId of sharedOrgs) {
        found.push(
          conflict('warning', 'organization', orgId, organizationById.get(orgId)?.legal_name ?? null, a, b),
        )
      }
    }
  }

  // L'édition doit exister : un identifiant forgé ne renvoie rien, il ne
  // renvoie pas les conflits d'une autre édition.
  return event ? found : []
}

/**
 * Équivalent de `programme.publication_readiness(event_id)` : ce qui doit être
 * réglé avant de rendre la programmation publique. C'est le seul moment où un
 * contrôle bloquant a du sens — pas pendant que l'équipe déplace ses blocs.
 */
export function publicationReadiness(eventId: string): PublicationReadinessIssue[] {
  const issues: PublicationReadinessIssue[] = detectConflicts(eventId).map((c) => ({
    severity: c.severity,
    issue:
      c.conflict_kind === 'venue_capacity'
        ? 'Deux activités simultanées sur un stand unique'
        : c.conflict_kind === 'broadcast'
          ? 'Deux diffusions en direct simultanées'
          : c.conflict_kind === 'room'
            ? 'Salle réservée deux fois'
            : c.conflict_kind === 'speaker'
              ? 'Intervenant attendu à deux endroits'
              : 'Organisation programmée deux fois',
    detail: `${c.session_a_title} ↔ ${c.session_b_title} (${c.overlap})`,
    session_id: c.session_a,
  }))

  // Une séance sans salle ni précision de lieu ne peut pas être publiée.
  for (const session of allSessions) {
    if (session.event_id !== eventId) continue
    if (session.status === 'cancelled' || session.status === 'postponed') continue
    if (session.room_id === null && session.location_note === null) {
      issues.push({
        severity: 'blocking',
        issue: 'Séance sans lieu ni précision de lieu',
        detail: session.title.fr,
        session_id: session.id,
      })
    }
  }

  return issues
}
