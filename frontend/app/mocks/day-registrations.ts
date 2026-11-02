/**
 * L'inscription aux journées de la CdP31 d'Antalya (`track-pages.ts`), sans API.
 *
 * Jeu de départ :
 * - la personne connectée est inscrite à la journée action jeunesse, deux
 *   activités décochées ; elle ne l'est pas à la journée finance durable ;
 * - deux liens de gestion d'invité, sur la journée finance :
 *   `?jeton=demo-finance` (inscrite, une activité décochée ; le direct n'est
 *   pas prêt aux deux premiers essais) et `?jeton=demo-annulee` (désinscrite) ;
 * - une dizaine d'inscrits fictifs pour la liste du back-office.
 *
 * L'état vit le temps de l'onglet, comme les autres écritures simulées.
 * L'accès signé échoue comme l'API sans configuration Zoom (503), sauf par le
 * lien d'un invité, qui rend son lien Zoom personnel.
 */
import type {
  DayRegisterPayload,
  DayRegistration,
  DayRegistrationManage,
  DayRegistrationResult,
  DayRegistrationRow,
  DaySessionsPayload,
} from '~/types/programme/day-registration'
import type { ApplicableRegistrationForm } from '~/types/programme/registration'
import type { MeetingAccess, MeetingStatus, PublicSessionMeeting } from '~/types/live'
import type { PublicTrackPage } from '~/types/programme/track-page'
import type { ApiErrorCode } from '~/types/api-error'
import { ApiRequestError } from '~/utils/api-error'
import { registrationFormFields, registrationForms } from './registration-form'
import { trackPages } from './track-pages'

interface Inscrit {
  registration: DayRegistration
  first_name: string
  last_name: string
  email: string
  organization_name: string | null
  has_account: boolean
  /** La personne connectée des données simulées. */
  is_me: boolean
  token: string | null
}

const OPENS_BEFORE_MS = 30 * 60_000
const CLOSES_AFTER_MS = 2 * 3_600_000

const pageOf = (trackId: string): PublicTrackPage | null =>
  Object.values(trackPages).find((page) => page.track.id === trackId) ?? null

const slugToId = (slug: string): string => trackPages[slug]?.track.id ?? ''

/** Activités publiées non annulées : le M de « N activités sur M ». */
function activityIds(page: PublicTrackPage): string[] {
  return page.agenda.flatMap((entry) =>
    entry.kind === 'activity' && entry.activity.session.status !== 'cancelled' ? [entry.activity.session.id] : [],
  )
}

function refuse(code: ApiErrorCode, message: string, status: number): never {
  throw new ApiRequestError({ code, message }, status)
}

const inscrits = new Map<string, Inscrit>()
let numero = 0

function creer(trackId: string, keptIds: string[], person: Omit<Inscrit, 'registration'>, at = '2026-10-28T09:00:00Z'): Inscrit {
  numero += 1
  const page = pageOf(trackId)
  const inscrit: Inscrit = {
    ...person,
    registration: {
      id: `01990000-0000-7000-8000-0000000f${String(numero).padStart(4, '0')}`,
      track_id: trackId,
      status: 'registered',
      session_ids: keptIds,
      activity_count: page ? activityIds(page).length : keptIds.length,
      created_at: at,
      joined_at: null,
    },
  }
  inscrits.set(inscrit.registration.id, inscrit)
  return inscrit
}

const sauf = (trackId: string, skip: number[]): string[] => {
  const page = pageOf(trackId)
  return page ? activityIds(page).filter((_, index) => !skip.includes(index)) : []
}

const FINANCE = slugToId('journee-finance-durable')
const JEUNESSE = slugToId('journee-jeunesse-climat')

const personne = (first_name: string, last_name: string, organization_name: string | null, has_account: boolean) => ({
  first_name,
  last_name,
  email: `${first_name}.${last_name}`.toLowerCase().normalize('NFD').replace(/[^a-z.]/g, '') + '@exemple.org',
  organization_name,
  has_account,
  is_me: false,
  token: null,
})

creer(JEUNESSE, sauf(JEUNESSE, [1, 4]), { ...personne('Angenor', 'Test', 'IFDD', true), is_me: true })
creer(FINANCE, sauf(FINANCE, [3]), { ...personne('Awa', 'Koné', null, false), token: 'demo-finance' })
{
  const annulee = creer(FINANCE, sauf(FINANCE, []), { ...personne('Paul', 'Mbarga', 'Commune de Sakété', false), token: 'demo-annulee' })
  annulee.registration = { ...annulee.registration, status: 'cancelled' }
}
const FICTIFS: Array<[string, string, string | null, boolean, number[]]> = [
  ['Mariam', 'Ndiaye', 'Délégation du Sénégal', true, []],
  ['Jean-Baptiste', 'Hounkpè', 'Ministère du Cadre de vie, Bénin', true, [0, 1]],
  ['Fatou', 'Sarr', null, false, [5]],
  ['Rachid', 'Benali', 'Agence marocaine pour l\'efficacité énergétique', true, []],
  ['Claire', 'Dubois', 'OIF', true, [2, 3, 4]],
  ['Ibrahim', 'Maïga', 'CILSS', false, []],
  ['Nadia', 'Rakotomalala', 'Business Maurice', true, [0]],
]
for (const [prenom, nom, org, compte, skip] of FICTIFS) {
  creer(FINANCE, sauf(FINANCE, skip), personne(prenom, nom, org, compte), '2026-10-30T14:00:00Z')
  creer(JEUNESSE, sauf(JEUNESSE, skip), personne(prenom, nom, org, compte), '2026-10-31T10:00:00Z')
}

const vivants = (trackId: string) => [...inscrits.values()].filter((i) => i.registration.track_id === trackId && i.registration.status !== 'cancelled')
const parId = (id: string): Inscrit => inscrits.get(id) ?? refuse('NOT_FOUND', 'Inscription introuvable.', 404)

export function dayForm(trackId: string): ApplicableRegistrationForm | null {
  if (!pageOf(trackId)) return null
  const form = registrationForms.find((f) => f.is_default)
  if (!form) return null
  return {
    form,
    fields: registrationFormFields.filter((f) => f.form_id === form.id && f.is_active).sort((a, b) => a.sort_order - b.sort_order),
  }
}

export function dayRegister(payload: DayRegisterPayload): DayRegistrationResult {
  const page = pageOf(payload.track_id) ?? refuse('NOT_FOUND', "Cette journée n'existe pas.", 404)
  if (page.ends_at && Date.now() >= Date.parse(page.ends_at)) return { status: 'closed', closed_at: page.ends_at }
  const known = new Set(activityIds(page))
  const kept = payload.session_ids.filter((id) => known.has(id))
  const guest = payload.guest
  if (guest) {
    const email = guest.email.trim().toLowerCase()
    if (vivants(payload.track_id).some((i) => i.email === email)) return { status: 'already_registered', registration: null }
    const inscrit = creer(payload.track_id, kept, {
      first_name: guest.first_name,
      last_name: guest.last_name,
      email,
      organization_name: null,
      has_account: false,
      is_me: false,
      token: `demo-${numero + 1}`,
    }, new Date().toISOString())
    // Ce que porterait le courriel : le lien de gestion, à ouvrir à la main sur les données simulées.
    console.info(`[données simulées] lien de gestion : ?jeton=${inscrit.token}`)
    return { status: 'registered', registration: inscrit.registration }
  }
  const mine = vivants(payload.track_id).find((i) => i.is_me)
  if (mine) return { status: 'already_registered', registration: mine.registration }
  const inscrit = creer(payload.track_id, kept, { ...personne('Angenor', 'Test', 'IFDD', true), is_me: true }, new Date().toISOString())
  return { status: 'registered', registration: inscrit.registration }
}

export function dayMine(trackId: string): DayRegistration | null {
  return vivants(trackId).find((i) => i.is_me)?.registration ?? null
}

export function dayManage(token: string): DayRegistrationManage | null {
  const inscrit = [...inscrits.values()].find((i) => i.token === token)
  const page = inscrit ? pageOf(inscrit.registration.track_id) : null
  if (!inscrit || !page) return null
  return {
    registration: inscrit.registration,
    event_slug: page.edition.slug,
    track_slug: page.track.slug,
    track_title: page.track.title,
    first_name: inscrit.first_name,
  }
}

export function dayUpdateSessions(registrationId: string, payload: DaySessionsPayload): DayRegistration {
  const inscrit = parId(registrationId)
  if (inscrit.registration.status === 'cancelled') {
    refuse('REGISTRATION_LOCKED', 'Cette inscription est annulée : inscrivez-vous de nouveau pour choisir vos activités.', 422)
  }
  const page = pageOf(inscrit.registration.track_id)
  const known = new Set(page ? activityIds(page) : [])
  inscrit.registration = { ...inscrit.registration, session_ids: payload.session_ids.filter((id) => known.has(id)) }
  return inscrit.registration
}

export function dayCancel(registrationId: string): DayRegistration {
  const inscrit = parId(registrationId)
  inscrit.registration = { ...inscrit.registration, status: 'cancelled' }
  return inscrit.registration
}

export function dayRegistrationRows(trackId: string): DayRegistrationRow[] {
  return [...inscrits.values()]
    .filter((i) => i.registration.track_id === trackId)
    .map((i) => ({
      id: i.registration.id,
      status: i.registration.status,
      person_id: `${i.registration.id.slice(0, -4)}${i.first_name.length.toString().padStart(4, '0')}`,
      first_name: i.first_name,
      last_name: i.last_name,
      email: i.email,
      organization_name: i.organization_name,
      has_account: i.has_account,
      kept_count: i.registration.session_ids.length,
      created_at: i.registration.created_at,
      joined_at: i.registration.joined_at,
    }))
}

function statut(page: PublicTrackPage, start: string, end: string, at: number): MeetingStatus {
  if (page.agenda.length && page.agenda.every((e) => e.kind === 'moment' || e.activity.session.status === 'cancelled')) return 'cancelled'
  if (at >= Date.parse(end) + CLOSES_AFTER_MS) return 'ended'
  return at >= Date.parse(start) ? 'started' : 'scheduled'
}

export function trackMeeting(trackId: string, at: number = Date.now()): PublicSessionMeeting | null {
  const page = pageOf(trackId)
  const start = page?.opens_at ?? page?.starts_at
  if (!page || !start || !page.ends_at) return null
  if (!page.agenda.some((e) => e.kind === 'activity' && e.activity.session.is_streamed)) return null
  return {
    provider: 'zoom',
    status: statut(page, start, page.ends_at, at),
    opens_at: new Date(Date.parse(start) - OPENS_BEFORE_MS).toISOString(),
    starts_at: start,
    ends_at: page.ends_at,
    closes_at: new Date(Date.parse(page.ends_at) + CLOSES_AFTER_MS).toISOString(),
  }
}

/** `GET /sessions/{id}/meeting` d'une activité de journée : la réunion de sa journée. */
export function dayMeetingOfSession(sessionId: string, at: number = Date.now()): PublicSessionMeeting | null {
  const page = Object.values(trackPages).find((p) => p.agenda.some((e) => e.kind === 'activity' && e.activity.session.id === sessionId))
  return page ? trackMeeting(page.track.id, at) : null
}

const essais = new Map<string, number>()

export function trackMeetingAccess(trackId: string, token: string | null): MeetingAccess {
  if (token) {
    const inscrit = [...inscrits.values()].find((i) => i.token === token && i.registration.track_id === trackId)
    if (!inscrit) refuse('NOT_FOUND', "Ce lien n'est plus valable.", 404)
    if (inscrit.registration.status === 'cancelled') {
      refuse('LIVE_MEETING_REGISTRATION_REQUIRED', "Votre inscription à cette journée est annulée : le direct ne vous est plus ouvert.", 403)
    }
    const n = (essais.get(token) ?? 0) + 1
    essais.set(token, n)
    if (n <= 2) refuse('LIVE_MEETING_NOT_READY', 'Votre lien personnel est en cours de préparation. Réessayez dans un instant.', 409)
    inscrit.registration = { ...inscrit.registration, joined_at: inscrit.registration.joined_at ?? new Date().toISOString() }
    const page = pageOf(trackId)
    return {
      meeting_number: '84112000002',
      password: '',
      registrant_token: null,
      user_name: `${inscrit.first_name} ${inscrit.last_name}`,
      user_email: inscrit.email,
      sdk_key: '',
      signature: '',
      expires_at: new Date(Date.now() + 3_600_000).toISOString(),
      join_url: `https://zoom.us/w/84112000002?tk=${encodeURIComponent(token)}&from=${page?.track.slug ?? ''}`,
    }
  }
  if (!dayMine(trackId)) {
    refuse('LIVE_MEETING_REGISTRATION_REQUIRED', 'Inscrivez-vous à la journée pour rejoindre son direct.', 403)
  }
  refuse(
    'LIVE_ZOOM_UNAVAILABLE',
    "La réunion Zoom est momentanément indisponible. Réessayez dans quelques instants ou ouvrez-la dans l'application Zoom.",
    503,
  )
}
