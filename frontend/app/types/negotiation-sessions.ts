/**
 * Les sessions de négociation officielles, « Mon groupe » et « Mon agenda » de
 * Guide Négo. Champ pour champ comme `negotiation/src/domain/{sessions,groups,agenda}.rs` ;
 * contrat dans `specs/014-guide-nego-sessions-agenda/contracts/api-sessions.md`.
 *
 * Toute heure est un instant ISO 8601 en UTC ; le fuseau de la COP voyage à part.
 */

import type { I18nText, IsoDateTime, Uuid } from './shared'

// ---------------------------------------------------------------------------
// `GET /negotiation/sessions?edition=`
// ---------------------------------------------------------------------------

export interface OfficialSessions {
  edition: { slug: string; timezone: string; city: string | null }
  official_programme_url: string
  state: 'serving' | 'cut'
  /** `disabled` : import éteint ou jamais lu. */
  cut_reason: 'disabled' | 'unreachable' | null
  failing_since: IsoDateTime | null
  /** Dernière lecture réussie. */
  read_at: IsoDateTime | null
  server_time: IsoDateTime
  /** Vide quand `state = 'cut'`. */
  sessions: OfficialSession[]
}

export interface OfficialSession {
  id: Uuid
  title_en: string
  /** Traduction automatique ; `null` = anglais seul. */
  title_fr: string | null
  start_at: IsoDateTime
  end_at: IsoDateTime | null
  venue: string | null
  /** N'existe que si l'heure ou la salle a changé : « Déplacée ». */
  previous: OfficialSessionPrevious | null
  type: { code: string; label: I18nText; term_en: string } | null
  /** Coordination seulement. */
  group: { code: string; label: I18nText } | null
  /** Code `negotiation_theme` ; `null` = « Thématique non précisée ». */
  theme: string | null
  agenda_item: { code: string; title: string } | null
  open_access: boolean | null
  status: 'scheduled' | 'cancelled'
  cancelled: { at: IsoDateTime; reason: 'source' | 'postponed' | 'removed' } | null
  source_url: string | null
  /** Dernière lecture où elle figurait. */
  read_at: IsoDateTime
}

export interface OfficialSessionPrevious {
  start_at: IsoDateTime
  end_at: IsoDateTime | null
  venue: string | null
  changed_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// `GET`/`PUT /negotiation/me/groups`
// ---------------------------------------------------------------------------

export interface MyGroups {
  groups: string[]
  etag: string
}

/** La liste entière ; vide permise — « aucun groupe » est un choix. */
export interface GroupsPayload {
  groups: string[]
}

// ---------------------------------------------------------------------------
// `GET /negotiation/me/agenda` · `PUT`/`DELETE /negotiation/me/agenda/{session_id}`
// ---------------------------------------------------------------------------

export interface MyAgenda {
  entries: MyAgendaEntry[]
}

export interface MyAgendaEntry {
  session_id: Uuid
  /** Effectif : faux dès que la session est annulée. */
  remind: boolean
  added_at: IsoDateTime
}

export interface AgendaEntryPayload {
  remind: boolean
}
