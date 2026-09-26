/**
 * Les signalements du réseau, sans API : les trois états dans « Mes signalements »,
 * deux en attente dans la file de validation, un encart publié, deux réunions non
 * annoncées. Chargé à la demande, comme les sessions dont il partage les identifiants.
 *
 * Comme l'API, rien n'est public avant la publication : une validation d'ici paraît
 * trente secondes plus tard, et s'annule tant qu'elle n'a pas paru.
 */
import type { ApiErrorCode } from '~/types/api-error'
import type { NotificationSettings } from '~/types/negotiation'
import type {
  MyReport,
  MyReports,
  ReportPayload,
  ReportQueue,
  ReportQueueItem,
  RejectPayload,
} from '~/types/negotiation-reports'
import type { NetworkMeeting, NetworkReport } from '~/types/negotiation-sessions'
import { ApiRequestError } from '~/utils/api-error'
import { sessionDExemple, sessionsOfficielles } from './negotiation-sessions'

const PUBLICATION_MS = 30_000
const S = (n: number) => `01937a00-0000-7000-8000-000000000${n}`
const R = (n: number) => `01937a00-0000-7000-8000-0000000009${String(n).padStart(2, '0')}`

interface Exemple extends ReportQueueItem {
  /** Déposé par la personne connectée : il paraît dans « Mes signalements ». */
  mien: boolean
  reunion_id: string | null
}

const refus = (code: ApiErrorCode, status: number, message: string, field?: string) =>
  new ApiRequestError({ code, message, field }, status)

function exemple(n: number, champs: Partial<Exemple> & Pick<Exemple, 'reason' | 'submitted_at'>): Exemple {
  return {
    id: R(n),
    client_ref: `01937a00-0000-4000-8000-0000000009${String(n).padStart(2, '0')}`,
    session: null,
    what: null,
    proposed_start: null,
    proposed_venue: null,
    day: null,
    theme: null,
    network_meeting_id: null,
    detail: null,
    status: 'submitted',
    decided_at: null,
    reject_reason: null,
    reject_detail: null,
    author: { name: 'Awa Diop', country: 'Sénégal' },
    source_now: null,
    decided_by: null,
    published_at: null,
    withdrawn_at: null,
    mien: false,
    reunion_id: null,
    ...champs,
  }
}

let signalements: Exemple[] | null = null

function semer(): Exemple[] {
  const valide = (decided_at: string) => ({
    status: 'validated' as const,
    decided_at,
    decided_by: 'Équipe IFDD',
    published_at: new Date(Date.parse(decided_at) + PUBLICATION_MS).toISOString(),
  })
  return [
    exemple(1, {
      reason: 'venue',
      session: sessionDExemple(S(303)),
      proposed_venue: 'Room Negro',
      detail: 'Affiché à l’entrée de la salle Tapajós.',
      submitted_at: '2027-11-10T11:50:00Z',
      ...valide('2027-11-10T12:10:00Z'),
    }),
    exemple(2, {
      reason: 'unannounced',
      what: 'Coordination francophone sur l’article 6.4',
      proposed_venue: 'Pavillon de la Francophonie',
      proposed_start: '2027-11-11T20:00:00Z',
      day: '2027-11-11',
      theme: 'article_6',
      submitted_at: '2027-11-10T14:00:00Z',
      mien: true,
      author: { name: 'Vous', country: null },
      reunion_id: '01937a00-0000-7000-8000-000000000981',
      ...valide('2027-11-10T14:20:00Z'),
    }),
    exemple(3, {
      reason: 'unannounced',
      what: 'Point des négociatrices du Sahel',
      proposed_venue: 'Salle Xingu, côté B',
      day: '2027-11-10',
      theme: 'adaptation',
      submitted_at: '2027-11-10T09:30:00Z',
      author: { name: 'Mariam Traoré', country: 'Mali' },
      reunion_id: '01937a00-0000-7000-8000-000000000982',
      ...valide('2027-11-10T10:05:00Z'),
    }),
    exemple(4, {
      reason: 'cancelled',
      session: sessionDExemple(S(307)),
      submitted_at: '2027-11-10T13:40:00Z',
      mien: true,
      author: { name: 'Vous', country: null },
    }),
    exemple(5, {
      reason: 'time',
      session: sessionDExemple(S(301)),
      proposed_start: '2027-11-10T14:00:00Z',
      submitted_at: '2027-11-10T08:15:00Z',
      mien: true,
      author: { name: 'Vous', country: null },
      status: 'rejected',
      decided_at: '2027-11-10T08:40:00Z',
      decided_by: 'Équipe IFDD',
      reject_reason: 'source_maintains',
    }),
    exemple(6, {
      reason: 'other',
      session: sessionDExemple(S(305)),
      detail: 'La salle a été divisée en deux ; entrée par le couloir nord.',
      submitted_at: '2027-11-10T12:30:00Z',
      author: { name: 'Jean-Baptiste Kaboré', country: 'Burkina Faso' },
    }),
  ]
}

const tous = () => (signalements ??= semer())

/** L'horloge de la publication : un travail qui aurait tourné trente secondes après la validation. */
function publier(): Exemple[] {
  const maintenant = Date.now()
  for (const s of tous()) {
    if (s.status === 'validated' && !s.published_at && s.decided_at && maintenant - Date.parse(s.decided_at) >= PUBLICATION_MS) {
      s.published_at = new Date(Date.parse(s.decided_at) + PUBLICATION_MS).toISOString()
      if (s.reason === 'unannounced') s.reunion_id ??= `01937a00-0000-7000-8000-${String(maintenant).slice(-12)}`
    }
  }
  return tous()
}

const affiche = (s: Exemple) => s.status === 'validated' && s.published_at !== null && s.withdrawn_at === null

function champsCommuns(s: Exemple): MyReport {
  return {
    id: s.id,
    client_ref: s.client_ref,
    reason: s.reason,
    session: s.session,
    what: s.what,
    proposed_start: s.proposed_start,
    proposed_venue: s.proposed_venue,
    day: s.day,
    theme: s.theme,
    network_meeting_id: affiche(s) ? s.reunion_id : null,
    detail: s.detail,
    status: s.status,
    submitted_at: s.submitted_at,
    decided_at: s.decided_at,
    reject_reason: s.reject_reason,
    reject_detail: s.reject_detail,
  }
}

function vuParLAutrice(s: Exemple): MyReport {
  const lu = champsCommuns(s)
  // Validé mais pas encore paru : l'autrice le voit encore « envoyé » (R3).
  return s.status === 'validated' && !s.published_at ? { ...lu, status: 'submitted', decided_at: null } : lu
}

function vuParLaFile(s: Exemple): ReportQueueItem {
  const officielle = s.session ? sessionsOfficielles().valeur.sessions.find((x) => x.id === s.session?.id) : null
  return {
    ...champsCommuns(s),
    author: s.author,
    decided_by: s.decided_by,
    published_at: s.published_at,
    withdrawn_at: s.withdrawn_at,
    source_now: officielle
      ? { status: officielle.status, start_at: officielle.start_at, end_at: officielle.end_at, venue: officielle.venue, read_at: officielle.read_at }
      : null,
  }
}

export function encartsPublies(sessionId: string): NetworkReport[] {
  return publier()
    .filter((s) => affiche(s) && s.reason !== 'unannounced' && s.session?.id === sessionId)
    .map((s) => ({
      reason: s.reason as NetworkReport['reason'],
      proposed_start: s.proposed_start,
      proposed_venue: s.proposed_venue,
      detail: s.detail,
      validated_at: s.decided_at as string,
    }))
}

export function reunionsPubliees(): NetworkMeeting[] {
  return publier()
    .filter((s) => affiche(s) && s.reason === 'unannounced' && s.reunion_id && s.day)
    .map((s) => ({
      id: s.reunion_id as string,
      title: s.what ?? '',
      venue: s.proposed_venue,
      start_at: s.proposed_start,
      day: s.day as string,
      theme: s.theme,
      validated_at: s.decided_at as string,
    }))
}

export function mesSignalements(): MyReports {
  return {
    reports: publier()
      .filter((s) => s.mien)
      .sort((a, b) => Date.parse(b.submitted_at) - Date.parse(a.submitted_at))
      .map(vuParLAutrice),
  }
}

export function signaler(p: ReportPayload): MyReport {
  const deja = tous().find((s) => s.mien && s.client_ref === p.client_ref)
  if (deja) return vuParLAutrice(deja)

  const nonAnnoncee = p.reason === 'unannounced'
  if (nonAnnoncee && !p.what?.trim()) throw refus('NEGOTIATION_REPORT_INVALID', 400, 'Dites de quelle réunion il s’agit.', 'what')
  if (nonAnnoncee && !p.day) throw refus('NEGOTIATION_REPORT_INVALID', 400, 'Précisez le jour de la réunion.', 'day')
  if ((p.detail?.length ?? 0) > 600) throw refus('NEGOTIATION_REPORT_INVALID', 400, 'La précision dépasse 600 caractères.', 'detail')
  const session = p.session_id ? sessionDExemple(p.session_id) : null
  if (!nonAnnoncee && !session) throw refus('NEGOTIATION_SESSION_UNKNOWN', 404, "Cette session de négociation n'existe pas.")
  if (session && tous().some((s) => s.mien && s.status === 'submitted' && s.session?.id === session.id)) {
    throw refus('NEGOTIATION_REPORT_DUPLICATE', 409, 'Vous avez déjà signalé cette session : votre signalement est en cours de vérification.')
  }

  const nouveau = exemple(tous().length + 1, {
    id: `01937a00-0000-7000-8000-${String(Date.now()).slice(-12)}`,
    client_ref: p.client_ref,
    reason: p.reason,
    session,
    what: p.what ?? null,
    proposed_start: p.proposed_start ?? null,
    proposed_venue: p.proposed_venue ?? null,
    day: p.day ?? null,
    detail: p.detail ?? null,
    theme: p.theme ?? null,
    submitted_at: new Date().toISOString(),
    mien: true,
    author: { name: 'Vous', country: null },
  })
  tous().push(nouveau)
  return vuParLAutrice(nouveau)
}

export function fileDeValidation(): ReportQueue {
  const lus = publier()
  return {
    pending: lus
      .filter((s) => s.status === 'submitted')
      .sort((a, b) => Date.parse(a.submitted_at) - Date.parse(b.submitted_at))
      .map(vuParLaFile),
    decided_today: lus
      .filter((s) => s.decided_at)
      .sort((a, b) => Date.parse(b.decided_at as string) - Date.parse(a.decided_at as string))
      .map(vuParLaFile),
  }
}

function trouver(id: string): Exemple {
  const s = publier().find((x) => x.id === id)
  if (!s) throw refus('NEGOTIATION_REPORT_UNKNOWN', 404, "Ce signalement n'existe pas.")
  return s
}

const dejaTranche = () => refus('NEGOTIATION_REPORT_ALREADY_DECIDED', 409, 'Ce signalement a déjà été tranché.')

export function valider(id: string): ReportQueueItem {
  const s = trouver(id)
  if (s.status !== 'submitted') throw dejaTranche()
  Object.assign(s, { status: 'validated', decided_at: new Date().toISOString(), decided_by: 'Vous', published_at: null })
  return vuParLaFile(s)
}

export function annuler(id: string): ReportQueueItem {
  const s = trouver(id)
  if (s.status !== 'validated' || s.published_at) {
    throw refus('NEGOTIATION_REPORT_UNDO_EXPIRED', 409, 'Trop tard pour annuler : le signalement est déjà affiché.')
  }
  Object.assign(s, { status: 'submitted', decided_at: null, decided_by: null })
  return vuParLaFile(s)
}

export function refuser(id: string, corps: RejectPayload): ReportQueueItem {
  const s = trouver(id)
  if (s.status !== 'submitted') throw dejaTranche()
  Object.assign(s, {
    status: 'rejected',
    decided_at: new Date().toISOString(),
    decided_by: 'Vous',
    reject_reason: corps.reason,
    reject_detail: corps.detail ?? null,
  })
  return vuParLaFile(s)
}

export function retirer(id: string): ReportQueueItem {
  const s = trouver(id)
  s.withdrawn_at ??= new Date().toISOString()
  return vuParLaFile(s)
}

// ---------------------------------------------------------------------------
// L'accord « Notifications » d'« À propos »
// ---------------------------------------------------------------------------

let courriel = true

export const reglageDesNotifications = (): NotificationSettings => ({ email: courriel, version: '2026-09' })

export function reglerLesNotifications(email: boolean): NotificationSettings {
  courriel = email
  return reglageDesNotifications()
}
