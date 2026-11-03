/**
 * Présence : `POST /presence` (chaque onglet visible) et `GET /admin/presence`
 * (carte « En ce moment » du tableau de bord, `analytics.presence.read`).
 * Compté en mémoire par l'API (`backend/crates/api/src/presence/`), sans table.
 */

import type { IsoDateTime } from './shared'

/** Liste fermée, partagée avec l'API ; un code inconnu y compte comme `other`. */
export type PresenceZone =
  | 'live'
  | 'programme'
  | 'day'
  | 'activity'
  | 'organization'
  | 'admin'
  | 'negotiation'
  | 'other'

export interface PresenceBeat {
  /** Tiré au hasard par l'onglet, gardé dans `sessionStorage`. */
  tab: string
  zone: PresenceZone
}

export interface PresenceAck {
  /** Compté comme connecté, d'après la session : jamais déclaré par le site. */
  signed_in: boolean
}

export interface PresenceZoneCount {
  zone: PresenceZone
  count: number
}

export interface PresenceSample {
  /** Début de la minute. */
  at: IsoDateTime
  total: number
}

export interface AdminPresence {
  total: number
  signed_in: number
  visitors: number
  /** Effectif décroissant ; une zone vide n'y figure pas. */
  zones: PresenceZoneCount[]
  /** Une entrée par minute, la plus ancienne d'abord ; la dernière est en cours. */
  last_hour: PresenceSample[]
  peak: PresenceSample | null
  observed_at: IsoDateTime
}
