/**
 * Les réunions de la Francophonie au back-office, sans API — pour les tests et
 * le travail hors ligne. Trois réunions de la COP31 : un brouillon, une publiée,
 * une annulée. Les refus reprennent les codes et les champs de l'API.
 */

import type {
  AdminFrancophoneMeeting,
  AdminFrancophoneMeetings,
  AdminMeetingRegistrations,
  FrancophoneMeetingInput,
  PavilionActivityOption,
} from '~/types/negotiation-meetings'
import type { ApiErrorCode } from '~/types/api-error'
import type { I18nText, IsoDateTime, Uuid } from '~/types/shared'
import { ApiRequestError } from '~/utils/api-error'

const FUSEAU = 'Europe/Istanbul'
const VILLE = 'Antalya'

const NATURES: Record<string, I18nText> = {
  preparatory_workshop: { fr: 'Atelier préparatoire', en: 'Preparatory workshop' },
  negotiators_consultation: { fr: 'Concertation des négociatrices et négociateurs', en: "Negotiators' consultation" },
  ministerial_consultation: { fr: 'Concertation ministérielle', en: 'Ministerial consultation' },
}

const ACTIVITES: PavilionActivityOption[] = [
  {
    id: '01990000-0000-7000-8000-0000000c0001',
    title: { fr: 'Financer l\'adaptation en Afrique francophone', en: 'Financing adaptation in Francophone Africa' },
    starts_at: '2026-11-10T07:00:00Z',
  },
  {
    id: '01990000-0000-7000-8000-0000000c0002',
    title: { fr: 'Journée de la Francophonie : dialogue ministériel', en: 'Francophonie Day: ministerial dialogue' },
    starts_at: '2026-11-12T11:30:00Z',
  },
]

const reunion = (partiel: Partial<AdminFrancophoneMeeting> & Pick<AdminFrancophoneMeeting, 'id' | 'slug' | 'title' | 'start_at' | 'end_at'>): AdminFrancophoneMeeting => ({
  edition: 'cop31',
  timezone: FUSEAU,
  type: null,
  description: null,
  format: 'onsite',
  venue: null,
  external_url: null,
  capacity: null,
  waitlist_enabled: true,
  requires_registration: true,
  registration_opens_at: null,
  registration_closes_at: null,
  open_access: true,
  access_audience: null,
  is_ifdd_organized: true,
  organizer_org_id: null,
  organizer: 'IFDD',
  status: 'draft',
  cancellation_reason: null,
  pavilion_session_id: null,
  registered_count: 0,
  waitlisted_count: 0,
  updated_at: '2026-09-24T09:00:00Z',
  ...partiel,
})

let reunions: AdminFrancophoneMeeting[] = [
  reunion({
    id: '01990000-0000-7000-8000-0000000b0001',
    slug: 'atelier-preparatoire-finance',
    type: { code: 'preparatory_workshop', label: NATURES.preparatory_workshop! },
    title: { fr: 'Atelier préparatoire : finance climat', en: 'Preparatory workshop: climate finance' },
    description: {
      fr: 'Revue des positions communes avant l\'ouverture des négociations sur le nouvel objectif de financement.',
      en: 'Review of common positions before negotiations on the new finance goal open.',
    },
    start_at: '2026-11-08T06:30:00Z',
    end_at: '2026-11-08T09:00:00Z',
    format: 'hybrid',
    venue: 'Pavillon de la Francophonie, salle B',
    external_url: 'https://visio.example.org/atelier-finance',
    capacity: 40,
    registration_closes_at: '2026-11-07T18:00:00Z',
    status: 'scheduled',
    pavilion_session_id: ACTIVITES[0]!.id,
    registered_count: 40,
    waitlisted_count: 3,
  }),
  reunion({
    id: '01990000-0000-7000-8000-0000000b0002',
    slug: 'concertation-ministerielle',
    type: { code: 'ministerial_consultation', label: NATURES.ministerial_consultation! },
    title: { fr: 'Concertation ministérielle francophone', en: 'Francophone ministerial consultation' },
    start_at: '2026-11-16T12:00:00Z',
    end_at: '2026-11-16T14:00:00Z',
    format: 'onsite',
    open_access: false,
    access_audience: { fr: 'Ministres et chefs de délégation', en: 'Ministers and heads of delegation' },
  }),
  reunion({
    id: '01990000-0000-7000-8000-0000000b0003',
    slug: 'concertation-negociateurs-adaptation',
    type: { code: 'negotiators_consultation', label: NATURES.negotiators_consultation! },
    title: { fr: 'Concertation sur l\'objectif mondial d\'adaptation', en: 'Consultation on the global goal on adaptation' },
    start_at: '2026-11-11T14:00:00Z',
    end_at: '2026-11-11T15:30:00Z',
    format: 'online',
    external_url: 'https://visio.example.org/concertation-adaptation',
    requires_registration: false,
    status: 'cancelled',
    cancellation_reason: 'Reportée : la séance plénière a été prolongée.',
  }),
]

function refus(code: ApiErrorCode, status: number, message: string, field?: string): ApiRequestError {
  return new ApiRequestError({ code, message, field }, status)
}
const inconnue = () => refus('NEGOTIATION_MEETING_UNKNOWN', 404, "Cette réunion n'existe pas.")

function trouver(id: Uuid): AdminFrancophoneMeeting {
  const trouvee = reunions.find((r) => r.id === id)
  if (!trouvee) throw inconnue()
  return trouvee
}

function remplacer(suivante: AdminFrancophoneMeeting): AdminFrancophoneMeeting {
  reunions = reunions.map((r) => (r.id === suivante.id ? suivante : r))
  return suivante
}

const maintenant = (): IsoDateTime => new Date().toISOString()

function depuisLaSaisie(entree: FrancophoneMeetingInput): Omit<AdminFrancophoneMeeting, 'id' | 'slug' | 'edition' | 'timezone' | 'status' | 'cancellation_reason' | 'pavilion_session_id' | 'registered_count' | 'waitlisted_count'> {
  if (!entree.title.fr?.trim()) throw refus('NEGOTIATION_MEETING_INVALID', 400, 'Le titre en français est obligatoire.', 'title')
  if (Date.parse(entree.end_at) <= Date.parse(entree.start_at)) {
    throw refus('NEGOTIATION_MEETING_INVALID', 400, 'La fin doit suivre le début.', 'end_at')
  }
  const label = entree.type ? NATURES[entree.type] : undefined
  return {
    type: entree.type && label ? { code: entree.type, label } : null,
    title: entree.title,
    description: entree.description,
    start_at: entree.start_at,
    end_at: entree.end_at,
    format: entree.format,
    venue: entree.venue,
    external_url: entree.external_url,
    capacity: entree.capacity,
    waitlist_enabled: entree.waitlist_enabled,
    requires_registration: entree.requires_registration,
    registration_opens_at: entree.registration_opens_at,
    registration_closes_at: entree.registration_closes_at,
    open_access: entree.open_access,
    access_audience: entree.access_audience,
    is_ifdd_organized: entree.is_ifdd_organized,
    organizer_org_id: entree.is_ifdd_organized ? null : entree.organizer_org_id,
    organizer: entree.is_ifdd_organized ? 'IFDD' : 'Organisation partenaire',
    updated_at: maintenant(),
  }
}

export function reunionsDeLEdition(edition: string): AdminFrancophoneMeetings {
  return {
    edition: { slug: edition, timezone: FUSEAU, city: VILLE },
    meetings: reunions
      .filter((r) => r.edition === edition)
      .sort((a, b) => a.start_at.localeCompare(b.start_at)),
  }
}

export function uneReunion(id: Uuid): AdminFrancophoneMeeting | null {
  return reunions.find((r) => r.id === id) ?? null
}

export function creer(entree: FrancophoneMeetingInput): AdminFrancophoneMeeting {
  const nouvelle: AdminFrancophoneMeeting = {
    ...depuisLaSaisie(entree),
    id: crypto.randomUUID(),
    slug: `reunion-${reunions.length + 1}`,
    edition: entree.edition ?? 'cop31',
    timezone: FUSEAU,
    status: 'draft',
    cancellation_reason: null,
    pavilion_session_id: null,
    registered_count: 0,
    waitlisted_count: 0,
  }
  reunions = [...reunions, nouvelle]
  return nouvelle
}

export function modifier(id: Uuid, entree: FrancophoneMeetingInput): AdminFrancophoneMeeting {
  return remplacer({ ...trouver(id), ...depuisLaSaisie(entree) })
}

export function publier(id: Uuid): AdminFrancophoneMeeting {
  const r = trouver(id)
  if (!r.type) throw refus('NEGOTIATION_MEETING_INVALID', 400, 'Choisissez la nature de la réunion avant de la publier.', 'type')
  if (r.format !== 'online' && !r.venue) {
    throw refus('NEGOTIATION_MEETING_INVALID', 400, 'Une réunion sur place indique où elle se tient.', 'venue')
  }
  if (r.format !== 'onsite' && !r.external_url) {
    throw refus('NEGOTIATION_MEETING_INVALID', 400, 'Une réunion en ligne offre un lien pour s\'y connecter.', 'external_url')
  }
  return remplacer({ ...r, status: 'scheduled', updated_at: maintenant() })
}

export function annuler(id: Uuid, motif: string): AdminFrancophoneMeeting {
  if (!motif.trim()) throw refus('NEGOTIATION_MEETING_INVALID', 400, 'Le motif de l\'annulation est obligatoire.', 'reason')
  return remplacer({ ...trouver(id), status: 'cancelled', cancellation_reason: motif.trim(), updated_at: maintenant() })
}

export function lierAuPavillon(id: Uuid, sessionId: Uuid | null): AdminFrancophoneMeeting {
  return remplacer({ ...trouver(id), pavilion_session_id: sessionId, updated_at: maintenant() })
}

export function inscrites(id: Uuid): AdminMeetingRegistrations {
  const r = trouver(id)
  const pays: I18nText[] = [
    { fr: 'Sénégal', en: 'Senegal' },
    { fr: 'Côte d\'Ivoire', en: "Côte d'Ivoire" },
    { fr: 'Haïti', en: 'Haiti' },
  ]
  const personne = (i: number, attente: number | null) => ({
    person_id: `01990000-0000-7000-8000-0000000d${String(i).padStart(4, '0')}`,
    name: `Négociatrice ${i}`,
    country: pays[i % pays.length] ?? null,
    registered_at: new Date(Date.parse('2026-10-01T08:00:00Z') + i * 3_600_000).toISOString(),
    waitlist_position: attente,
  })
  return {
    registered: Array.from({ length: Math.min(r.registered_count, 12) }, (_, i) => personne(i + 1, null)),
    waitlisted: Array.from({ length: r.waitlisted_count }, (_, i) => personne(100 + i, i + 1)),
  }
}

export function activitesDuPavillon(_edition: string): PavilionActivityOption[] {
  return [...ACTIVITES]
}
