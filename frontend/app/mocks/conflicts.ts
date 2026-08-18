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
 *   venue_capacity  un seul stand : deux séances simultanées de l'édition qui
 *                   OCCUPENT chacune une salle physique — une séance en ligne ou
 *                   pas encore installée n'occupe rien (écart n° 10) ;
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

      // 1. Un seul stand — MAIS SEULEMENT ENTRE SÉANCES QUI L'OCCUPENT
      //    RÉELLEMENT : deux salles physiques différentes du pavillon. Une
      //    séance en salle virtuelle, ou pas encore installée, n'occupe aucun
      //    mètre carré ; la remonter en gravité bloquante apprenait à l'équipe à
      //    ignorer le bandeau d'alerte (écart n° 10, corrigé dans le SQL le
      //    18/08). Deux séances dans la MÊME salle relèvent de la branche 3, qui
      //    le dit mieux en nommant la salle.
      if (a.enforce_room_exclusivity && b.enforce_room_exclusivity && a.room_id !== b.room_id) {
        found.push(conflict('blocking', 'venue_capacity', eventId, "Stand unique de l'événement", a, b))
      }

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
    // Le créneau n'est PAS collé dans le texte : `occurs_at` porte l'instant, et
    // l'écran le situe dans le fuseau de l'édition et la langue du lecteur.
    detail: `${c.session_a_title} ↔ ${c.session_b_title}`,
    session_id: c.session_a,
    occurs_at: c.overlap.slice(2, c.overlap.indexOf('","')),
  }))

  // Les quatre contrôles que la fonction SQL ajoute aux conflits, dans son
  // ordre. Ils ne portent que sur les séances `planned` et `scheduled` : une
  // séance annulée ou reportée n'a rien à régler avant publication.
  for (const session of allSessions) {
    if (session.event_id !== eventId) continue
    if (session.status !== 'planned' && session.status !== 'scheduled') continue

    if (Date.parse(session.ends_at) <= Date.parse(session.starts_at)) {
      issues.push({
        severity: 'blocking',
        issue: 'Session sans créneau valide',
        detail: session.title.fr,
        session_id: session.id,
        occurs_at: session.starts_at,
      })
    }

    // SANS LIEU : l'état normal d'une activité retenue mais pas encore
    // installée — celles du panneau « à placer ». Rien ne l'empêche ; c'est la
    // PUBLICATION qui l'exige, faute de quoi le visiteur cherche une salle qui
    // n'a pas de nom.
    if (session.room_id === null && session.location_note === null) {
      issues.push({
        severity: 'blocking',
        issue: 'Séance sans lieu ni précision de lieu',
        detail: session.title.fr,
        session_id: session.id,
        occurs_at: session.starts_at,
      })
    }

    if (session.is_streamed && session.broadcast_channel_id === null) {
      issues.push({
        severity: 'warning',
        issue: 'Session diffusée sans canal assigné',
        detail: session.title.fr,
        session_id: session.id,
        occurs_at: session.starts_at,
      })
    }

    if (!sessionSpeakers.some((speaker) => speaker.session_id === session.id)) {
      issues.push({
        severity: 'warning',
        issue: 'Session sans intervenant déclaré',
        detail: session.title.fr,
        session_id: session.id,
        occurs_at: session.starts_at,
      })
    }
  }

  return issues
}
