/**
 * Journal du back-office, hors ligne : erreurs, refus et lenteurs de
 * `platform.ops_log`, modifications de `platform.audit_log`, travaux et
 * événements en échec. Les heures sont recalées sur maintenant, pour que la
 * fenêtre des 24 heures ait toujours quelque chose à montrer.
 *
 * Trois demandes se recoupent d'une source à l'autre (`REQUEST.draft`,
 * `REQUEST.decision`, `REQUEST.merge`) : c'est ce que le détail « Même demande,
 * dans l'ordre » doit savoir rassembler.
 */

import type {
  JournalChange,
  JournalChangeFilters,
  JournalChangePage,
  JournalEntry,
  JournalEntryFilters,
  JournalEntryPage,
  JournalFailedJob,
  JournalFailures,
  JournalOutboxEvent,
  JournalPeriod,
  JournalPerson,
  JournalRequestTrace,
  JournalSummary,
} from '~/types/admin-journal'
import type { IsoDateTime } from '~/types/shared'
import { ApiRequestError } from '~/utils/api-error'
import {
  JOURNAL_CHANGE,
  JOURNAL_ENTRY,
  JOURNAL_EVENT,
  JOURNAL_JOB,
  ORG,
  PERSON,
  PROPOSAL,
  SESSION,
  mockUuid,
} from './ids'

const REQUEST = {
  draft: mockUuid('7124', 1),
  draftRefused: mockUuid('7124', 2),
  slowSchedule: mockUuid('7124', 3),
  login: mockUuid('7124', 4),
  outOfScope: mockUuid('7124', 5),
  refresh: mockUuid('7124', 6),
  decision: mockUuid('7124', 7),
  merge: mockUuid('7124', 8),
  slowExport: mockUuid('7124', 9),
  upload: mockUuid('7124', 10),
  incident: mockUuid('7124', 11),
} as const

const WHO = {
  perret: { id: PERSON.perretAdmin, display_name: 'Claire Perret', organization_name: 'IFDD' },
  bakayoko: { id: PERSON.bakayoko, display_name: 'Aminata Bakayoko', organization_name: 'IFDD' },
  kabore: { id: PERSON.kabore, display_name: 'Alizeta Kaboré', organization_name: 'OSED Sahel' },
  sowFall: { id: PERSON.sowFall, display_name: 'Awa Sow Fall', organization_name: 'ROAC' },
  lemoine: { id: PERSON.lemoine, display_name: 'Hélène Lemoine', organization_name: 'CUDCM' },
  admin: { id: PERSON.adminPivot, display_name: 'Administration ePavillon', organization_name: null },
} satisfies Record<string, JournalPerson>

const MINUTE = 60_000

/** Un instant, en minutes avant maintenant. Calculé à la lecture : la page reste « fraîche » sans rechargement du jeu. */
function ago(minutes: number, ms = 0): IsoDateTime {
  return new Date(Date.now() - minutes * MINUTE + ms).toISOString()
}

function entry(n: number, minutes: number, fields: Partial<JournalEntry> & Pick<JournalEntry, 'kind'>): JournalEntry {
  return {
    id: JOURNAL_ENTRY(n),
    occurred_at: ago(minutes),
    source: 'api',
    level: fields.kind === 'http_error' || fields.kind === 'job_failed' ? 'error' : fields.kind === 'http_refused' ? 'warn' : 'info',
    request_id: null,
    actor: null,
    method: null,
    route: null,
    status: null,
    duration_ms: null,
    error_code: null,
    message: null,
    task: null,
    job_id: null,
    ...fields,
  }
}

function buildEntries(): JournalEntry[] {
  return [
    entry(1, 26, {
      kind: 'http_error', request_id: REQUEST.draft, actor: WHO.kabore,
      method: 'PUT', route: '/api/proposals/{id}/draft', status: 500, duration_ms: 214, error_code: 'INTERNAL',
      message: "Erreur interne : la base a refusé l'écriture (ck_proposal_speakers_complete).",
    }),
    entry(2, 29, {
      kind: 'http_refused', request_id: REQUEST.draftRefused, actor: WHO.kabore,
      method: 'PUT', route: '/api/proposals/{id}/draft', status: 422, duration_ms: 38, error_code: 'PROPOSAL_TEXT_TOO_LONG',
      message: 'Ce texte dépasse la longueur autorisée : 1 200 caractères au maximum.',
    }),
    entry(3, 62, {
      kind: 'http_slow', request_id: REQUEST.slowSchedule,
      method: 'GET', route: '/api/events/{slug}/schedule', status: 200, duration_ms: 3412,
      message: '3,4 s pour répondre.',
    }),
    entry(4, 100, {
      kind: 'http_refused', request_id: REQUEST.login,
      method: 'POST', route: '/api/auth/login', status: 429, duration_ms: 12, error_code: 'FORBIDDEN',
      message: 'Compte verrouillé 15 minutes après 5 essais.',
    }),
    entry(5, 178, {
      kind: 'http_refused', request_id: REQUEST.outOfScope, actor: WHO.perret,
      method: 'GET', route: '/api/admin/organizations/{id}', status: 403, duration_ms: 9, error_code: 'FORBIDDEN',
      message: "Cette organisation n'est pas dans votre périmètre d'administration.",
    }),
    entry(6, 236, {
      kind: 'job_failed', source: 'worker', task: 'engagement.notification_email', job_id: JOURNAL_JOB(1),
      message: 'Le relais de courriel ne répond pas (délai dépassé).',
    }),
    entry(7, 290, {
      kind: 'job_failed', source: 'worker', task: 'live.sync_meeting', job_id: JOURNAL_JOB(2),
      message: 'Zoom : limite de requêtes atteinte (429).',
    }),
    entry(8, 560, {
      kind: 'http_error', request_id: REQUEST.refresh, actor: WHO.lemoine,
      method: 'POST', route: '/api/auth/refresh', status: 500, duration_ms: 51, error_code: 'IDENTITY_REFRESH_REUSED',
      message: 'Jeton de reconnexion présenté deux fois : sessions coupées.',
    }),
    entry(9, 1_210, {
      kind: 'http_slow', request_id: REQUEST.decision, actor: WHO.bakayoko,
      method: 'POST', route: '/api/admin/proposals/{id}/decision', status: 200, duration_ms: 1_284,
      message: '1,3 s pour répondre.',
    }),
    entry(10, 1_900, {
      kind: 'http_slow', request_id: REQUEST.slowExport, actor: WHO.bakayoko,
      method: 'GET', route: '/api/admin/proposals', status: 200, duration_ms: 2_045,
      message: '2,0 s pour répondre.',
    }),
    entry(11, 3_100, {
      kind: 'http_refused', request_id: REQUEST.upload, actor: WHO.sowFall,
      method: 'POST', route: '/api/media/uploads', status: 413, duration_ms: 140, error_code: 'MEDIA_TOO_LARGE',
      message: 'Ce fichier pèse plus de 10 Mo, la limite pour une image.',
    }),
    entry(12, 5_800, {
      kind: 'http_error', request_id: REQUEST.incident, actor: WHO.perret,
      method: 'POST', route: '/api/admin/incidents', status: 503, duration_ms: 30_002, error_code: 'SERVICE_UNAVAILABLE',
      message: 'Délai dépassé en attendant la base.',
    }),
    entry(13, 14_000, {
      kind: 'job_failed', source: 'worker', task: 'media.process_asset', job_id: JOURNAL_JOB(9),
      message: 'Image illisible : format non reconnu.',
    }),
    entry(14, 50_000, {
      kind: 'http_error', actor: null,
      method: 'GET', route: '*', status: 500, duration_ms: 4, error_code: 'INTERNAL',
      message: 'Erreur interne : réponse du serveur de fichiers incomplète.',
    }),
  ]
}

function change(n: number, minutes: number, fields: Omit<JournalChange, 'id' | 'occurred_at'>, ms = 0): JournalChange {
  return { id: JOURNAL_CHANGE(n), occurred_at: ago(minutes, ms), ...fields }
}

function buildChanges(): JournalChange[] {
  return [
    change(1, 23, {
      actor: WHO.kabore, request_id: null, entity_schema: 'programme', entity_table: 'proposals',
      entity_id: PROPOSAL.adaptationCotiere, action: 'update',
      fields: [{ name: 'detailed_description', before: 'Une présentation en trois temps…', after: 'Une présentation en quatre temps, avec un retour du terrain…', masked: false }],
    }),
    change(2, 45, {
      actor: WHO.kabore, request_id: null, entity_schema: 'programme', entity_table: 'sessions',
      entity_id: SESSION.adaptationCotiere, action: 'update',
      fields: [
        { name: 'summary', before: 'Ateliers côtiers', after: 'Ateliers côtiers et retours de Saint-Louis', masked: false },
        { name: 'objectives', before: null, after: 'Partager trois méthodes de suivi du trait de côte.', masked: false },
      ],
    }),
    change(3, 1_210, {
      actor: WHO.bakayoko, request_id: REQUEST.decision, entity_schema: 'programme', entity_table: 'proposals',
      entity_id: PROPOSAL.alertePrecoce, action: 'update',
      fields: [{ name: 'status', before: 'under_review', after: 'accepted', masked: false }],
    }, 120),
    change(4, 1_210, {
      actor: WHO.bakayoko, request_id: REQUEST.decision, entity_schema: 'programme', entity_table: 'sessions',
      entity_id: SESSION.alertePrecoce, action: 'insert', fields: [],
    }, 340),
    change(5, 2_400, {
      actor: WHO.admin, request_id: REQUEST.merge, entity_schema: 'org', entity_table: 'organizations',
      entity_id: ORG.osedSigle, action: 'delete', fields: [],
    }),
    change(6, 2_400, {
      actor: WHO.admin, request_id: REQUEST.merge, entity_schema: 'org', entity_table: 'organizations',
      entity_id: ORG.osed, action: 'update',
      fields: [{ name: 'acronym', before: null, after: 'OSED', masked: false }],
    }, 80),
    change(7, 4_300, {
      actor: WHO.lemoine, request_id: null, entity_schema: 'identity', entity_table: 'people',
      entity_id: PERSON.lemoine, action: 'update',
      fields: [
        { name: 'phone', before: null, after: null, masked: true },
        { name: 'job_title', before: 'Professeure', after: 'Professeure titulaire', masked: false },
      ],
    }),
    change(8, 9_000, {
      actor: WHO.perret, request_id: null, entity_schema: 'live', entity_table: 'incidents',
      entity_id: mockUuid('7125', 1), action: 'insert', fields: [],
    }),
  ]
}

function buildJobs(): JournalFailedJob[] {
  return [
    {
      id: JOURNAL_JOB(1), queue: 'email', task: 'engagement.notification_email', state: 'dead',
      attempts: 5, max_attempts: 5, last_error: 'Le relais de courriel ne répond pas (délai dépassé).',
      last_failed_at: ago(236), run_at: ago(236), created_at: ago(600),
    },
    {
      id: JOURNAL_JOB(2), queue: 'live', task: 'live.sync_meeting', state: 'retrying',
      attempts: 3, max_attempts: 5, last_error: 'Zoom : limite de requêtes atteinte (429).',
      last_failed_at: ago(290), run_at: ago(-30), created_at: ago(400),
    },
    {
      id: JOURNAL_JOB(3), queue: 'default', task: 'org.trust_score.recompute', state: 'failed',
      attempts: 1, max_attempts: 3, last_error: 'Organisation introuvable : fusionnée entre-temps.',
      last_failed_at: null, run_at: ago(2_400), created_at: ago(2_401),
    },
  ]
}

function buildFailedEvents(): JournalOutboxEvent[] {
  return [
    {
      id: JOURNAL_EVENT(1), event_type: 'programme.registration.created', aggregate_schema: 'programme',
      aggregate_type: 'registration', aggregate_id: mockUuid('7126', 1), occurred_at: ago(330),
      published_at: null, available_at: ago(-12), attempts: 4,
      last_error: 'Valkey injoignable : connexion refusée.',
    },
  ]
}

function buildTraceEvents(): JournalOutboxEvent[] {
  return [
    {
      id: JOURNAL_EVENT(2), event_type: 'programme.session.created', aggregate_schema: 'programme',
      aggregate_type: 'session', aggregate_id: SESSION.alertePrecoce, occurred_at: ago(1_210, 360),
      published_at: ago(1_210, 900), available_at: ago(1_210, 360), attempts: 1, last_error: null,
    },
    {
      id: JOURNAL_EVENT(3), event_type: 'org.organization.merged', aggregate_schema: 'org',
      aggregate_type: 'organization', aggregate_id: ORG.osed, occurred_at: ago(2_400, 100),
      published_at: ago(2_400, 700), available_at: ago(2_400, 100), attempts: 1, last_error: null,
    },
  ]
}

/** Les relances retirent un échec de la liste, jusqu'au rechargement du jeu. */
const retried = new Set<string>()

const PERIOD_MINUTES: Record<JournalPeriod, number> = { '24h': 1_440, '7d': 10_080, '30d': 43_200, '90d': 129_600 }

function withinPeriod(at: IsoDateTime, period: JournalPeriod | undefined): boolean {
  return Date.parse(at) >= Date.now() - PERIOD_MINUTES[period ?? '24h'] * MINUTE
}

function matches(needle: string | undefined, ...haystack: Array<string | null | undefined>): boolean {
  const q = needle?.trim().toLocaleLowerCase('fr')
  if (!q) return true
  return haystack.some((value) => value?.toLocaleLowerCase('fr').includes(q))
}

function paginate<T>(rows: T[], page = 1, perPage = 50): { items: T[]; total: number; page: number; per_page: number } {
  const size = Math.min(Math.max(perPage, 1), 200)
  const current = Math.max(page, 1)
  return { items: rows.slice((current - 1) * size, current * size), total: rows.length, page: current, per_page: size }
}

const byNewest = (a: { occurred_at: IsoDateTime }, b: { occurred_at: IsoDateTime }) =>
  Date.parse(b.occurred_at) - Date.parse(a.occurred_at)
const byOldest = (a: { occurred_at: IsoDateTime }, b: { occurred_at: IsoDateTime }) =>
  Date.parse(a.occurred_at) - Date.parse(b.occurred_at)

export function journalSummary(): JournalSummary {
  const recent = buildEntries().filter((row) => withinPeriod(row.occurred_at, '24h'))
  const count = (kind: JournalEntry['kind']) => recent.filter((row) => row.kind === kind).length
  const failures = journalFailures()
  return {
    since: ago(1_440),
    http_error: count('http_error'),
    http_refused: count('http_refused'),
    http_slow: count('http_slow'),
    job_failed: count('job_failed'),
    failing_jobs: failures.jobs.length,
    failing_events: failures.events.length,
    dropped: 0,
    retention_months: 3,
  }
}

export function journalEntryPage(filters: JournalEntryFilters): JournalEntryPage {
  const rows = buildEntries()
    .filter((row) => withinPeriod(row.occurred_at, filters.period))
    .filter((row) => !filters.kind || row.kind === filters.kind)
    .filter((row) => !filters.source || row.source === filters.source)
    .filter((row) =>
      matches(filters.q, row.actor?.display_name, row.route, row.error_code, row.request_id, row.message, row.task),
    )
    .sort(byNewest)
  return paginate(rows, filters.page, filters.per_page)
}

export function journalChangePage(filters: JournalChangeFilters): JournalChangePage {
  const rows = buildChanges()
    .filter((row) => withinPeriod(row.occurred_at, filters.period))
    .filter((row) => !filters.entity || `${row.entity_schema}.${row.entity_table}` === filters.entity)
    .filter((row) => !filters.action || row.action === filters.action)
    .filter((row) => matches(filters.q, row.actor?.display_name, row.entity_id))
    .sort(byNewest)
  return paginate(rows, filters.page, filters.per_page)
}

export function journalTrace(requestId: string): JournalRequestTrace {
  const id = requestId.trim()
  return {
    request_id: id,
    entries: buildEntries().filter((row) => row.request_id === id).sort(byOldest),
    changes: buildChanges().filter((row) => row.request_id === id).sort(byOldest),
    events: buildTraceEvents()
      .filter((event) =>
        (id === REQUEST.decision && event.event_type === 'programme.session.created') ||
        (id === REQUEST.merge && event.event_type === 'org.organization.merged'),
      )
      .sort(byOldest),
  }
}

export function journalFailures(): JournalFailures {
  return {
    jobs: buildJobs().filter((job) => !retried.has(job.id)),
    events: buildFailedEvents().filter((event) => !retried.has(event.id)),
  }
}

export function retryJournalJob(jobId: string): void {
  if (retried.has(jobId) || !buildJobs().some((job) => job.id === jobId)) {
    throw new ApiRequestError({ code: 'CONFLICT', message: "Ce travail n'est plus en échec : il a déjà été relancé ou a abouti." }, 409)
  }
  retried.add(jobId)
}

export function retryJournalEvent(eventId: string): void {
  if (retried.has(eventId) || !buildFailedEvents().some((event) => event.id === eventId)) {
    throw new ApiRequestError({ code: 'CONFLICT', message: 'Cet événement a déjà été relayé.' }, 409)
  }
  retried.add(eventId)
}
