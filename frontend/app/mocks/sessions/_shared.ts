/**
 * Fabrique commune aux sessions simulées.
 *
 * Elle porte ce que la BASE écrit elle-même, et qu'un formulaire ne doit jamais
 * envoyer :
 *
 *   - `time_range`, colonne générée `tstzrange(starts_at, ends_at, '[)')` ;
 *   - `enforce_room_exclusivity`, dérivée de `event.rooms.is_virtual` par
 *     trigger — vraie pour une salle physique, fausse pour une salle virtuelle
 *     ou en l'absence de salle. Purement indicative : elle colore un
 *     chevauchement, elle n'empêche aucune écriture ;
 *   - `broadcast_channel_id`, renseigné d'office avec le canal par défaut de
 *     l'édition dès que `is_streamed` passe à vrai. La règle « un seul direct à
 *     la fois » ne doit pas dépendre d'une saisie.
 *
 * Les deux premières sont `readonly` dans les types : le compilateur est le seul
 * endroit où cette règle se vérifie sans relire le SQL.
 */

import type { IsoDateTime } from '~/types/shared'
import type { Session, SessionStatus } from '~/types/programme/session'
import type { ParticipationMode } from '~/types/event/edition'
import type { I18nText } from '~/types/shared'
import { CHANNEL, EVENT, EVENT_DAY, REGISTRATION_FORM, ROOM } from '../ids'

/** Salles virtuelles : pas d'exclusivité de créneau. */
const virtualRooms = new Set<string>([ROOM.atelier])

/** Jour du calendrier correspondant à une date de Belém, `null` hors édition. */
const daysByDate: Record<string, string> = {
  '2027-11-09': EVENT_DAY.nov09,
  '2027-11-10': EVENT_DAY.nov10,
  '2027-11-11': EVENT_DAY.nov11,
  '2027-11-12': EVENT_DAY.nov12,
  '2027-11-13': EVENT_DAY.nov13,
  '2027-11-14': EVENT_DAY.nov14,
  '2027-11-15': EVENT_DAY.nov15,
  '2027-11-16': EVENT_DAY.nov16,
  '2027-11-17': EVENT_DAY.nov17,
  '2027-11-18': EVENT_DAY.nov18,
  '2027-11-19': EVENT_DAY.nov19,
  '2027-11-20': EVENT_DAY.nov20,
}

export interface SessionFields {
  id: string
  /** Nul quand l'IFDD programme directement, sans passer par l'appel. */
  proposal: string | null
  organization: string | null
  sequence?: number
  title: I18nText
  slug: string
  summary?: I18nText | null
  description?: I18nText | null
  status?: SessionStatus
  format: ParticipationMode
  /** Heure locale de Belém, décalage explicite (`-03:00`). */
  startsAt: IsoDateTime
  endsAt: IsoDateTime
  room?: string | null
  locationNote?: I18nText | null
  registration?: {
    required: boolean
    opensAt?: IsoDateTime | null
    closesAt?: IsoDateTime | null
    capacity?: number | null
    waitlist?: boolean
    form?: string | null
  }
  streamed?: boolean
  recorded?: boolean
  questions?: boolean
  publishedAt?: IsoDateTime | null
  cancelledReason?: I18nText | null
  createdBy: string
  createdAt: IsoDateTime
  updatedAt?: IsoDateTime
  viewCount?: number
  attendeeCount?: number | null
}

export function session(fields: SessionFields): Session {
  const room_id = fields.room ?? null
  const registration = fields.registration ?? { required: false }
  const status = fields.status ?? 'scheduled'

  if (status === 'cancelled' && !fields.cancelledReason) {
    // `ck_sessions_cancelled_reason` refuserait la ligne.
    throw new Error(`Session ${fields.slug} : une session annulée doit porter un motif.`)
  }

  return {
    id: fields.id,
    event_id: EVENT.cop31,
    proposal_id: fields.proposal,
    event_day_id: daysByDate[fields.startsAt.slice(0, 10)] ?? null,
    organization_id: fields.organization,
    sequence_number: fields.sequence ?? 1,
    title: fields.title,
    slug: fields.slug,
    summary: fields.summary ?? null,
    description: fields.description ?? null,
    status,
    format: fields.format,
    starts_at: fields.startsAt,
    ends_at: fields.endsAt,
    time_range: `["${fields.startsAt}","${fields.endsAt}")`,
    timezone: 'America/Belem',
    room_id,
    enforce_room_exclusivity: room_id !== null && !virtualRooms.has(room_id),
    location_note: fields.locationNote ?? null,
    registration_required: registration.required,
    registration_opens_at: registration.opensAt ?? null,
    registration_closes_at: registration.closesAt ?? null,
    capacity: registration.capacity ?? null,
    waitlist_enabled: registration.waitlist ?? false,
    registration_form_id: registration.required ? (registration.form ?? REGISTRATION_FORM.cop31) : null,
    is_streamed: fields.streamed ?? false,
    broadcast_channel_id: fields.streamed ? CHANNEL.cop31 : null,
    is_recorded: fields.recorded ?? false,
    allows_questions: fields.questions ?? true,
    report: null,
    report_submitted_at: null,
    attendee_count: fields.attendeeCount ?? null,
    view_count: fields.viewCount ?? 0,
    published_at: fields.publishedAt ?? null,
    cancelled_reason: fields.cancelledReason ?? null,
    created_by: fields.createdBy,
    created_at: fields.createdAt,
    updated_at: fields.updatedAt ?? fields.createdAt,
  }
}
