/**
 * Le Pavillon dans l'application, sans API. Les heures se posent autour de
 * maintenant : aujourd'hui une activité terminée avec rediffusion, une en cours (où
 * la personne est inscrite), une prévue au formulaire complet, une pleine avec liste
 * d'attente ; hier une rediffusion ; demain une prévue et une sans inscription.
 *
 * Le lieu vient de `rooms.ts`, et le formulaire de `registration-form.ts` : la séance
 * prévue reprend l'identifiant d'une séance du site liée au formulaire COP31 (complet),
 * les autres tombent sur le formulaire de la plateforme (le pays seul).
 */
import type { AvecEmpreinte } from '~/composables/api/etiquete'
import type {
  AskQuestionPayload,
  CancelRegistrationResult,
  PublicSessionQuestion,
  Registration,
  RegistrationResult,
  SessionRegisterPayload,
} from '~/types/programme/registration'
import type { PublicScheduleRow, PublicSessionDetail } from '~/types/views'
import { ApiRequestError } from '~/utils/api-error'
import { REGISTRATION_FORM } from './ids'
import { allSessions } from './sessions'
import { publicSchedule } from './views'

const FUSEAU = 'America/Belem'
const MOI = '01990000-0000-7000-8000-0000000d0000'
const MINUTE = 60_000

const avecFormulaireComplet = allSessions.find((s) => s.registration_form_id === REGISTRATION_FORM.cop31 && s.published_at)

type Modele = Pick<PublicScheduleRow, 'id' | 'slug' | 'title'> & {
  debut: number
  duree: number
  extra?: Partial<PublicScheduleRow>
}

const id = (n: number) => `01990000-0000-7000-8000-0000000c01${String(n).padStart(2, '0')}`

const MODELES: Modele[] = [
  {
    id: id(1),
    slug: 'jeunes-negociateurs-retour-cop30',
    title: { fr: 'Jeunes négociateurs francophones : retour de la COP30', en: 'Young francophone negotiators: back from COP30' },
    debut: -24 * 60,
    duree: 60,
    extra: { replay_url: 'https://www.youtube.com/watch?v=exemple-hier', replay_duration_seconds: 47 * 60 },
  },
  {
    id: id(2),
    slug: 'adaptation-securite-alimentaire-sahel',
    title: { fr: 'Adaptation et sécurité alimentaire au Sahel', en: 'Adaptation and food security in the Sahel' },
    debut: -180,
    duree: 60,
    extra: { replay_url: 'https://www.youtube.com/watch?v=exemple-matin', replay_duration_seconds: 52 * 60 + 10 },
  },
  {
    id: id(3),
    slug: 'financer-adaptation-afrique-ouest',
    title: { fr: "Financer l'adaptation en Afrique de l'Ouest", en: 'Financing adaptation in West Africa' },
    debut: -30,
    duree: 90,
    extra: { capacity: 60, registered_count: 41 },
  },
  {
    id: avecFormulaireComplet?.id ?? id(4),
    slug: avecFormulaireComplet?.slug ?? 'femmes-et-negociations-climat',
    title: { fr: 'Femmes et négociations climat : dix ans de formation', en: 'Women and climate negotiations: ten years of training' },
    debut: 150,
    duree: 60,
    extra: { capacity: 80, registered_count: 23, language_codes: ['fr', 'en'] },
  },
  {
    id: id(5),
    slug: 'article-6-marches-carbone',
    title: { fr: "L'article 6 et les marchés du carbone", en: 'Article 6 and carbon markets' },
    debut: 240,
    duree: 60,
    extra: { capacity: 30, registered_count: 30, waitlist_enabled: true, waitlisted_count: 4 },
  },
  {
    id: id(6),
    slug: 'villes-francophones-resilientes',
    title: { fr: 'Villes francophones résilientes', en: 'Resilient francophone cities' },
    debut: 24 * 60 + 60,
    duree: 60,
  },
  {
    id: id(7),
    slug: 'projection-oceans-francophonie',
    title: { fr: 'Projection : les océans de la Francophonie', en: 'Screening: the oceans of the Francophonie' },
    debut: 24 * 60 + 180,
    duree: 45,
    extra: { registration_required: false },
  },
]

function ligne(m: Modele, eventId: string, maintenant: number): PublicScheduleRow {
  const debut = maintenant + m.debut * MINUTE
  const fin = debut + m.duree * MINUTE
  const temporal_state = maintenant >= fin ? 'past' : maintenant >= debut ? 'ongoing' : 'upcoming'
  return {
    id: m.id,
    event_id: eventId,
    event_day_id: null,
    proposal_id: null,
    slug: m.slug,
    title: m.title,
    summary: null,
    starts_at: new Date(debut).toISOString(),
    ends_at: new Date(fin).toISOString(),
    timezone: FUSEAU,
    format: 'hybrid',
    status: temporal_state === 'past' ? 'completed' : temporal_state === 'ongoing' ? 'live' : 'scheduled',
    room_id: null,
    room_name: { fr: 'Salle du Pavillon', en: 'Pavilion room' },
    organization_id: null,
    organization_name: 'Institut de la Francophonie pour le développement durable',
    organization_acronym: 'IFDD',
    organization_country_code: 'CA',
    organization_country: { fr: 'Canada', en: 'Canada' },
    is_streamed: true,
    broadcast_channel_id: null,
    capacity: null,
    tracks: [],
    cover: null,
    temporal_state,
    registered_count: 0,
    theme_codes: [],
    themes: [],
    waitlist_enabled: false,
    registration_required: true,
    registration_opens_at: null,
    registration_closes_at: null,
    waitlisted_count: 0,
    listing_changed_at: null,
    language_codes: ['fr'],
    replay_url: null,
    replay_duration_seconds: null,
    organization_logo: null,
    organization_type_code: 'international_organization',
    ...m.extra,
  }
}

function empreinte(valeur: unknown): string {
  let h = 7
  for (const c of JSON.stringify(valeur)) h = (h * 31 + c.charCodeAt(0)) >>> 0
  return `"${h.toString(36)}"`
}

const miennes = new Map<string, Registration>()
let numero = 0

function nouvelle(sessionId: string, statut: 'registered' | 'waitlisted', answers: Record<string, unknown>, position: number | null): Registration {
  numero += 1
  const maintenant = new Date().toISOString()
  return {
    id: `01990000-0000-7000-8000-0000000e${String(numero).padStart(4, '0')}`,
    session_id: sessionId,
    person_id: MOI,
    organization_id: null,
    status: statut,
    answers,
    locale: 'fr',
    waitlist_position: position,
    joined_at: null,
    attendance_minutes: null,
    certificate_asset_id: null,
    source: 'api',
    cancelled_at: null,
    cancelled_reason: null,
    created_at: maintenant,
    updated_at: maintenant,
  }
}

const enCours = MODELES[2]!.id
{
  const r = nouvelle(enCours, 'registered', { country: 'SN' }, null)
  miennes.set(r.id, r)
}

const vivante = (sessionId: string) => [...miennes.values()].find((r) => r.session_id === sessionId && r.status !== 'cancelled')

export function programme(eventId: string): AvecEmpreinte<PublicScheduleRow[]> {
  const maintenant = Date.now()
  const lignes = MODELES.map((m) => {
    const l = ligne(m, eventId, maintenant)
    const mienne = vivante(m.id)
    return mienne?.status === 'registered' && m.id !== enCours ? { ...l, registered_count: l.registered_count + 1 } : l
  })
  // Hors heure : l'empreinte ne bouge que si le contenu bouge, comme celle de l'API.
  return { valeur: lignes, empreinte: empreinte(lignes.map((l) => [l.id, l.status, l.registered_count])) }
}

export function activite(eventId: string, slug: string): PublicSessionDetail | null {
  const session =
    programme(eventId).valeur.find((l) => l.slug === slug) ??
    publicSchedule().find((l) => l.event_id === eventId && l.slug === slug)
  if (!session) return null
  return {
    session,
    description: { fr: "Un échange entre négociatrices, chercheurs et partenaires sur les priorités francophones de la COP.", en: 'An exchange between negotiators, researchers and partners on francophone priorities at the COP.' },
    allows_questions: session.id !== MODELES[6]!.id,
    is_recorded: true,
    speakers: [
      { id: `${session.id}-1`, session_id: session.id, role: 'moderator', job_title_snapshot: 'Spécialiste de programme', organization_snapshot: 'IFDD', bio: null, sort_order: 1, created_at: session.starts_at, display_name: 'Aïssatou Diallo', avatar: null },
      { id: `${session.id}-2`, session_id: session.id, role: 'panelist', job_title_snapshot: 'Négociatrice', organization_snapshot: 'Délégation du Sénégal', bio: null, sort_order: 2, created_at: session.starts_at, display_name: 'Mariam Ndiaye', avatar: null },
    ],
    organizations: [
      {
        session_id: session.id,
        organization_id: '01990000-0000-7000-8000-0000000b0001',
        role: 'lead',
        sort_order: 1,
        added_at: session.starts_at,
        name: 'Institut de la Francophonie pour le développement durable',
        acronym: 'IFDD',
        country_code: 'CA',
        country: { fr: 'Canada', en: 'Canada' },
      },
    ],
  }
}

export function mesInscriptions(): AvecEmpreinte<Registration[]> {
  const valeur = [...miennes.values()]
  return { valeur, empreinte: empreinte(valeur) }
}

export function inscrire(sessionId: string, payload: SessionRegisterPayload): RegistrationResult {
  const l = programme('').valeur.find((x) => x.id === sessionId)
  if (!l) throw new ApiRequestError({ code: 'NOT_FOUND', message: "Cette séance n'existe pas." }, 404)
  if (!l.registration_required || l.status === 'cancelled') {
    throw new ApiRequestError({ code: 'REGISTRATION_NOT_ACCEPTED', message: "Cette séance ne prend pas d'inscription." }, 422)
  }
  const deja = vivante(sessionId)
  if (deja) return { status: 'already_registered', registration: deja }
  if (l.registration_closes_at && Date.now() >= new Date(l.registration_closes_at).getTime()) {
    return { status: 'closed', closed_at: l.registration_closes_at }
  }
  if (l.capacity !== null && l.registered_count >= l.capacity) {
    if (!l.waitlist_enabled) return { status: 'full', capacity: l.capacity }
    const r = nouvelle(sessionId, 'waitlisted', payload.answers, (l.waitlisted_count ?? 0) + 1)
    miennes.set(r.id, r)
    return { status: 'waitlisted', registration: r, position: r.waitlist_position ?? 1 }
  }
  const r = nouvelle(sessionId, 'registered', payload.answers, null)
  miennes.set(r.id, r)
  return { status: 'registered', registration: r }
}

export function annuler(registrationId: string): CancelRegistrationResult {
  const r = miennes.get(registrationId)
  if (!r || r.status === 'cancelled') throw new ApiRequestError({ code: 'NOT_FOUND', message: 'Inscription introuvable.' }, 404)
  const annulee: Registration = { ...r, status: 'cancelled', waitlist_position: null, cancelled_at: new Date().toISOString() }
  miennes.set(registrationId, annulee)
  return { registration: annulee, promoted: 0 }
}

interface QuestionSimulee {
  question: PublicSessionQuestion
  votants: Set<string>
  auteur: string | null
}

const questionsPosees = new Map<string, QuestionSimulee>()
let numeroQuestion = 0

function simulee(sessionId: string, body: string, auteur: string | null, votants: string[], minutes: number): QuestionSimulee {
  numeroQuestion += 1
  return {
    question: {
      id: `01990000-0000-7000-8000-0000000f${String(numeroQuestion).padStart(4, '0')}`,
      session_id: sessionId,
      body,
      vote_count: 0,
      has_voted: false,
      is_mine: false,
      answered_at: null,
      created_at: new Date(Date.now() - minutes * MINUTE).toISOString(),
      answers: [],
    },
    votants: new Set(votants),
    auteur,
  }
}

for (const q of [
  simulee(enCours, "Quels mécanismes de financement sont accessibles aux collectivités locales ?", null, ['a', 'b', 'c'], 20),
  simulee(enCours, "Comment les délégations francophones se coordonnent-elles avant les sessions ?", null, [MOI], 12),
]) questionsPosees.set(q.question.id, q)

const vue = (q: QuestionSimulee): PublicSessionQuestion => ({
  ...q.question,
  vote_count: q.votants.size,
  has_voted: q.votants.has(MOI),
  is_mine: q.auteur === MOI,
})

function ouverte(sessionId: string) {
  const l = programme('').valeur.find((x) => x.id === sessionId)
  if (!l) throw new ApiRequestError({ code: 'NOT_FOUND', message: 'La ressource demandée est introuvable.' }, 404)
  if (sessionId === MODELES[6]!.id || l.status === 'cancelled') {
    throw new ApiRequestError({ code: 'CONFLICT', message: 'Cette séance ne prend pas de questions.' }, 409)
  }
}

function trouvee(sessionId: string, questionId: string): QuestionSimulee {
  const q = questionsPosees.get(questionId)
  if (!q || q.question.session_id !== sessionId) {
    throw new ApiRequestError({ code: 'NOT_FOUND', message: 'La ressource demandée est introuvable.' }, 404)
  }
  return q
}

export function questions(sessionId: string): AvecEmpreinte<PublicSessionQuestion[]> {
  if (!programme('').valeur.some((l) => l.id === sessionId)) {
    throw new ApiRequestError({ code: 'NOT_FOUND', message: 'La ressource demandée est introuvable.' }, 404)
  }
  const valeur = [...questionsPosees.values()]
    .filter((q) => q.question.session_id === sessionId)
    .map(vue)
    .sort((a, b) => b.vote_count - a.vote_count || a.created_at.localeCompare(b.created_at))
  return { valeur, empreinte: empreinte(valeur) }
}

export function poserQuestion(sessionId: string, payload: AskQuestionPayload): PublicSessionQuestion {
  ouverte(sessionId)
  const body = payload.body.trim()
  if (body.length < 3 || body.length > 2000) {
    throw new ApiRequestError({ code: 'VALIDATION_FAILED', message: 'Une question compte entre 3 et 2000 caractères.', field: 'body' }, 422)
  }
  const q = simulee(sessionId, body, MOI, [], 0)
  questionsPosees.set(q.question.id, q)
  return vue(q)
}

export function voter(sessionId: string, questionId: string): PublicSessionQuestion {
  ouverte(sessionId)
  const q = trouvee(sessionId, questionId)
  if (q.votants.has(MOI)) throw new ApiRequestError({ code: 'CONFLICT', message: 'Vous soutenez déjà cette question.' }, 409)
  q.votants.add(MOI)
  return vue(q)
}

export function retirerVote(sessionId: string, questionId: string): PublicSessionQuestion {
  const q = trouvee(sessionId, questionId)
  q.votants.delete(MOI)
  return vue(q)
}
