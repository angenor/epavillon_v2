/**
 * L'onglet « Visites » de la fiche (02/11/2026) : les membres de la fiche
 * simulée, une visite de « Mon organisation » et une connexion chacun.
 */

import type { OrganizationVisits, VisitEvent } from '~/types/admin-organizations'
import { organizationDetail } from './detail'

const HOUR = 3_600_000

function event(partial: Partial<VisitEvent> & Pick<VisitEvent, 'kind' | 'person_id' | 'at'>): VisitEvent {
  return {
    until: null,
    count: null,
    proposal_id: null,
    proposal_reference: null,
    proposal_title: null,
    device: null,
    tables: [],
    route: null,
    status: null,
    message: null,
    request_id: null,
    ...partial,
  }
}

export function organizationVisits(organizationId: string, days: number): OrganizationVisits | null {
  const detail = organizationDetail(organizationId)
  if (!detail) return null

  const now = Date.now()
  const members = detail.members.filter((m) => m.status === 'active')
  const iso = (ms: number) => new Date(ms).toISOString()

  return {
    recording_since: iso(now - 10 * 24 * HOUR),
    window_start: days > 0 ? iso(now - days * 24 * HOUR) : null,
    members: members.map((m, rank) => ({
      person_id: m.person_id,
      display_name: m.display_name,
      role: m.role,
      active: true,
      last_connection_at: rank === 0 ? iso(now - 26 * HOUR) : null,
      last_visit_at: rank === 0 ? iso(now - 26 * HOUR) : null,
      last_visit_reference: null,
      visit_days: rank === 0 ? 1 : 0,
      changes: 0,
      last_change_at: null,
    })),
    events: members.slice(0, 1).flatMap((m) => [
      event({ kind: 'visit', person_id: m.person_id, at: iso(now - 26 * HOUR), count: 2 }),
      event({
        kind: 'connection',
        person_id: m.person_id,
        at: iso(now - 26.1 * HOUR),
        until: iso(now - 25.5 * HOUR),
        device: 'Chrome, ordinateur',
      }),
    ]),
  }
}

export function recordSpaceVisit(): void {}
