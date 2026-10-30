/**
 * Temps réel : ce que disent `/realtime/pulse` et `/realtime/stream`.
 * Dérivé de `backend/crates/api/src/realtime/` ; le signal naît de
 * `platform.signal()` (`010_platform.sql` § 3 bis). Aucun ne porte de donnée :
 * la page relit sa route.
 */

/** Un sujet : `proposal:<uuid>`, `event:<uuid>:mailings`, `edition:<uuid>:public:live`. */
export type RealtimeTopic = string

/** `GET /realtime/pulse` : la version de chaque sujet accordé, seule sa variation compte. */
export interface RealtimePulse {
  /** Change au redémarrage de l'API ou à la reprise de son écoute : tout relire. */
  boot: string
  versions: Record<RealtimeTopic, number>
  denied: RealtimeTopic[]
}

/** Événement `ready` du flux, à l'ouverture. */
export type RealtimeReady = RealtimePulse

/** Événement `signal` du flux : les sujets changés, regroupés sur 500 ms. */
export interface RealtimeSignal {
  topics: RealtimeTopic[]
}
