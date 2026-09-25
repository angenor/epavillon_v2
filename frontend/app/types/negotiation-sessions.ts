/**
 * Les sessions de négociation officielles, « Mon groupe » et « Mon agenda » de
 * Guide Négo. Champ pour champ comme `negotiation/src/domain/{sessions,groups,agenda}.rs` ;
 * contrat dans `specs/014-guide-nego-sessions-agenda/contracts/api-sessions.md`.
 *
 * Toute heure est un instant ISO 8601 en UTC ; le fuseau de la COP voyage à part.
 */

import type { I18nText, IsoDate, IsoDateTime, Uuid } from './shared'

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
  /** Servies aussi quand `state = 'cut'` : elles ne viennent pas de la source. */
  network_meetings: NetworkMeeting[]
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
  /** Publiés, non retirés, session non terminée. Jamais de nom d'autrice. */
  network_reports: NetworkReport[]
}

/** L'encart « Signalé par le réseau — validé par l'IFDD à … ». */
export interface NetworkReport {
  reason: 'cancelled' | 'time' | 'venue' | 'other'
  proposed_start: IsoDateTime | null
  proposed_venue: string | null
  detail: string | null
  validated_at: IsoDateTime
}

/** Une réunion non annoncée, publiée : jamais une session officielle. */
export interface NetworkMeeting {
  id: Uuid
  title: string
  venue: string | null
  /** `null` = sans heure. */
  start_at: IsoDateTime | null
  /** Jour de la liste, dans le fuseau de la COP. */
  day: IsoDate
  /** Code `negotiation_theme`. */
  theme: string | null
  validated_at: IsoDateTime
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
// · `PUT`/`DELETE /negotiation/me/agenda/network/{id}`
// ---------------------------------------------------------------------------

export interface MyAgenda {
  entries: MyAgendaEntry[]
  /** Les réunions non annoncées gardées. */
  network_entries: MyNetworkAgendaEntry[]
}

export interface MyNetworkAgendaEntry {
  network_meeting_id: Uuid
  /** Effectif : faux dès que la réunion est retirée. */
  remind: boolean
  added_at: IsoDateTime
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

// ---------------------------------------------------------------------------
// Back-office — `GET`/`PUT /admin/negotiation/import?edition=`
// ---------------------------------------------------------------------------

export type ImportReader = 'archive' | 'live'

export interface OfficialImportAdmin {
  edition: { slug: string; name: I18nText; timezone: string }
  enabled: boolean
  reader: ImportReader
  archive_name: string | null
  /** AAAA-MM-JJ : le premier jour de l'archive est posé sur ce jour. */
  archive_first_day: IsoDate | null
  /** Les jeux archivés présents dans le binaire. */
  archives: string[]
  live_url: string | null
  time_correction_minutes: number
  official_programme_url: string
  interval_seconds: number
  missed_threshold: number
  missed_reads: number
  /** `negotiation.import_is_serving()` — la règle de coupure, lue et non recopiée. */
  serving: boolean
  last_success_at: IsoDateTime | null
  last_attempt_at: IsoDateTime | null
  last_error: string | null
  failing_since: IsoDateTime | null
  last_change_count: number | null
  session_count: number
  agenda_items_without_theme: number
  /** Les vingt dernières lectures, la plus récente d'abord. */
  runs: ImportRun[]
}

export interface ImportRun {
  started_at: IsoDateTime
  outcome: 'success' | 'failure'
  error: string | null
  session_count: number | null
  change_count: number | null
  /** « Lire maintenant », et non la chaîne récurrente. */
  manual: boolean
}

export type UpdateOfficialImportPayload = Pick<
  OfficialImportAdmin,
  | 'enabled'
  | 'reader'
  | 'archive_name'
  | 'archive_first_day'
  | 'live_url'
  | 'time_correction_minutes'
  | 'official_programme_url'
  | 'interval_seconds'
  | 'missed_threshold'
>

// ---------------------------------------------------------------------------
// Back-office — `GET /admin/negotiation/agenda-items?edition=` · `PUT …/{id}`
// ---------------------------------------------------------------------------

export interface AgendaItemAdmin {
  id: Uuid
  code: string
  title: string
  /** Code `negotiation_theme` ; son libellé vient du vocabulaire. */
  theme: string | null
  session_count: number
  theme_set_at: IsoDateTime | null
}

export interface AgendaItemThemePayload {
  theme: string | null
}
