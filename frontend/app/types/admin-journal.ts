/**
 * JOURNAL DU BACK-OFFICE (`/admin/journal`) : contrats des routes `/admin/journal/*`.
 *
 * Dérivé de `docs/database/010_platform.sql` § 2 bis (masquage de l'audit),
 * § 2 ter (`platform.ops_log`), § 3 (`platform.outbox_events`) et § 4
 * (`platform.jobs`, `platform.retry_job()`). Réservé à `identity.platform.journal`
 * en portée globale ; relancer exige en plus `identity.platform.maintenance`.
 *
 * Trois sources, jamais confondues :
 * - `JournalEntry` : ce qui a échoué ou traîné (`platform.ops_log`, trois mois) ;
 * - `JournalChange` : ce qui a été écrit (`platform.audit_log`) ;
 * - `JournalFailures` : ce qui attend encore (travaux et événements en échec, lus en direct).
 * Le `request_id` les relie : `JournalRequestTrace` rassemble les trois pour une demande.
 */

import type { IsoDateTime, Uuid } from './shared'

/** `http_error` : panne (5xx) ; `http_refused` : refus retenu ; `http_slow` : plus d'une seconde ; `job_failed` : essai d'un travail en échec. */
export type JournalKind = 'http_error' | 'http_refused' | 'http_slow' | 'job_failed'
export type JournalSource = 'api' | 'worker'
export type JournalLevel = 'error' | 'warn' | 'info'
/** Fenêtre des listes, comptée depuis maintenant côté serveur. */
export type JournalPeriod = '24h' | '7d' | '30d' | '90d'

/** La personne derrière une ligne, et son organisation principale si elle en a une. */
export interface JournalPerson {
  id: Uuid
  display_name: string
  organization_name: string | null
}

/** Une ligne de `platform.ops_log`. */
export interface JournalEntry {
  id: Uuid
  occurred_at: IsoDateTime
  source: JournalSource
  kind: JournalKind
  level: JournalLevel
  request_id: string | null
  /** Nul : visiteur sans session, ou travail de fond. */
  actor: JournalPerson | null
  method: string | null
  /** Modèle de route (`/api/admin/proposals/{id}`), jamais le chemin reçu ; `*` si aucune route ne correspond. */
  route: string | null
  status: number | null
  duration_ms: number | null
  /** Code stable de l'erreur d'API (`FORBIDDEN`…). */
  error_code: string | null
  /** Ce qui s'est passé, en français : message du catalogue pour un refus, détail nettoyé pour une panne ou un travail. */
  message: string | null
  task: string | null
  job_id: Uuid | null
}

export interface JournalEntryFilters {
  period?: JournalPeriod
  kind?: JournalKind
  source?: JournalSource
  /** Personne, route, code d'erreur ou code de la demande. */
  q?: string
  page?: number
  per_page?: number
}

export interface JournalEntryPage {
  items: JournalEntry[]
  total: number
  page: number
  per_page: number
}

/** Les quatre chiffres de l'en-tête (24 dernières heures) et l'état des files. */
export interface JournalSummary {
  since: IsoDateTime
  http_error: number
  http_refused: number
  http_slow: number
  job_failed: number
  /** Travaux morts, en échec ou en reprise : ce qu'on peut relancer. */
  failing_jobs: number
  /** Événements de l'outbox non relayés après au moins un essai. */
  failing_events: number
  /** Lignes perdues par ce processus de l'API depuis son démarrage (canal plein ou écriture refusée). */
  dropped: number
  retention_months: number
}

/** Un champ modifié. `masked` : valeur sensible, jamais rendue (`before` et `after` sont alors nuls). */
export interface JournalChangeField {
  name: string
  before: unknown
  after: unknown
  masked: boolean
}

export type JournalChangeAction = 'insert' | 'update' | 'delete'

/** Une ligne de `platform.audit_log`, valeurs sensibles retirées. */
export interface JournalChange {
  id: Uuid
  occurred_at: IsoDateTime
  /** `actor_label` est figé à l'écriture : il reste lisible après anonymisation. */
  actor: JournalPerson | null
  request_id: string | null
  entity_schema: string
  entity_table: string
  entity_id: Uuid | null
  action: JournalChangeAction
  /** Champs modifiés d'une mise à jour (hors `updated_at`, `search_vector`) ; vide pour une création ou une suppression. Textes longs tronqués. */
  fields: JournalChangeField[]
}

export interface JournalChangeFilters {
  period?: JournalPeriod
  /** `schema.table`, par exemple `programme.proposals`. */
  entity?: string
  action?: JournalChangeAction
  /** Auteur ou identifiant de l'objet. */
  q?: string
  page?: number
  per_page?: number
}

export interface JournalChangePage {
  items: JournalChange[]
  total: number
  page: number
  per_page: number
}

/** Un événement de `platform.outbox_events`, sans sa charge utile. */
export interface JournalOutboxEvent {
  id: Uuid
  event_type: string
  aggregate_schema: string
  aggregate_type: string
  aggregate_id: Uuid
  occurred_at: IsoDateTime
  published_at: IsoDateTime | null
  available_at: IsoDateTime
  attempts: number
  last_error: string | null
}

/** Tout ce qui porte ce code de demande, chaque liste dans l'ordre chronologique. */
export interface JournalRequestTrace {
  request_id: string
  entries: JournalEntry[]
  changes: JournalChange[]
  events: JournalOutboxEvent[]
}

/** `dead` : abandonné ; `failed` : en échec ; `retrying` : remis en file, nouvel essai à `run_at`. */
export type JournalJobState = 'dead' | 'failed' | 'retrying'

/** Un travail de `platform.jobs` en échec, sans sa charge utile. */
export interface JournalFailedJob {
  id: Uuid
  queue: string
  task: string
  state: JournalJobState
  attempts: number
  max_attempts: number
  last_error: string | null
  /** Dernier échec consigné dans le journal, s'il y en a un dans les trois mois. */
  last_failed_at: IsoDateTime | null
  run_at: IsoDateTime
  created_at: IsoDateTime
}

export interface JournalFailures {
  jobs: JournalFailedJob[]
  events: JournalOutboxEvent[]
}
