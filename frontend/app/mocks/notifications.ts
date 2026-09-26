/**
 * Le fil de notifications de Guide Négo, sans API : un changement lu par l'import,
 * un signalement publié, une réunion non annoncée, une décision déjà lue. Les heures
 * sont relatives à l'ouverture, pour que le fil ait toujours un « aujourd'hui ».
 */
import type { Notification, NotificationFeed } from '~/types/engagement'

const ilYA = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString()

function avis(
  id: string,
  type_code: string,
  title: Notification['title'],
  body: Notification['body'],
  link_path: string,
  created_at: string,
  read_at: string | null = null,
): Notification {
  return {
    id,
    type_code,
    title,
    body,
    variables: {},
    link_path,
    subject_schema: 'negotiation',
    subject_table: 'meetings',
    subject_id: null,
    group_count: 1,
    read_at,
    created_at,
  }
}

let fil: Notification[] | null = null

const semer = (): Notification[] => [
  avis(
    '01937a00-0000-7000-8000-000000000a01',
    'negotiation.meeting.changed',
    { fr: 'Déplacée — Objectif mondial en matière d’adaptation', en: 'Moved — Global goal on adaptation' },
    { fr: 'Salle Tocantins · Sessions de négociation', en: 'Room Tocantins · Negotiation sessions' },
    '/guide-nego/negociations/01937a00-0000-7000-8000-000000000302',
    ilYA(12),
  ),
  avis(
    '01937a00-0000-7000-8000-000000000a02',
    'negotiation.report.published',
    {
      fr: 'Salle changée — Nouvel objectif collectif quantifié, signalé par le réseau',
      en: 'Room changed — New collective quantified goal, reported by the network',
    },
    { fr: 'Room Negro · Sessions de négociation', en: 'Room Negro · Negotiation sessions' },
    '/guide-nego/negociations/01937a00-0000-7000-8000-000000000303',
    ilYA(95),
  ),
  avis(
    '01937a00-0000-7000-8000-000000000a03',
    'negotiation.network_meeting.published',
    { fr: 'Non annoncée — Coordination francophone sur l’article 6.4', en: 'Unannounced — Francophone coordination on Article 6.4' },
    { fr: 'Pavillon de la Francophonie · Sessions de négociation', en: 'Francophonie Pavilion · Negotiation sessions' },
    '/guide-nego/negociations/reseau/01937a00-0000-7000-8000-000000000981',
    ilYA(60 * 26),
  ),
  avis(
    '01937a00-0000-7000-8000-000000000a04',
    'negotiation.report.decided',
    { fr: 'Votre signalement n’a pas été retenu', en: 'Your report was not accepted' },
    { fr: 'Plénière d’ouverture du SBSTA · Sessions de négociation', en: 'Opening plenary of the SBSTA · Negotiation sessions' },
    '/guide-nego/negociations/signalements',
    ilYA(60 * 27),
    ilYA(60 * 25),
  ),
]

const tous = () => (fil ??= semer())

export function filDeNotifications(limite: number): NotificationFeed {
  const items = [...tous()].sort((a, b) => Date.parse(b.created_at) - Date.parse(a.created_at))
  return { items: items.slice(0, limite).map((n) => ({ ...n })), unread_count: items.filter((n) => !n.read_at).length }
}

export function marquerLues(ids: string[]): { marked: number } {
  const maintenant = new Date().toISOString()
  let marked = 0
  for (const n of tous()) {
    if (!n.read_at && ids.includes(n.id)) {
      n.read_at = maintenant
      marked += 1
    }
  }
  return { marked }
}
