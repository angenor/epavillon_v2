/**
 * `GET /events/{event_id}/programme-changes`, hors ligne : quelques changements
 * de l'édition d'Antalya, datés à rebours de maintenant pour que la fenêtre et
 * le lien « depuis votre dernière visite » aient de quoi montrer.
 */
import type { ProgrammeChange, ProgrammeChangeKind } from '~/types/programme/changes'
import type { PublicScheduleRow } from '~/types/views'
import { TRACK_PAGE } from './ids'
import { trackPagesSchedule } from './track-pages'

const HOUR = 3_600_000

const shift = (iso: string, hours: number) => new Date(Date.parse(iso) + hours * HOUR).toISOString()

function change(row: PublicScheduleRow, kind: ProgrammeChangeKind, hoursAgo: number, previousShift = 0): ProgrammeChange {
  return {
    occurred_at: new Date(Date.now() - hoursAgo * HOUR).toISOString(),
    kind,
    session_id: row.id,
    slug: row.slug,
    title: row.title,
    starts_at: row.starts_at,
    ends_at: row.ends_at,
    previous_starts_at: kind === 'added' ? null : shift(row.starts_at, previousShift),
    previous_ends_at: kind === 'added' ? null : shift(row.ends_at, previousShift),
    previous_title: kind === 'retitled' ? { fr: 'Enjeux de financements pour les régions et les municipalités' } : null,
    new_title: kind === 'retitled' ? row.title : null,
    day_changed: kind === 'rescheduled' && Math.abs(previousShift) >= 24,
  }
}

export function programmeChanges(eventId: string): ProgrammeChange[] {
  if (eventId !== TRACK_PAGE.edition) return []
  const rows = trackPagesSchedule
  const pick = (index: number) => rows[index % rows.length]!
  if (!rows.length) return []
  return [
    change(pick(3), 'rescheduled', 2, -1.25),
    change(pick(5), 'cancelled', 5),
    change(pick(2), 'retitled', 26),
    change(pick(7), 'postponed', 28),
    change(pick(9), 'added', 30),
    change(pick(4), 'rescheduled', 75, -192),
  ]
}
