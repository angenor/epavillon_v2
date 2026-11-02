import type {
  JournalChange,
  JournalChangeAction,
  JournalEntry,
  JournalKind,
  JournalOutboxEvent,
  JournalPeriod,
  JournalRequestTrace,
  JournalSource,
} from '~/types/admin-journal'
import type { Intent } from '~/types/ui'

/**
 * Le journal du back-office (`/admin/journal`) : ce que l'URL porte, et ce qui
 * se calcule sans écran. Les paramètres d'URL sont en français, comme sur les
 * autres listes : une adresse filtrée se transmet telle quelle.
 */

export const JOURNAL_TABS = ['erreurs', 'taches', 'modifications'] as const
export type JournalTab = (typeof JOURNAL_TABS)[number]

export function journalTabOf(value: unknown): JournalTab {
  return JOURNAL_TABS.find((tab) => tab === value) ?? 'erreurs'
}

export const JOURNAL_PERIOD_PARAM: Record<string, JournalPeriod> = { '24h': '24h', '7j': '7d', '30j': '30d', '90j': '90d' }
export const JOURNAL_KIND_PARAM: Record<string, JournalKind> = {
  panne: 'http_error',
  refus: 'http_refused',
  lent: 'http_slow',
  tache: 'job_failed',
}
export const JOURNAL_SOURCE_PARAM: Record<string, JournalSource> = { serveur: 'api', taches: 'worker' }
export const JOURNAL_ACTION_PARAM: Record<string, JournalChangeAction> = {
  cree: 'insert',
  modifie: 'update',
  supprime: 'delete',
}
/** Les objets proposés au filtre ; le tableau sait nommer les autres. */
export const JOURNAL_ENTITY_PARAM: Record<string, string> = {
  dossiers: 'programme.proposals',
  activites: 'programme.sessions',
  organisations: 'org.organizations',
  personnes: 'identity.people',
}

/** La valeur de l'API derrière un paramètre d'URL, ou rien s'il est inconnu. */
export function journalParam<T>(map: Record<string, T>, value: unknown): T | undefined {
  return typeof value === 'string' ? map[value] : undefined
}

/** Le paramètre d'URL d'une valeur de l'API. */
export function journalParamFor<T>(map: Record<string, T>, value: T | undefined): string | null {
  if (value === undefined) return null
  return Object.entries(map).find(([, mapped]) => mapped === value)?.[0] ?? null
}

/** Page d'URL : un entier à partir de 1, sinon 1. */
export function journalPageOf(value: unknown): number {
  const page = typeof value === 'string' ? Number.parseInt(value, 10) : Number.NaN
  return Number.isFinite(page) && page > 0 ? page : 1
}

/** Les filtres vides ne partent pas : `?q=` filtrerait sur une chaîne vide. */
export function journalQuery(filters: object): Record<string, string | number> {
  const query: Record<string, string | number> = {}
  for (const [key, value] of Object.entries(filters)) {
    if (value === undefined || value === null || value === '') continue
    if (typeof value === 'string' || typeof value === 'number') query[key] = value
  }
  return query
}

/** Le chemin d'API de l'export : les filtres de la liste, sans sa pagination. */
export function journalCsvPath(filters: { period?: string; kind?: string; source?: string; q?: string }): string {
  const params = new URLSearchParams()
  for (const [key, value] of Object.entries(journalQuery(filters))) params.set(key, String(value))
  const search = params.toString()
  return `/admin/journal/entries.csv${search ? `?${search}` : ''}`
}

/** Panne rouge, refus jaune, lenteur cyan : la règle des couleurs d'état du guide. */
export const JOURNAL_KIND_INTENT: Record<JournalKind, Intent> = {
  http_error: 'danger',
  http_refused: 'warning',
  http_slow: 'info',
  job_failed: 'danger',
}

export const JOURNAL_ACTION_INTENT: Record<JournalChangeAction, Intent> = {
  insert: 'success',
  update: 'info',
  delete: 'danger',
}

/** Clé i18n d'un code pointé (`engagement.notification_email`) : le point sépare les niveaux pour vue-i18n. */
export function journalCodeKey(code: string): string {
  return code.replace(/[.\s]/g, '_')
}

/** Fin d'identifiant : le début d'un UUID v7 est l'horodatage, commun aux lignes voisines. */
export function journalShortId(id: string | null): string {
  return id ? `…${id.slice(-8)}` : ''
}

/** Une valeur d'audit, lisible : `null` reste nul pour que l'écran dise « vide ». */
export function journalValueText(value: unknown): string | null {
  if (value === null || value === undefined || value === '') return null
  if (typeof value === 'string') return value
  if (typeof value === 'number' || typeof value === 'boolean') return String(value)
  return JSON.stringify(value)
}

export type JournalTraceItem =
  | { kind: 'entry'; at: string; key: string; entry: JournalEntry }
  | { kind: 'change'; at: string; key: string; change: JournalChange }
  | { kind: 'event'; at: string; key: string; event: JournalOutboxEvent }

/** À heure égale : l'écriture, puis l'événement émis dans la même transaction, puis la ligne du journal, écrite en fin de demande. */
const TRACE_ORDER: Record<JournalTraceItem['kind'], number> = { change: 0, event: 1, entry: 2 }

/** Les trois sources d'une demande, fusionnées par heure. */
export function journalTraceItems(trace: JournalRequestTrace): JournalTraceItem[] {
  const items: JournalTraceItem[] = [
    ...trace.entries.map((entry) => ({ kind: 'entry' as const, at: entry.occurred_at, key: `e-${entry.id}`, entry })),
    ...trace.changes.map((change) => ({ kind: 'change' as const, at: change.occurred_at, key: `c-${change.id}`, change })),
    ...trace.events.map((event) => ({ kind: 'event' as const, at: event.occurred_at, key: `o-${event.id}`, event })),
  ]
  return items.sort((a, b) => Date.parse(a.at) - Date.parse(b.at) || TRACE_ORDER[a.kind] - TRACE_ORDER[b.kind])
}

/** L'heure d'une ligne, à la seconde, et à la milliseconde pour le fil d'une demande. */
export function journalClock(value: string, timeZone: string, locale: string, withMilliseconds = false): string {
  return new Intl.DateTimeFormat(locale, {
    timeZone,
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hourCycle: 'h23',
    ...(withMilliseconds ? { fractionalSecondDigits: 3 as const } : {}),
  }).format(new Date(value))
}

/** « 2 nov. » : le jour, quand la ligne n'est pas d'aujourd'hui. */
export function journalShortDay(value: string, timeZone: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, { timeZone, day: 'numeric', month: 'short' }).format(new Date(value))
}
