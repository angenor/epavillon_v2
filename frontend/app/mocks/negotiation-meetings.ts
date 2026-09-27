/**
 * Les réunions de la Francophonie dans l'application, sans API — les quatre de la
 * recette du back-office : un atelier en ligne, une concertation ministérielle à accès
 * limité, une concertation à une place avec liste d'attente et liée au Pavillon, un
 * atelier à une place sans liste d'attente. La personne est inscrite à la première et
 * en attente à la troisième. Les refus reprennent les codes et messages de l'API.
 */
import type { AvecEmpreinte } from '~/composables/api/etiquete'
import type {
  FrancophoneMeeting,
  FrancophoneMeetings,
  MeetingRegistration,
  MeetingRegistrationState,
  MyMeetingRegistrations,
} from '~/types/negotiation-meetings'
import type { ApiErrorCode } from '~/types/api-error'
import { ApiRequestError } from '~/utils/api-error'
import { estComplete, estTerminee, inscriptionOuverte } from '~/utils/guide-nego/reunions'
import { taxonomyTerms } from './reference'

const LU_A = '2027-11-10T09:00:00Z'
const ACTIVITE_DU_PAVILLON = '01990000-0000-7000-8000-0000000c0001'

function nature(code: string): FrancophoneMeeting['type'] {
  const terme = taxonomyTerms.find((t) => t.taxonomy_code === 'francophone_meeting_type' && t.code === code)
  return { code, label: terme?.label ?? { fr: code } }
}

type Base = Omit<FrancophoneMeeting, 'registered_count'>

const commun = {
  description: null,
  format: 'onsite' as const,
  venue: null,
  has_video: false,
  organizer: 'IFDD',
  open_access: true,
  access_audience: null,
  requires_registration: true,
  capacity: null,
  waitlist_enabled: true,
  registration_opens_at: null,
  registration_closes_at: null,
  status: 'scheduled' as const,
  cancellation_reason: null,
  pavilion_session_id: null,
}

const REUNIONS: Base[] = [
  {
    ...commun,
    id: '01990000-0000-7000-8000-0000000f0001',
    type: nature('preparatory_workshop'),
    title: { fr: 'Atelier préparatoire : l’objectif mondial d’adaptation', en: 'Preparatory workshop: the global goal on adaptation' },
    description: { fr: 'Revue des textes en discussion et répartition des suivis entre délégations.' },
    start_at: '2027-11-10T12:00:00Z',
    end_at: '2027-11-10T13:30:00Z',
    format: 'online',
    has_video: true,
  },
  {
    ...commun,
    id: '01990000-0000-7000-8000-0000000f0002',
    type: nature('ministerial_consultation'),
    title: { fr: 'Concertation ministérielle francophone', en: 'Francophone ministerial consultation' },
    start_at: '2027-11-11T17:00:00Z',
    end_at: '2027-11-11T19:00:00Z',
    venue: 'Salle Amazonas, zone bleue',
    open_access: false,
    access_audience: { fr: 'ministres et chefs de délégation', en: 'ministers and heads of delegation' },
  },
  {
    ...commun,
    id: '01990000-0000-7000-8000-0000000f0003',
    type: nature('negotiators_consultation'),
    title: { fr: 'Concertation des négociatrices et négociateurs : finance', en: "Negotiators' consultation: finance" },
    start_at: '2027-11-12T13:00:00Z',
    end_at: '2027-11-12T14:30:00Z',
    format: 'hybrid',
    venue: 'Pavillon de la Francophonie',
    has_video: true,
    capacity: 1,
    pavilion_session_id: ACTIVITE_DU_PAVILLON,
  },
  {
    ...commun,
    id: '01990000-0000-7000-8000-0000000f0004',
    type: nature('preparatory_workshop'),
    title: { fr: 'Atelier préparatoire : l’article 6', en: 'Preparatory workshop: Article 6' },
    start_at: '2027-11-13T12:00:00Z',
    end_at: '2027-11-13T13:00:00Z',
    venue: 'Salle 12',
    capacity: 1,
    waitlist_enabled: false,
  },
]

const LIENS: Record<string, string> = {
  '01990000-0000-7000-8000-0000000f0001': 'https://visio.exemple.org/atelier-adaptation',
  '01990000-0000-7000-8000-0000000f0003': 'https://visio.exemple.org/concertation-finance',
}

type Place = Omit<MeetingRegistration, 'meeting_id'>

/** Les autres personnes, par réunion. */
const autres: Record<string, { inscrites: number; enAttente: number }> = {
  '01990000-0000-7000-8000-0000000f0003': { inscrites: 1, enAttente: 0 },
  '01990000-0000-7000-8000-0000000f0004': { inscrites: 1, enAttente: 0 },
}
const miennes = new Map<string, Place>([
  ['01990000-0000-7000-8000-0000000f0001', { status: 'registered', waitlist_position: null, client_ref: null, registered_at: '2027-11-01T10:00:00Z' }],
  ['01990000-0000-7000-8000-0000000f0003', { status: 'waitlisted', waitlist_position: 1, client_ref: null, registered_at: '2027-11-02T10:00:00Z' }],
])
/** La référence du dernier geste d'une ligne désinscrite : rejouée, elle rend `cancelled`. */
const desinscrites = new Map<string, string | null>()

const refus = (code: ApiErrorCode, status: number, message: string) => new ApiRequestError({ code, message }, status)

function avecLeCompte(r: Base): FrancophoneMeeting {
  const deLAutre = autres[r.id]?.inscrites ?? 0
  return { ...r, registered_count: deLAutre + (miennes.get(r.id)?.status === 'registered' ? 1 : 0) }
}

function empreinte(valeur: unknown): string {
  let h = 7
  for (const c of JSON.stringify(valeur)) h = (h * 31 + c.charCodeAt(0)) >>> 0
  return `"${h.toString(36)}"`
}

export function reunions(slug: string): AvecEmpreinte<FrancophoneMeetings> {
  const meetings = REUNIONS.map(avecLeCompte)
  return {
    valeur: {
      edition: { slug, timezone: 'America/Belem', city: 'Belém' },
      read_at: LU_A,
      server_time: new Date().toISOString(),
      meetings,
    },
    empreinte: empreinte(meetings),
  }
}

export function mesInscriptions(): AvecEmpreinte<MyMeetingRegistrations> {
  const registrations = [...miennes].map(([meeting_id, place]) => ({ meeting_id, ...place }))
  const video = REUNIONS.filter((r) => r.has_video && LIENS[r.id])
    .filter((r) => (r.requires_registration ? miennes.get(r.id)?.status === 'registered' : true))
    .map((r) => ({ meeting_id: r.id, url: LIENS[r.id]! }))
  return { valeur: { registrations, video }, empreinte: empreinte({ registrations, video }) }
}

export function inscrire(id: string, client_ref: string): MeetingRegistrationState {
  const base = REUNIONS.find((r) => r.id === id)
  if (!base) throw refus('NEGOTIATION_MEETING_UNKNOWN', 404, "Cette réunion n'existe pas.")
  const active = miennes.get(id)
  if (active) return { status: active.status, waitlist_position: active.waitlist_position }
  if (desinscrites.has(id) && desinscrites.get(id) === client_ref) return { status: 'cancelled', waitlist_position: null }

  const r = avecLeCompte(base)
  const maintenant = new Date()
  if (r.status === 'cancelled' || estTerminee(r, maintenant) || maintenant.getTime() >= new Date(r.start_at).getTime()) {
    throw refus('NEGOTIATION_MEETING_UNAVAILABLE', 409, "Cette réunion n'accepte plus ce geste : elle est annulée ou a déjà commencé.")
  }
  if (!inscriptionOuverte(r, maintenant)) {
    throw refus('NEGOTIATION_MEETING_CLOSED', 409, 'Les inscriptions à cette réunion ne sont pas ouvertes.')
  }
  const file = autres[id] ?? { inscrites: 0, enAttente: 0 }
  const place: Place =
    estComplete(r) || file.enAttente > 0
      ? { status: 'waitlisted', waitlist_position: file.enAttente + 1, client_ref, registered_at: maintenant.toISOString() }
      : { status: 'registered', waitlist_position: null, client_ref, registered_at: maintenant.toISOString() }
  if (place.status === 'waitlisted' && !r.waitlist_enabled) {
    throw refus('NEGOTIATION_MEETING_FULL', 409, "Complet — votre inscription n'a pas pu être prise.")
  }
  miennes.set(id, place)
  desinscrites.delete(id)
  return { status: place.status, waitlist_position: place.waitlist_position }
}

export function desinscrire(id: string): void {
  const place = miennes.get(id)
  if (!place) return
  miennes.delete(id)
  desinscrites.set(id, place.client_ref)
  const file = autres[id]
  if (place.status === 'registered' && file && file.enAttente > 0) {
    file.enAttente -= 1
    file.inscrites += 1
  }
}
