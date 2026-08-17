/**
 * Point d'entrée des séances simulées. Ne contient AUCUNE donnée.
 *
 * La programmation publique ne lit que `publishedSessions` ; le planificateur
 * (A9) lit les trente, publiées ou non — c'est précisément son travail.
 */

import type { Session } from '~/types/programme/session'
import { otherEditionSessions } from './other-editions'
import { plannedSessions } from './planned'
import { publishedSessions } from './published'

export { publishedSessions } from './published'
export { plannedSessions } from './planned'
export { otherEditionSessions } from './other-editions'
export { sessionSpeakers } from './speakers'
export { sessionTracks } from './tracks'
export { sessionOrganizations } from './organizations'

/**
 * Les quarante-quatre séances, dans l'ordre chronologique : les trente de la
 * COP31 et les quatorze des autres éditions (deux COP passées, cycle PACO).
 *
 * Rien ne filtre ici par édition. C'est `useApi()` qui le fait, comme la requête
 * le fera : une séance appartient toujours à une édition, et aucun écran ne
 * mélange deux programmes sans l'avoir décidé.
 */
export const allSessions: Session[] = [
  ...publishedSessions,
  ...plannedSessions,
  ...otherEditionSessions,
].sort((a, b) => a.starts_at.localeCompare(b.starts_at))
