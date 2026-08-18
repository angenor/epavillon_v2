/**
 * Schéma `programme`, partie 3 — sessions programmées, intervenants du jour,
 * co-organisation, rattachement aux journées spéciales, conflits de créneaux.
 * Dérivé de `docs/database/075_programme_sessions.sql` § 1, 2, 2 bis, 2 ter, 7.
 *
 * RÈGLE MÉTIER, la plus facile à trahir en écrivant le planificateur : LES
 * CHEVAUCHEMENTS NE SONT JAMAIS BLOQUÉS. Aucune contrainte d'exclusion n'existe
 * sur les créneaux ; `detect_conflicts()` les recense, l'interface les affiche en
 * permanence, l'équipe arbitre par glisser-déposer. Le seul garde-fou dur se
 * situe à la PUBLICATION du programme (`publication_readiness()`).
 */

import type {
  BroadcastChannelId,
  EventDayId,
  EventId,
  I18nText,
  IsoDateTime,
  OrganizationId,
  PersonId,
  ProposalId,
  RegistrationFormId,
  RoomId,
  SessionId,
  Slug,
  TimeZoneName,
  TrackId,
  TsTzRange,
  Uuid,
} from '../shared'
import type { ParticipationMode } from '../event/edition'
import type { OrganizationRole, SpeakerRole } from './proposal'

// ---------------------------------------------------------------------------
// Sessions
// ---------------------------------------------------------------------------

/** ENUM `programme.session_status`. */
export type SessionStatus =
  | 'planned'
  | 'scheduled'
  | 'live'
  | 'completed'
  | 'postponed'
  | 'cancelled'

/**
 * Table `programme.sessions` — `075` § 1.
 * La colonne `search_vector` (`tsvector`) est omise : sans usage côté client.
 */
export interface Session {
  id: SessionId
  event_id: EventId
  /** Nul quand l'IFDD programme directement une activité, sans appel. */
  proposal_id: ProposalId | null
  /** Déduit de `starts_at` par trigger quand il n'est pas fourni. */
  event_day_id: EventDayId | null
  organization_id: OrganizationId | null
  /** Rang de l'occurrence dans une proposition multi-sessions (cycles de webinaires). */
  sequence_number: number
  title: I18nText
  slug: Slug
  summary: I18nText | null
  description: I18nText | null
  status: SessionStatus
  format: ParticipationMode
  starts_at: IsoDateTime
  ends_at: IsoDateTime
  /** Colonne générée `tstzrange(starts_at, ends_at, '[)')` : support de la
   *  détection des chevauchements et des requêtes de calendrier. */
  readonly time_range: TsTzRange
  timezone: TimeZoneName
  room_id: RoomId | null
  /** Dérivée de `event.rooms.is_virtual` par trigger. Purement indicative : elle
   *  colore un chevauchement, elle ne bloque aucune écriture. */
  readonly enforce_room_exclusivity: boolean
  location_note: I18nText | null
  registration_required: boolean
  registration_opens_at: IsoDateTime | null
  registration_closes_at: IsoDateTime | null
  capacity: number | null
  waitlist_enabled: boolean
  registration_form_id: RegistrationFormId | null
  is_streamed: boolean
  /** Renseigné d'office avec le canal par défaut de l'événement dès que
   *  `is_streamed` passe à vrai : la règle « un seul direct » ne doit pas
   *  dépendre d'une saisie. Remis à nul quand la diffusion est retirée. */
  broadcast_channel_id: BroadcastChannelId | null
  is_recorded: boolean
  allows_questions: boolean
  /** Compte rendu post-activité. */
  report: I18nText | null
  report_submitted_at: IsoDateTime | null
  attendee_count: number | null
  view_count: number
  published_at: IsoDateTime | null
  /** Obligatoire quand `status` vaut `cancelled` (`ck_sessions_cancelled_reason`). */
  cancelled_reason: I18nText | null
  created_by: PersonId | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// Intervenants et organisations du jour
// ---------------------------------------------------------------------------

/**
 * Table `programme.session_speakers` — `075` § 2.
 * Recopiés depuis la proposition à la programmation, puis modifiables : les
 * intervenants annoncés dans le dossier ne sont pas toujours ceux du jour.
 */
export interface SessionSpeaker {
  id: Uuid
  session_id: SessionId
  person_id: PersonId
  role: SpeakerRole
  job_title_snapshot: string | null
  organization_snapshot: string | null
  bio: I18nText | null
  confirmed_at: IsoDateTime | null
  /** Renseigné après coup ; nul tant que la présence n'a pas été constatée. */
  attended: boolean | null
  sort_order: number
  created_at: IsoDateTime
}

/** Table `programme.session_organizations` — `075` § 2 ter. */
export interface SessionOrganization {
  session_id: SessionId
  organization_id: OrganizationId
  role: OrganizationRole
  sort_order: number
  added_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// Composition des journées spéciales
// ---------------------------------------------------------------------------

/**
 * Table `programme.session_tracks` — `075` § 2 bis.
 *
 * Le rattachement d'une session à une journée spéciale est un CHOIX ÉDITORIAL de
 * l'IFDD, jamais une déduction faite sur les dates. On trace qui a rattaché
 * quoi : il arrive d'avoir à l'expliquer à une organisation qui s'étonne de ne
 * pas y figurer.
 */
export interface SessionTrack {
  session_id: SessionId
  track_id: TrackId
  /** Ordre dans la page du fil : le déroulé d'une journée spéciale n'est pas
   *  toujours l'ordre chronologique brut. */
  sort_order: number
  is_highlight: boolean
  added_by: PersonId | null
  added_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// Conflits de créneaux
// ---------------------------------------------------------------------------

/**
 * Gravité d'un conflit.
 * `blocking` — matériellement impossible : un seul stand par édition, un seul
 * direct sur la plateforme, une salle physique à la fois. À régler avant
 * publication. `warning` — gênant mais possible : l'équipe juge.
 */
export type ConflictSeverity = 'blocking' | 'warning'

/** Nature du conflit remonté par `detect_conflicts()`. */
export type ConflictKind = 'venue_capacity' | 'broadcast' | 'room' | 'speaker' | 'organization'

/**
 * Ligne de `programme.detect_conflicts(event_id)` — `075` § 7.
 * Aucun de ces conflits n'empêche l'écriture : ils alimentent l'écran
 * d'arbitrage et le bandeau d'alerte du planificateur.
 */
export interface ScheduleConflict {
  severity: ConflictSeverity
  conflict_kind: ConflictKind
  /** Ressource en cause : l'édition, le canal, la salle, la personne, l'organisation. */
  subject_id: Uuid | null
  subject_label: string | null
  session_a: SessionId
  session_a_title: string | null
  session_b: SessionId
  session_b_title: string | null
  /** Intersection des deux créneaux. */
  overlap: TsTzRange
}

/**
 * Ligne de `programme.publication_readiness(event_id)` — `075` § 7.
 * Ce qui doit être réglé avant de rendre la programmation publique : le seul
 * moment où un contrôle bloquant a du sens.
 */
export interface PublicationReadinessIssue {
  severity: ConflictSeverity
  /** Libellé du problème, déjà rédigé en français par la base. */
  issue: string
  detail: string | null
  session_id: SessionId | null
  /**
   * Quand cela se produit — début du chevauchement, ou de la séance en cause.
   *
   * UN INSTANT, PAS UN TEXTE. La première version du modèle glissait le
   * `tstzrange` brut dans `detail`, qui s'affichait tel quel à l'écran ; une
   * chaîne figée en base ne peut ni se traduire, ni se situer dans le fuseau de
   * l'édition, alors que la règle du projet l'exige de toute date affichée.
   */
  occurs_at: IsoDateTime | null
}

// ---------------------------------------------------------------------------
// Historique
// ---------------------------------------------------------------------------

/** Ligne de `programme.session_history(session_id)` — `075` § 7 bis. */
export interface SessionHistoryEntry {
  occurred_at: IsoDateTime
  actor_id: PersonId | null
  actor_label: string | null
  action: string
  field: string | null
  old_value: unknown
  new_value: unknown
}

/**
 * Ligne de `programme.session_schedule_history(session_id)` — `075` § 7 bis.
 * Les reports de créneau ont leur propre lecture : c'est la modification la plus
 * lourde de conséquences pour les inscrits, et celle qu'on doit pouvoir retracer
 * quand une organisation conteste un horaire.
 */
export interface SessionScheduleChange {
  occurred_at: IsoDateTime
  actor_label: string | null
  previous_start: IsoDateTime | null
  new_start: IsoDateTime | null
  previous_end: IsoDateTime | null
  new_end: IsoDateTime | null
}
