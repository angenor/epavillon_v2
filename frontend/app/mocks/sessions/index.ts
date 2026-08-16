/**
 * Point d'entrée des séances simulées. Ne contient AUCUNE donnée.
 *
 * La programmation publique ne lit que `publishedSessions` ; le planificateur
 * (A9) lit les trente, publiées ou non — c'est précisément son travail.
 */

import type { Session } from '~/types/programme/session'
import { plannedSessions } from './planned'
import { publishedSessions } from './published'

export { publishedSessions } from './published'
export { plannedSessions } from './planned'
export { sessionSpeakers } from './speakers'
export { sessionTracks } from './tracks'
export { sessionOrganizations } from './organizations'

/** Les trente séances, dans l'ordre chronologique. */
export const allSessions: Session[] = [...publishedSessions, ...plannedSessions].sort((a, b) =>
  a.starts_at.localeCompare(b.starts_at),
)
