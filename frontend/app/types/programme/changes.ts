/**
 * Ce qui a changé dans le programme publié d'une édition :
 * `GET /events/{event_id}/programme-changes`, sans session.
 *
 * Lu dans `platform.audit_log` (010 § 2) sur les lignes de `programme.sessions`
 * (075 § 1) : les quinze derniers jours, le plus récent d'abord, seulement ce
 * qui est arrivé à une séance pendant qu'elle était publique. La première
 * publication du programme n'y figure pas. Aucun auteur.
 */

import type { I18nText, IsoDateTime, SessionId } from '../shared'

/** `rescheduled` couvre l'horaire et la date (`day_changed`). */
export type ProgrammeChangeKind = 'rescheduled' | 'retitled' | 'cancelled' | 'postponed' | 'added'

export interface ProgrammeChange {
  occurred_at: IsoDateTime
  kind: ProgrammeChangeKind
  session_id: SessionId
  slug: string
  /** Titre courant de l'activité. */
  title: I18nText
  /** Créneau juste après le changement. */
  starts_at: IsoDateTime
  ends_at: IsoDateTime
  /** Créneau juste avant ; nul pour `added`. */
  previous_starts_at: IsoDateTime | null
  previous_ends_at: IsoDateTime | null
  /** Ancien et nouveau titre d'un `retitled` ; nuls sinon. */
  previous_title: I18nText | null
  new_title: I18nText | null
  /** Un `rescheduled` qui change de jour civil dans le fuseau de la séance. */
  day_changed: boolean
}
