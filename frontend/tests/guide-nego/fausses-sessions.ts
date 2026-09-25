import type { OfficialSession } from '../../app/types/negotiation-sessions.ts'

/** Belém : UTC−3, sans heure d'été. */
export const BELEM = 'America/Belem'

export function session(id: string, start: string, end: string | null, extra: Partial<OfficialSession> = {}): OfficialSession {
  return {
    id,
    title_en: `Session ${id}`,
    title_fr: null,
    start_at: start,
    end_at: end,
    venue: null,
    previous: null,
    type: null,
    group: null,
    theme: null,
    agenda_item: null,
    open_access: null,
    status: 'scheduled',
    cancelled: null,
    source_url: null,
    read_at: '2027-11-10T09:00:00Z',
    ...extra,
  }
}
