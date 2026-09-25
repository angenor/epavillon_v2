/**
 * L'import des sessions officielles et l'ordre du jour, sans API — pour les
 * tests et le travail hors ligne. Les jeux et les valeurs par défaut sont ceux
 * du binaire et du modèle (`100_negotiations.sql`).
 */

import type {
  AgendaItemAdmin,
  OfficialImportAdmin,
  UpdateOfficialImportPayload,
} from '~/types/negotiation-sessions'
import type { Uuid } from '~/types/shared'

const JEUX = ['cop30/lecture-1', 'cop30/lecture-2', 'cop30/illisible', 'cop30/injoignable']

let reglage: UpdateOfficialImportPayload = {
  enabled: false,
  reader: 'archive',
  archive_name: 'cop30/lecture-1',
  archive_first_day: null,
  live_url: null,
  time_correction_minutes: 60,
  official_programme_url: 'https://unfccc.int/cop31/schedule',
  interval_seconds: 300,
  missed_threshold: 3,
}

const POINTS: AgendaItemAdmin[] = [
  { id: '01990000-0000-7000-8000-00000000a001', code: 'CMA 8 (a)', title: 'Global goal on adaptation', theme: null, session_count: 4, theme_set_at: null },
  { id: '01990000-0000-7000-8000-00000000a002', code: 'SBI 12', title: 'Gender and climate change', theme: 'gender', session_count: 2, theme_set_at: '2026-09-20T09:00:00Z' },
  { id: '01990000-0000-7000-8000-00000000a003', code: 'SBSTA 7', title: 'Research and systematic observation', theme: null, session_count: 1, theme_set_at: null },
]

export function etatDeLImport(edition: string): OfficialImportAdmin {
  return {
    ...reglage,
    edition: { slug: edition, name: { fr: 'COP31', en: 'COP31' }, timezone: 'Europe/Istanbul' },
    archives: [...JEUX],
    missed_reads: 0,
    serving: reglage.enabled,
    last_success_at: reglage.enabled ? '2026-09-25T08:40:00Z' : null,
    last_attempt_at: reglage.enabled ? '2026-09-25T08:40:00Z' : null,
    last_error: null,
    failing_since: null,
    last_change_count: reglage.enabled ? 0 : null,
    session_count: reglage.enabled ? 44 : 0,
    agenda_items_without_theme: POINTS.filter((p) => !p.theme).length,
    runs: reglage.enabled
      ? [{ started_at: '2026-09-25T08:40:00Z', outcome: 'success', error: null, session_count: 44, change_count: 0, manual: false }]
      : [],
  }
}

export function reglerLImport(edition: string, nouveau: UpdateOfficialImportPayload): OfficialImportAdmin {
  reglage = { ...nouveau }
  return etatDeLImport(edition)
}

export function points(_edition: string): AgendaItemAdmin[] {
  return [...POINTS].sort((a, b) => Number(Boolean(a.theme)) - Number(Boolean(b.theme)) || a.code.localeCompare(b.code))
}

export function rattacher(pointId: Uuid, theme: string | null): AgendaItemAdmin {
  const point = POINTS.find((p) => p.id === pointId)
  if (!point) throw new Error("Ce point de l'ordre du jour n'existe pas.")
  point.theme = theme
  point.theme_set_at = theme ? new Date().toISOString() : null
  return { ...point }
}
