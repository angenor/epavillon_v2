/**
 * Schéma `live` — la part que le jalon consomme réellement : LES MESSAGES
 * D'INCIDENT. Dérivé de `docs/database/080_live.sql` § 5.
 *
 * POURQUOI CE FICHIER EXISTE ALORS QUE LE MODULE DIRECT EST HORS JALON. Le
 * bandeau d'incident est un motif transverse de toute la plateforme : il
 * s'affiche sur la programmation publique, sur la fiche d'une activité et dans
 * le back-office (écran A13), bien avant qu'une visioconférence soit branchée.
 * Les tables `meetings`, `streams` et `provider_webhook_events` ne sont PAS
 * couvertes ici ; elles viendront avec leurs écrans.
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
 * Ligne de `live.active_incidents(session_id, at)` — `080` § 5.
 *
 * La fonction remonte la hiérarchie : incidents de la séance, de sa journée, de
 * son édition, de son organisation porteuse, plus les incidents globaux. Le
 * front n'a donc AUCUN filtre de portée à réimplémenter — il affiche ce qui lui
 * est renvoyé, dans l'ordre où il le reçoit.
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
}
