/**
 * Les sessions de négociation, sans API : deux jours de la COP31 d'exemple, avec
 * chacun des cas qu'un écran doit savoir dire — déplacée, annulée, sans fin,
 * sans thématique, coordination rattachée ou non. Chargé à la demande.
 */
import type { AvecEmpreinte } from '~/composables/api/etiquete'
import type { MyAgenda, MyAgendaEntry, MyGroups, OfficialSession, OfficialSessions } from '~/types/negotiation-sessions'

const SLUG = 'cop31-belem-2027'
const LU_A = '2027-11-10T09:00:00Z'

const type = (code: string, fr: string, en: string) => ({ code, label: { fr, en }, term_en: en })
const PLENIERE = type('plenary', 'Plénière', 'Plenary')
const CONTACT = type('contact_group', 'Groupe de contact', 'Contact group')
const INFORMELLES = type('informal_consultations', 'Consultations informelles', 'Informal consultations')
const COORDINATION = type('group_coordination', 'Coordination de groupe', 'Coordination meeting')

const base = {
  previous: null,
  type: null,
  group: null,
  theme: null,
  agenda_item: null,
  open_access: null,
  status: 'scheduled' as const,
  cancelled: null,
  source_url: 'https://unfccc.int/cop31',
  read_at: LU_A,
  network_reports: [],
}

const SESSIONS: OfficialSession[] = [
  {
    ...base,
    id: '01937a00-0000-7000-8000-000000000301',
    title_en: 'Opening plenary of the SBSTA',
    title_fr: 'Plénière d’ouverture du SBSTA',
    start_at: '2027-11-10T13:00:00Z',
    end_at: '2027-11-10T15:00:00Z',
    venue: 'Plenary Amazonas',
    type: PLENIERE,
    open_access: true,
  },
  {
    ...base,
    id: '01937a00-0000-7000-8000-000000000302',
    title_en: 'Global goal on adaptation',
    title_fr: 'Objectif mondial en matière d’adaptation',
    start_at: '2027-11-10T14:00:00Z',
    end_at: '2027-11-10T16:00:00Z',
    venue: 'Room Tocantins',
    previous: { start_at: '2027-11-10T12:00:00Z', end_at: '2027-11-10T14:00:00Z', venue: 'Room Xingu', changed_at: LU_A },
    type: INFORMELLES,
    theme: 'adaptation',
    agenda_item: { code: 'SBSTA 4', title: 'Global goal on adaptation' },
    open_access: false,
  },
  {
    ...base,
    id: '01937a00-0000-7000-8000-000000000303',
    title_en: 'New collective quantified goal on climate finance',
    title_fr: 'Nouvel objectif collectif quantifié de financement climatique',
    start_at: '2027-11-10T17:00:00Z',
    end_at: null,
    venue: 'Room Tapajós',
    type: CONTACT,
    theme: 'finance',
    open_access: false,
  },
  {
    ...base,
    id: '01937a00-0000-7000-8000-000000000304',
    title_en: 'Gender action plan',
    title_fr: null,
    start_at: '2027-11-10T18:00:00Z',
    end_at: '2027-11-10T19:00:00Z',
    venue: 'Room Xingu',
    type: INFORMELLES,
    theme: 'gender',
    status: 'cancelled',
    cancelled: { at: LU_A, reason: 'source' },
  },
  {
    ...base,
    id: '01937a00-0000-7000-8000-000000000305',
    title_en: 'African Group of Negotiators coordination meeting',
    title_fr: 'Réunion de coordination du Groupe africain des négociateurs',
    start_at: '2027-11-11T11:00:00Z',
    end_at: '2027-11-11T12:30:00Z',
    venue: 'Room Negro',
    type: COORDINATION,
    group: { code: 'african_group', label: { fr: 'Groupe africain', en: 'African Group' } },
  },
  {
    ...base,
    id: '01937a00-0000-7000-8000-000000000306',
    title_en: 'Observer constituencies coordination',
    title_fr: 'Coordination des observateurs',
    start_at: '2027-11-11T11:00:00Z',
    end_at: '2027-11-11T12:00:00Z',
    venue: null,
    type: COORDINATION,
  },
  {
    ...base,
    id: '01937a00-0000-7000-8000-000000000307',
    title_en: 'Presidency open-ended consultations',
    title_fr: 'Consultations ouvertes de la présidence',
    start_at: '2027-11-11T15:00:00Z',
    end_at: '2027-11-11T17:00:00Z',
    venue: 'Plenary Amazonas',
    type: PLENIERE,
    open_access: true,
  },
]

let groupes: string[] = []
let agenda: MyAgendaEntry[] = []

export function sessionsOfficielles(): AvecEmpreinte<OfficialSessions> {
  return {
    valeur: {
      edition: { slug: SLUG, timezone: 'America/Belem', city: 'Belém' },
      official_programme_url: 'https://unfccc.int/cop31',
      state: 'serving',
      cut_reason: null,
      failing_since: null,
      read_at: LU_A,
      server_time: new Date().toISOString(),
      sessions: SESSIONS.map((s) => ({ ...s })),
      network_meetings: [],
    },
    empreinte: '"sessions-exemple"',
  }
}

const empreinteDesGroupes = () => `"${[...groupes].sort().join('.')}"`

export function mesGroupes(): AvecEmpreinte<MyGroups> {
  const etag = empreinteDesGroupes()
  return { valeur: { groups: [...groupes], etag }, empreinte: etag }
}

export function suivreDesGroupes(codes: string[]): AvecEmpreinte<MyGroups> {
  groupes = [...new Set(codes)].sort()
  return mesGroupes()
}

export function monAgenda(): AvecEmpreinte<MyAgenda> {
  const annulees = new Set(SESSIONS.filter((s) => s.status === 'cancelled').map((s) => s.id))
  const entries = agenda.map((e) => ({ ...e, remind: e.remind && !annulees.has(e.session_id) }))
  return { valeur: { entries, network_entries: [] }, empreinte: `"${entries.map((e) => `${e.session_id}:${e.remind}`).join('.')}"` }
}

export function garderUneSession(sessionId: string, remind: boolean): void {
  const existante = agenda.find((e) => e.session_id === sessionId)
  agenda = [
    ...agenda.filter((e) => e.session_id !== sessionId),
    { session_id: sessionId, remind, added_at: existante?.added_at ?? new Date().toISOString() },
  ]
}

export function retirerUneSession(sessionId: string): void {
  agenda = agenda.filter((e) => e.session_id !== sessionId)
}
