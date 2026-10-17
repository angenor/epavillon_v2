/**
 * Schéma `live` — la part que le jalon consomme réellement : LES MESSAGES
 * D'INCIDENT. Dérivé de `docs/database/080_live.sql` § 5.
 *
 * POURQUOI CE FICHIER EXISTE ALORS QUE LE MODULE DIRECT EST HORS JALON. Le
 * bandeau d'incident est un motif transverse de toute la plateforme : il
 * s'affiche sur la programmation publique, sur la fiche d'une activité et dans
 * le back-office (écran A13), bien avant qu'une visioconférence soit branchée.
 * De `streams`, seule la lecture publique des flux en cours est couverte ; de
 * `meetings`, la réunion Zoom d'une séance (fin du fichier).
 */

import type {
  EventDayId,
  EventId,
  I18nText,
  IsoDateTime,
  OrganizationId,
  PersonId,
  SessionId,
  Url,
  Uuid,
} from './shared'

/**
 * ENUM `live.incident_severity`.
 *
 * L'ordre de déclaration est croissant en gravité — voulu côté base, pour que
 * `ORDER BY severity DESC` remonte le plus grave en premier. L'interface doit
 * conserver cet ordre : le bandeau affiche l'incident le plus grave en tête.
 */
export type IncidentSeverity = 'info' | 'warning' | 'error' | 'critical'

/** Ordre de gravité croissante, tel que le déclare l'ENUM. */
export const INCIDENT_SEVERITY_ORDER: readonly IncidentSeverity[] = [
  'info',
  'warning',
  'error',
  'critical',
] as const

/**
 * ENUM `live.incident_scope`. La portée et la cible ne peuvent pas diverger :
 * `ck_incidents_scope_target` impose exactement une cible renseignée par portée,
 * et aucune pour `global`.
 */
export type IncidentScope = 'global' | 'event' | 'event_day' | 'session' | 'organization'

/** Table `live.incidents` — `080` § 5. */
export interface Incident {
  id: Uuid
  scope: IncidentScope
  event_id: EventId | null
  event_day_id: EventDayId | null
  session_id: SessionId | null
  organization_id: OrganizationId | null
  /** Code de la taxonomie `incident_kind` — vocabulaire ouvert, pas un ENUM. */
  incident_kind_code: string
  severity: IncidentSeverity
  title: I18nText | null
  message: I18nText
  action_url: Url | null
  /** Bandeau refermable par le visiteur, ou permanent tant qu'il est publié. */
  is_dismissible: boolean
  display_from: IsoDateTime
  display_until: IsoDateTime | null
  published_at: IsoDateTime | null
  published_by: PersonId | null
  unpublished_at: IsoDateTime | null
  unpublished_by: PersonId | null
  unpublish_reason: string | null
  created_by: PersonId | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

/**
 * Ligne de `live.active_incidents(session_id, at)` et de
 * `live.active_incidents_for_event(event_id, at)` — `080` § 6.
 *
 * Les deux fonctions balaient les MÊMES cinq portées, dans deux sens opposés :
 * la première REMONTE depuis une activité (sa journée, son édition, son
 * organisation porteuse, plus les messages globaux), la seconde DESCEND depuis
 * une édition. Le front n'a donc AUCUN filtre de portée à réimplémenter — il
 * affiche ce qui lui est renvoyé, dans l'ordre où il le reçoit, le plus grave en
 * tête.
 *
 * LA CIBLE EST RÉSOLUE PAR LE MODÈLE, et elle compte : sur la page des
 * programmations, qui parle de trente activités, « la diffusion est
 * interrompue » ne dit pas laquelle. `target_label` le dit — « Atelier de
 * négociation », « Journée finance », le nom légal d'une organisation. Elle est
 * nulle pour un message global, qui ne vise rien de particulier, et pour la
 * fonction ascendante, qui parle d'une activité déjà nommée par la page.
 */
export interface ActiveIncident {
  incident_id: Uuid
  scope: IncidentScope
  severity: IncidentSeverity
  kind_code: string
  title: I18nText | null
  message: I18nText
  action_url: Url | null
  is_dismissible: boolean
  display_from: IsoDateTime
  display_until: IsoDateTime | null
  /** Cible de la portée. Nulle pour un message global. */
  target_id?: Uuid | null
  /** Cible résolue par la fonction — jamais un identifiant. */
  target_label?: string | null
}

/** ENUM `live.stream_provider`. */
export type StreamProvider = 'youtube' | 'vimeo' | 'facebook' | 'linkedin' | 'dailymotion' | 'custom'

/**
 * Ligne de `live.current_streams` pour une séance publiée — `GET
 * /sessions/{id}/streams`. Un flux par langue d'interprétation, le principal
 * en tête. `embed_url` est construite par `live.build_embed_url()`, à défaut
 * `watch_url` : le site ne recompose jamais l'adresse d'un diffuseur.
 */
export interface PublicSessionStream {
  id: Uuid
  /** Code de `reference.locales` ; nul quand la langue n'est pas renseignée. */
  locale: string | null
  provider: StreamProvider
  embed_url: Url | null
  watch_url: Url | null
  is_primary: boolean
  started_at: IsoDateTime | null
}

/** ENUM `live.meeting_provider` : seule `zoom` s'ouvre dans la page. */
export type MeetingProvider = 'zoom' | 'teams' | 'jitsi' | 'webex' | 'custom'

/** ENUM `live.meeting_status`. */
export type MeetingStatus = 'draft' | 'scheduled' | 'started' | 'ended' | 'cancelled' | 'failed'

/** ENUM `live.meeting_sync_status` : la synchronisation des inscrits vers le fournisseur. */
export type MeetingSyncStatus = 'not_required' | 'pending' | 'in_progress' | 'synced' | 'failed' | 'abandoned'

/**
 * `GET /sessions/{id}/meeting` : la réunion d'une séance, sans aucun secret.
 * 404 quand la séance n'en a pas. La salle s'ouvre à `opens_at` (début moins
 * 30 minutes) et ferme à `closes_at` : la fin réelle quand Zoom dit la réunion
 * terminée (`ended`), sinon fin prévue plus 4 heures, repoussée par l'API tant que
 * la réunion dure. À relire régulièrement.
 */
export interface PublicSessionMeeting {
  provider: MeetingProvider
  status: MeetingStatus
  opens_at: IsoDateTime
  starts_at: IsoDateTime
  ends_at: IsoDateTime
  closes_at: IsoDateTime
}

/**
 * `GET /sessions/{id}/meeting/access` : de quoi entrer, pour la personne
 * connectée et inscrite (ou en liste d'attente). La signature expire à
 * `expires_at` : au-delà, on redemande.
 */
export interface MeetingAccess {
  meeting_number: string
  password: string
  /** Jeton d'inscrit du fournisseur ; nul quand la réunion n'inscrit pas. */
  registrant_token: string | null
  user_name: string
  user_email: string
  sdk_key: string
  signature: string
  expires_at: IsoDateTime
  /** Lien d'entrée par l'application Zoom, en repli. */
  join_url: Url | null
}

/** `POST /admin/sessions/{id}/meeting` : crée ou resynchronise ; `external_id` rattache une réunion existante. */
export interface MeetingSyncInput {
  external_id: string | null
}

/** `GET` et `POST /admin/sessions/{id}/meeting`. */
export interface AdminSessionMeeting {
  id: Uuid
  session_id: SessionId
  provider: MeetingProvider
  /** Numéro de réunion chez le fournisseur. */
  external_id: string | null
  status: MeetingStatus
  sync_status: MeetingSyncStatus
  last_sync_error: string | null
  join_url: Url | null
  participants_total: number
  participants_synced: number
  participants_failed: number
}

/** `POST /registrations/{id}/join` : la première présence, écrite une seule fois. */
export interface RegistrationJoinResult {
  joined_at: IsoDateTime | null
}
