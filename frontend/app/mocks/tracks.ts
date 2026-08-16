/**
 * Données simulées de `event.programme_tracks` — les journées spéciales.
 *
 * UNE JOURNÉE SPÉCIALE N'EST PAS UN JOUR DU CALENDRIER. C'est un fil composé À
 * LA MAIN par l'IFDD parmi les activités retenues, et le rattachement de chaque
 * session est explicite (`programme.session_tracks`, voir `mocks/sessions/`).
 * Trois raisons, toutes vérifiables dans ce jeu de données :
 *
 *   - « Journée finance durable » n'occupe qu'une partie du 12 novembre ;
 *   - le 16 novembre porte à la fois la journée jeunesse et des activités qui
 *     n'en font pas partie ;
 *   - « Journée jeunesse et climat » déborde sur le 17 novembre au matin.
 *
 * Les dates portées ici sont PUREMENT INDICATIVES : elles annoncent au public la
 * portée du fil, elles ne contraignent pas les rattachements.
 */

import type { ProgrammeTrack } from '~/types/event/edition'
import { EVENT, PERSON, TRACK } from './ids'

export const programmeTracks = [
  {
    id: TRACK.financeDurable,
    event_id: EVENT.cop31,
    code: 'finance_durable',
    slug: 'journee-finance-durable',
    kind: 'special_day',
    title: { fr: 'Journée finance durable', en: 'Sustainable finance day' },
    subtitle: {
      fr: "Accéder aux financements climatiques sans intermédiaire",
      en: 'Accessing climate finance without intermediaries',
    },
    description: {
      fr: "Une demi-journée consacrée à l'accès direct aux fonds climatiques : accréditation auprès du Fonds vert, opérationnalisation du fonds pertes et préjudices, garanties d'intégrité des marchés carbone. Les activités sont choisies parmi les propositions retenues ; elles ne couvrent pas l'ensemble du 12 novembre.",
      en: 'Half a day on direct access to climate funds: Green Climate Fund accreditation, loss and damage fund, carbon market integrity.',
    },
    starts_on: '2027-11-12',
    ends_on: '2027-11-12',
    color_hex: '#1F5F8B',
    curated_by: PERSON.duchesne,
    published_at: '2026-08-03T14:00:00Z',
    sort_order: 10,
    created_at: '2026-07-06T09:30:00Z',
    updated_at: '2026-08-03T14:00:00Z',
  },
  {
    id: TRACK.jeunesseClimat,
    event_id: EVENT.cop31,
    code: 'jeunesse_climat',
    slug: 'journee-jeunesse-climat',
    kind: 'special_day',
    title: { fr: 'Journée jeunesse et climat', en: 'Youth and climate day' },
    subtitle: {
      fr: "La relève francophone dans les négociations",
      en: 'The next French-speaking generation in negotiations',
    },
    description: {
      fr: "Formation à la négociation, plaidoyer et prise de parole des jeunes délégations francophones. Le fil commence le 16 novembre après-midi et se poursuit le 17 au matin : une journée spéciale ne s'aligne pas sur les bornes d'un jour de calendrier.",
      en: 'Negotiation training, advocacy and public speaking for young French-speaking delegations.',
    },
    starts_on: '2027-11-16',
    ends_on: '2027-11-17',
    color_hex: '#8C6D1F',
    curated_by: PERSON.nkoDiop,
    published_at: '2026-08-03T14:00:00Z',
    sort_order: 20,
    created_at: '2026-07-06T09:45:00Z',
    updated_at: '2026-08-03T14:00:00Z',
  },
] satisfies ProgrammeTrack[]
