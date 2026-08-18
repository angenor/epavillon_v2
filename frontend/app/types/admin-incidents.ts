/**
 * MESSAGES D'INCIDENT (A13) — contrats de l'écran de back-office.
 *
 * Dérivé de `docs/database/080_live.sql` § 5 : `live.incidents`,
 * `live.event_incidents()`, `live.publish_incident()`,
 * `live.unpublish_incident()`. Aucun champ inventé — ce que le formulaire écrit
 * est une colonne de `live.incidents`, et ce que la liste affiche est une
 * colonne de `live.event_incidents()`.
 *
 * ── CE QUI GOUVERNE L'ÉCRAN ─────────────────────────────────────────────────
 *
 * UN INCIDENT NE SE LIT PAS À SA SEULE EXISTENCE. Quatre conditions cumulées
 * décident qu'un bandeau parle : publié, non dépublié, fenêtre ouverte, portée
 * concernée. La v1 les oubliait une par une — elle n'avait qu'un booléen
 * `is_active` basculé à la main, donc laissé allumé des mois après l'incident.
 * L'état est calculé UNE FOIS, par `live.event_incidents()`, et l'interface ne
 * le recompose jamais : elle lit `state`.
 *
 * LA PORTÉE ET LA CIBLE NE PEUVENT PAS DIVERGER. `ck_incidents_scope_target`
 * impose exactement une cible renseignée par portée, et aucune pour `global`.
 * Le formulaire tient cette règle par construction : choisir une portée efface
 * les cibles des autres.
 *
 * LE TEXTE EST UNE DONNÉE, PAS UNE TRADUCTION. `title` et `message` sont des
 * `platform.i18n_text` saisis par un administrateur : ils vivent en base, jamais
 * dans un fichier i18n. Le prompt exige les deux langues — c'est une règle
 * d'INTERFACE, la base n'exigeant qu'un message non nul.
 */

import type {
  EventDayId,
  EventId,
  I18nText,
  IsoDate,
  IsoDateTime,
  OrganizationId,
  PersonId,
  SessionId,
  TimeZoneName,
  Url,
  Uuid,
} from './shared'
import type { IncidentScope, IncidentSeverity } from './live'
import type { SessionStatus } from './programme/session'
import type { TaxonomyTerm } from './reference'
import type { TemporalState } from './views'

/**
 * L'état d'un message, calculé par `live.event_incidents()`.
 *
 * `expired` et `unpublished` sont deux fins distinctes : la première est venue
 * seule, à l'heure prévue — c'est la correction de la v1 ; la seconde est une
 * décision, tracée avec son auteur et son motif.
 */
export type IncidentState = 'active' | 'scheduled' | 'draft' | 'expired' | 'unpublished'

/** Ligne de `live.event_incidents(event_id, at)` — 080 § 5. */
export interface ManagedIncident {
  incident_id: Uuid
  scope: IncidentScope
  severity: IncidentSeverity
  /** Code de la taxonomie `incident_kind` — vocabulaire ouvert, pas un ENUM. */
  kind_code: string
  title: I18nText | null
  message: I18nText
  action_url: Url | null
  is_dismissible: boolean
  display_from: IsoDateTime
  display_until: IsoDateTime | null
  /** Cible de la portée : séance, journée, organisation ou édition. Nul si global. */
  target_id: Uuid | null
  /** Cible résolue par la fonction — « Atelier de négociation », pas un identifiant. */
  target_label: string | null
  state: IncidentState
  published_at: IsoDateTime | null
  published_by: PersonId | null
  published_by_name: string | null
  unpublished_at: IsoDateTime | null
  unpublished_by_name: string | null
  unpublish_reason: string | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// Ce que le formulaire a le droit de viser
// ---------------------------------------------------------------------------

/**
 * Une cible offerte au choix de portée, déjà résolue pour l'affichage.
 *
 * `hint` est un TEXTE (le sigle d'une organisation, la date civile d'une
 * journée) ; `starts_at` est un INSTANT, que seule l'interface sait afficher —
 * dans le fuseau de l'édition, jamais celui du navigateur. Les mélanger ferait
 * apparaître un « 2027-11-13T09:30:00-03:00 » brut dans une liste déroulante.
 */
export interface IncidentTargetOption {
  id: Uuid
  label: string
  hint: string | null
  /** Début du créneau, pour une séance. Formaté à l'affichage. */
  starts_at: IsoDateTime | null
}

/**
 * LES CIBLES DE L'ÉDITION EN COURS, ET RIEN D'AUTRE — règle métier n° 8.
 *
 * Un administrateur détaché sur la COP31 ne doit pas pouvoir viser une journée
 * d'une autre édition, y compris en forgeant une requête. Les organisations
 * offertes sont celles qui ANIMENT une séance de l'édition : c'est le même
 * critère que la portée `organization` de `live.event_incidents()`.
 */
export interface IncidentTargets {
  event: IncidentTargetOption
  days: IncidentTargetOption[]
  sessions: IncidentTargetOption[]
  organizations: IncidentTargetOption[]
}

// ---------------------------------------------------------------------------
// Le poste de direct
// ---------------------------------------------------------------------------

/**
 * Une activité que le poste de direct surveille — `programme.sessions`, plus
 * l'état temporel que calcule `v_public_schedule`.
 *
 * POURQUOI CE BLOC EXISTE, ET POURQUOI IL EST EN TÊTE D'ÉCRAN. Un message
 * d'incident se rédige presque toujours pendant qu'une activité se tient : la
 * salle attend, l'intervenante ne s'est pas connectée, la diffusion vient de
 * tomber. Demander alors de choisir une portée parmi cinq, une nature parmi
 * neuf et une cible dans une liste de trente activités, c'est demander trois
 * décisions à quelqu'un qui n'a pas trois secondes. Le poste de direct pose la
 * question à l'envers : voici ce qui se joue en ce moment, que se passe-t-il ?
 */
export interface LiveDeskSession {
  session_id: SessionId
  title: I18nText
  starts_at: IsoDateTime
  ends_at: IsoDateTime
  room_name: I18nText | null
  /** Une activité non diffusée n'a pas d'incident de diffusion à signaler. */
  is_streamed: boolean
  status: SessionStatus
  temporal_state: TemporalState
  /** Messages DÉJÀ actifs sur cette activité : ne pas publier deux fois la même panne. */
  active_incident_count: number
}

/**
 * Ce que le poste montre, et de quel jour il parle.
 *
 * `is_fallback` est vrai quand l'édition n'a AUCUNE activité aujourd'hui et que
 * le poste montre les prochaines à la place. L'écran le dit alors en toutes
 * lettres : « rien aujourd'hui » et « voici la suite » ne sont pas la même
 * information, et les confondre ferait croire à un direct en cours hors période.
 */
export interface LiveDesk {
  day: IsoDate
  sessions: LiveDeskSession[]
  is_fallback: boolean
}

// ---------------------------------------------------------------------------
// L'écran de liste
// ---------------------------------------------------------------------------

export interface IncidentFilters {
  search: string
  states: IncidentState[]
  severities: IncidentSeverity[]
  scopes: IncidentScope[]
  kinds: string[]
}

/** Compteurs de la barre d'états — établis AVANT filtrage, comme partout. */
export type IncidentStateCounts = Record<IncidentState, number>

/**
 * TOUT L'ÉCRAN EN UNE RÉPONSE.
 *
 * `timezone` est celui de l'ÉDITION : une fenêtre d'affichage se lit dans le
 * fuseau où l'incident a lieu, jamais dans celui du navigateur de qui publie.
 */
export interface IncidentListScreen {
  event_id: EventId
  event_title: I18nText
  timezone: TimeZoneName
  /** Ville de l'édition — « heure de Belém », et non « heure de America/Belem ». */
  zone_label: string | null
  rows: ManagedIncident[]
  /** Ce qui se joue maintenant — le bloc de tête, voir `LiveDesk`. */
  desk: LiveDesk
  counts: IncidentStateCounts
  /** Termes de la taxonomie `incident_kind`, actifs, dans leur ordre. */
  kinds: TaxonomyTerm[]
  targets: IncidentTargets
}

// ---------------------------------------------------------------------------
// Le formulaire
// ---------------------------------------------------------------------------

/**
 * Ce que le formulaire écrit — les colonnes de `live.incidents`, ni plus ni
 * moins, plus la décision de publier.
 *
 * `publish` À PART DU RESTE : l'enregistrement et la publication sont deux actes
 * distincts en base (`live.publish_incident()` horodate et attribue). Un
 * brouillon se relit avant de parler à toute une COP.
 */
export interface IncidentPayload {
  scope: IncidentScope
  event_id: EventId | null
  event_day_id: EventDayId | null
  session_id: SessionId | null
  organization_id: OrganizationId | null
  incident_kind_code: string
  severity: IncidentSeverity
  title: I18nText | null
  message: I18nText
  action_url: Url | null
  is_dismissible: boolean
  display_from: IsoDateTime
  display_until: IsoDateTime | null
  publish: boolean
}

export interface CreateIncidentPayload extends IncidentPayload {
  /** L'édition depuis laquelle l'écran agit — sert au contrôle de périmètre. */
  from_event_id: EventId
}

export interface UpdateIncidentPayload extends CreateIncidentPayload {
  incident_id: Uuid
}

export interface UnpublishIncidentPayload {
  incident_id: Uuid
  /** Motif, facultatif en base (`unpublish_reason`) mais demandé par l'écran. */
  reason: string | null
}

/**
 * L'issue d'une écriture.
 *
 * Chaque refus DIT QUOI CORRIGER, et chacun traduit une contrainte réelle :
 * `missing_target` traduit `ck_incidents_scope_target`, `invalid_window`
 * traduit `ck_incidents_window`, `not_published` traduit l'exception de
 * `live.unpublish_incident()` sur un message jamais publié, `forbidden` traduit
 * l'absence de `live.incident.publish` sur l'édition visée.
 */
export type IncidentWriteStatus =
  | 'created'
  | 'updated'
  | 'published'
  | 'unpublished'
  | 'missing_target'
  | 'missing_message'
  | 'invalid_window'
  | 'not_published'
  | 'not_found'
  | 'forbidden'

export interface IncidentWriteResult {
  status: IncidentWriteStatus
  incident: ManagedIncident | null
}
