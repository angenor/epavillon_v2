/**
 * Pages publiques des fils de la CdP31 (Antalya, 2026) : l'atelier préparatoire
 * francophone (veille, à l'hôtel), la journée finance durable (13 novembre) et
 * la journée action jeunesse pour le climat (14 novembre). Le contenu est celui
 * des maquettes retenues, `docs/maquettes/journees-speciales/2-bandeau-et-billet/`.
 *
 * Une seule source pour deux formes : `trackPages` (GET /events/{slug}/tracks/{slug})
 * et `trackPageSettings` (GET /admin/tracks/{id}/page), indexés par slug du fil.
 * Heures d'Antalya (UTC+3), écrites avec leur décalage.
 *
 * Les trois temps de la jeunesse sont une proposition de l'IFDD : le programme
 * de la journée ne les nomme pas. Les textes de présentation sont provisoires.
 */

import type { I18nText, IsoDateTime } from '~/types/shared'
import type { AttachedImage } from '~/types/media'
import type { PublicScheduleRow } from '~/types/views'
import type { PublicSessionSpeaker, SessionOrganization } from '~/types/programme/session'
import type {
  PublicScheduleTrack,
  PublicTrackEntry,
  PublicTrackPage,
  PublicTrackPart,
  PublicTrackPartner,
  PublicTrackSummary,
  PublicTrackVenue,
} from '~/types/programme/track-page'
import type { TrackPageSettings } from '~/types/event/track-page'
import {
  TRACK_PAGE,
  TRACK_PAGE_ASSET,
  TRACK_PAGE_MOMENT,
  TRACK_PAGE_ORG,
  TRACK_PAGE_PART,
  TRACK_PAGE_SESSION,
  TRACK_PAGE_SPEAKER,
} from './ids'

const FUSEAU = 'Europe/Istanbul'
const fr = (texte: string): I18nText => ({ fr: texte })
const a = (jour: number, heure: string): IsoDateTime =>
  `2026-11-${String(jour).padStart(2, '0')}T${heure}:00+03:00`

const edition = {
  id: TRACK_PAGE.edition,
  slug: 'cop31-antalya',
  title: { fr: 'CdP31, Antalya', en: 'COP31, Antalya' },
  acronym: 'CdP31',
  timezone: FUSEAU,
  starts_at: a(9, '09:00'),
  ends_at: a(20, '18:00'),
  city: 'Antalya',
  country_name: { fr: 'Turquie', en: 'Türkiye' },
}

const pavillon: PublicTrackVenue = {
  id: TRACK_PAGE.pavillon,
  name: { fr: 'Pavillon de la Francophonie', en: 'Francophonie Pavilion' },
  kind: 'pavilion',
  address: null,
  map_url: null,
  area: { fr: 'Zone bleue', en: 'Blue zone' },
  access_note: null,
  latitude: null,
  longitude: null,
}

const hotel: PublicTrackVenue = {
  id: TRACK_PAGE.hotel,
  name: fr('Hôtel DoubleTree by Hilton'),
  kind: 'other',
  address: 'Etiler, Adnan Menderes Blv. No:61, 07010 Muratpaşa / Antalya, Turquie',
  map_url: 'https://maps.google.com/?q=DoubleTree+by+Hilton+Antalya+City+Centre',
  area: null,
  access_note: fr('En ville, hors de la zone bleue. La CdP31 ouvre le lendemain.'),
  latitude: 36.8761,
  longitude: 30.7202,
}

// --- Organisations ----------------------------------------------------------

interface Org { id: string; name: string; acronym: string | null }
const org = (n: number, name: string, acronym: string | null = null): Org => ({
  id: TRACK_PAGE_ORG(n),
  name,
  acronym,
})

const O = {
  oif: org(1, 'Organisation internationale de la Francophonie', 'OIF'),
  ccnucc: org(2, 'Secrétariat de la CCNUCC', 'CCNUCC'),
  giz: org(3, 'Deutsche Gesellschaft für Internationale Zusammenarbeit', 'GIZ'),
  fondsAdaptation: org(4, "Fonds pour l'adaptation"),
  // « GCF » et « Fonds vert climat » : une seule fiche (règle n° 1).
  fondsVert: org(5, 'Fonds vert pour le climat', 'GCF'),
  ocde: org(6, 'Organisation de coopération et de développement économiques', 'OCDE'),
  cif: org(7, "Fonds d'investissement climatiques, Banque mondiale", 'CIF'),
  cilss: org(8, 'Comité permanent inter-États de lutte contre la sécheresse dans le Sahel', 'CILSS'),
  airf: org(9, 'Association internationale des régions francophones', 'AIRF'),
  sakete: org(10, 'Commune de Sakété, Bénin'),
  menteSenegal: org(11, "Ministère de l'Environnement et de la Transition écologique, Sénégal"),
  businessMaurice: org(12, 'Business Maurice'),
  ndcp: org(13, 'NDC Partnership'),
  gggi: org(14, 'Global Green Growth Institute', 'GGGI'),
  quebec: org(15, 'Gouvernement du Québec'),
  qjm: org(16, 'Québec jeunesse monde', 'QJM'),
  greenOpenLab: org(17, 'Fondation Green OpenLab'),
  jveRca: org(18, 'JVE Centrafrique'),
  youthCenter: org(19, 'Youth Center for impact'),
  airEtMoi: org(20, "Fédération l'Air et Moi"),
  youthVoices: org(21, 'Youth Voices Cameroun'),
  dakiFarm: org(22, 'Daki Farm NRCI'),
  cegepGaspesie: org(23, 'Cégep de la Gaspésie et des Îles'),
  genreClimatSn: org(24, 'Sénégal : Genre et Climat'),
}

const roles: Record<string, I18nText> = {
  organizer: { fr: 'Organisateur', en: 'Organiser' },
  partner: { fr: 'Partenaire', en: 'Partner' },
  financial_support: { fr: 'Soutien financier', en: 'Financial support' },
}

const partenaire = (o: Org, role_code: keyof typeof roles): PublicTrackPartner => ({
  organization_id: o.id,
  name: o.name,
  acronym: o.acronym,
  logo: null,
  role_code,
  role_label: roles[role_code]!,
})

// --- Activités ---------------------------------------------------------------

interface Activite {
  n: number
  jour: number
  debut: string
  fin: string
  titre: string
  points?: string[]
  orgs?: Org[]
  intervenants?: Array<{ nom: string; fonction: string }>
  partie: string | null
  tempsFort?: boolean
  diffusee?: boolean
}

const ligne = (x: Activite, track: { slug: string; title: I18nText; color: string | null; kind: 'special_day' | 'side_programme' }): PublicScheduleRow => {
  const porteur = x.orgs?.[0] ?? null
  return {
    id: TRACK_PAGE_SESSION(x.n),
    event_id: TRACK_PAGE.edition,
    event_day_id: null,
    proposal_id: null,
    slug: x.titre.toLowerCase().normalize('NFD').replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '').slice(0, 60),
    title: fr(x.titre),
    // Les puces de la maquette (« Mot d'ouverture, OIF ») : le résumé de l'activité, une ligne par point.
    summary: x.points ? fr(x.points.join('\n')) : null,
    starts_at: a(x.jour, x.debut),
    ends_at: a(x.jour, x.fin),
    timezone: FUSEAU,
    format: 'hybrid',
    status: 'scheduled',
    room_id: null,
    room_name: null,
    organization_id: porteur?.id ?? null,
    organization_name: porteur?.name ?? null,
    organization_acronym: porteur?.acronym ?? null,
    organization_country_code: null,
    organization_country: null,
    is_streamed: x.diffusee ?? true,
    broadcast_channel_id: null,
    capacity: null,
    tracks: [{ slug: track.slug, title: track.title, color: track.color, kind: track.kind }],
    cover: null,
    temporal_state: 'upcoming',
    registered_count: 0,
    theme_codes: [],
    themes: [],
  }
}

const organisations = (x: Activite): SessionOrganization[] =>
  (x.orgs ?? []).map((o, i) => ({
    session_id: TRACK_PAGE_SESSION(x.n),
    organization_id: o.id,
    role: i === 0 ? 'lead' : 'co_organizer',
    sort_order: i,
    added_at: '2026-10-20T10:00:00Z',
    name: o.name,
    acronym: o.acronym,
    country_code: null,
    country: null,
    logo: null,
  }))

let compteurIntervenant = 0
const intervenants = (x: Activite): PublicSessionSpeaker[] =>
  (x.intervenants ?? []).map((p, i) => ({
    id: TRACK_PAGE_SPEAKER(++compteurIntervenant),
    session_id: TRACK_PAGE_SESSION(x.n),
    role: 'speaker',
    job_title_snapshot: p.fonction,
    organization_snapshot: null,
    bio: null,
    sort_order: i,
    created_at: '2026-10-20T10:00:00Z',
    display_name: p.nom,
    avatar: null,
  }))

interface Moment { n: number; jour: number; debut: string; fin: string | null; libelle: string; partie: string | null; points?: string[] }

interface Source {
  id: string
  slug: string
  code: string
  kind: 'special_day' | 'side_programme'
  title: I18nText
  subtitle: I18nText
  description: I18nText
  tagline: I18nText | null
  jour: number
  color: string | null
  venue: PublicTrackVenue
  cover: AttachedImage | null
  publics: string[]
  support: string | null
  partenaires: PublicTrackPartner[]
  feature: PublicTrackPage['feature']
  documents: PublicTrackPage['documents']
  parties: Array<{ cle: string; label: string | null; title: string | null; summary: string | null }>
  activites: Activite[]
  moments: Moment[]
}

const PUBLICS = [
  'Négociateurs',
  'Points focaux de la CCNUCC',
  'Décideurs politiques',
  'Experts',
  'ONG',
  'Secteur privé',
  'Collectivités territoriales',
  'Organisations internationales et régionales',
]

const photo = (n: number, alt: string): AttachedImage => ({
  asset_id: TRACK_PAGE_ASSET(n),
  url: `https://media.epavillon.example/2026/11/journees/${n}.jpg`,
  width: 1920,
  height: 1080,
  alt_text: fr(alt),
  caption: null,
  credit: 'IFDD',
  sources: {},
})

const atelier: Source = {
  id: TRACK_PAGE.atelier,
  slug: 'atelier-preparatoire',
  code: 'atelier_preparatoire',
  kind: 'side_programme',
  title: fr('Atelier préparatoire francophone des négociateurs-trices et des représentant(e)s de la société civile'),
  subtitle: fr("Une synthèse actualisée des enjeux de la CdP31, la veille de l'ouverture."),
  description: fr(
    "Permettre aux négociateurs et autres parties prenantes francophones à la CdP31 d'avoir une synthèse actualisée des enjeux : les principaux enjeux des négociations, la mise en œuvre de l'Accord de Paris, les positions des principales Parties.\n\nUn temps d'échange entre négociateurs, délégués, ONG, secteur privé, collectivités et groupes régionaux.",
  ),
  tagline: null,
  jour: 8,
  color: null,
  venue: hotel,
  cover: null,
  publics: PUBLICS,
  support: null,
  partenaires: [partenaire(O.oif, 'organizer')],
  feature: {
    kicker: fr("Présenté pendant l'atelier"),
    title: fr('Guide des négociations de la CdP31'),
    body: fr(
      "Le Guide et son Résumé sont présentés l'après-midi par Dr Issa Bado et M. Serges Djohy. Le Guide se lit aussi dans l'application Guide Négo, au fil des sessions.",
    ),
    link_label: fr('Ouvrir dans Guide Négo'),
    link_url: '/guide-nego',
    visual: photo(10, 'Couverture du Guide des négociations de la CdP31'),
  },
  documents: [
    {
      asset_id: TRACK_PAGE_ASSET(11),
      url: 'https://media.epavillon.example/2026/11/journees/resume-guide-cdp31.pdf',
      mime_type: 'application/pdf',
      bytes: 1_840_000,
      filename: 'resume-guide-negociations-cdp31.pdf',
      caption: fr('Résumé (PDF)'),
    },
  ],
  parties: [
    { cle: 'matin', label: 'Matin', title: null, summary: 'Où en sont les négociations, thème par thème' },
    { cle: 'apres-midi', label: 'Après-midi', title: null, summary: 'Le Guide des négociations et le Programme de la Francophonie' },
  ],
  // Comme en production : l'atelier n'a que des moments, aucune activité à fiche.
  activites: [],
  moments: [
    { n: 3, jour: 8, debut: '09:00', fin: '09:15', libelle: "Ouverture de l'atelier", partie: 'matin', points: ["Mot de l'OIF", 'Mot du Secrétariat de la CCNUCC'] },
    { n: 4, jour: 8, debut: '09:45', fin: '12:00', libelle: 'Présentations des avancées par thématique', partie: 'matin', points: ['Atténuation', 'Adaptation', 'Pertes et préjudices', 'Mécanismes de coopération', 'Finance climat'] },
    { n: 5, jour: 8, debut: '12:00', fin: '12:30', libelle: 'Questions transversales', partie: 'matin', points: ['Cadre de la transparence', 'Transfert de technologies', 'Agriculture', 'Genre et climat'] },
    { n: 1, jour: 8, debut: '12:30', fin: '13:30', libelle: 'Déjeuner', partie: 'matin' },
    { n: 6, jour: 8, debut: '14:15', fin: '14:50', libelle: 'Présentation du Guide des négociations et du Programme de la Francophonie à la CdP31', partie: 'apres-midi', points: ['Présentation du Guide des négociations : Dr Issa Bado et M. Serges Djohy', 'Le Programme de la Francophonie à la CdP31 : Dr Issa Bado'] },
    { n: 2, jour: 8, debut: '14:50', fin: '15:00', libelle: 'Clôture', partie: 'apres-midi' },
  ],
}

const finance: Source = {
  id: TRACK_PAGE.finance,
  slug: 'journee-finance-durable',
  code: 'journee_finance_durable',
  kind: 'special_day',
  title: fr('Journée finance durable en Francophonie'),
  subtitle: fr("Opportunités et partenariats pour l'accès à la finance durable en Francophonie"),
  description: fr(
    "L'OIF, ses partenaires financiers et des porteurs de projets francophones se retrouvent au pavillon pour rapprocher les guichets de la finance climat de ceux qui en ont besoin.\n\nLancement de l'appel à manifestation d'intérêt des États membres pour l'accompagnement à l'accès à la finance durable, présentation de la plateforme francophone de la finance durable et de l'étude OIF-OCDE, puis des cas concrets : Sahel, collectivités, budgétisation de la CDN, littoral insulaire, villes africaines.",
  ),
  tagline: null,
  jour: 13,
  color: '#00A1E4',
  venue: pavillon,
  cover: photo(20, 'Le pavillon de la Francophonie pendant une table ronde'),
  publics: [],
  support: null,
  partenaires: [
    partenaire(O.oif, 'organizer'),
    partenaire(O.giz, 'partner'),
    partenaire(O.fondsAdaptation, 'partner'),
    partenaire(O.fondsVert, 'partner'),
  ],
  feature: null,
  documents: [
    {
      asset_id: TRACK_PAGE_ASSET(21),
      url: 'https://media.epavillon.example/2026/11/journees/programme-journee-finance.pdf',
      mime_type: 'application/pdf',
      bytes: 620_000,
      filename: 'programme-journee-finance-durable.pdf',
      caption: fr('Programme de la journée (PDF)'),
    },
  ],
  parties: [
    { cle: 'p1', label: null, title: "Opportunités de financement durable et synergies d'action en Francophonie", summary: "Opportunités de financement et synergies d'action" },
    { cle: 'p2', label: null, title: 'Initiatives pertinentes nationales, études de cas et témoignages', summary: 'Initiatives nationales, études de cas et témoignages' },
    { cle: 'p3', label: null, title: 'Partage entre les pairs et partenaires, réseautage', summary: 'Partage entre pairs et partenaires, réseautage' },
  ],
  activites: [
    {
      n: 12, jour: 13, debut: '09:15', fin: '10:00', titre: 'Panel de partenaires et d\'experts : solutions et synergies par les partenaires en Francophonie',
      points: ["Présentation de l'étude OIF-OCDE sur l'accès à la finance climat et biodiversité", 'Discussions'],
      orgs: [O.oif, O.ocde, O.fondsAdaptation, O.fondsVert, O.cif], partie: 'p1',
    },
    { n: 13, jour: 13, debut: '10:15', fin: '11:15', titre: "Du financement climatique à l'action régionale : les solutions du P2RS pour la résilience du Sahel, la restauration des terres et la sécurité alimentaire", orgs: [O.cilss], partie: 'p2' },
    { n: 14, jour: 13, debut: '11:30', fin: '12:30', titre: 'Enjeux de financements pour les régions et les municipalités', orgs: [O.airf, O.sakete], partie: 'p2' },
    { n: 15, jour: 13, debut: '13:00', fin: '14:00', titre: 'Autorité nationale de budgétisation climat : institutionnaliser, coordonner et budgétiser la mise en œuvre de la CDN', orgs: [O.menteSenegal], partie: 'p2' },
    { n: 16, jour: 13, debut: '14:15', fin: '15:15', titre: 'Du récif à la résilience : financer et mettre en œuvre la réhabilitation côtière dans les petits États insulaires', orgs: [O.businessMaurice], partie: 'p2' },
    { n: 17, jour: 13, debut: '15:30', fin: '16:30', titre: 'Rencontre avec les experts : mobiliser les investissements pour une transition résiliente et inclusive', orgs: [O.ndcp], partie: 'p3' },
    { n: 18, jour: 13, debut: '16:45', fin: '17:45', titre: 'Construire des villes africaines résilientes : financer les infrastructures vertes et le développement urbain circulaire', orgs: [O.gggi], partie: 'p3' },
  ],
  moments: [
    { n: 11, jour: 13, debut: '08:00', fin: '08:45', libelle: 'Accueil et installation', partie: 'p1' },
    {
      n: 13, jour: 13, debut: '09:00', fin: '09:15', libelle: 'Ouverture de la journée', partie: 'p1',
      points: ["Mot d'ouverture, OIF", "Lancement officiel de l'appel à manifestation d'intérêt des États membres de l'OIF pour l'accompagnement à l'accès à la finance durable", 'Présentation de la plateforme francophone de la finance durable'],
    },
    { n: 12, jour: 13, debut: '18:00', fin: null, libelle: 'Clôture', partie: 'p3' },
  ],
}

const jeunesse: Source = {
  id: TRACK_PAGE.jeunesse,
  slug: 'journee-jeunesse-climat',
  code: 'journee_jeunesse_climat',
  kind: 'special_day',
  title: fr('Journée action jeunesse pour le climat en Francophonie'),
  subtitle: fr("Innovations et engagement des jeunes pour accélérer l'action climatique en Francophonie"),
  description: fr(
    "Une matinée portée par les jeunes francophones : pitchs d'organisations, hackathon sur l'intelligence artificielle appliquée au climat, jeu concours et scène artistique, puis un temps sur les compétences vertes et le leadership.",
  ),
  tagline: fr('Hackathon, pitchs, jeu, scène'),
  jour: 14,
  color: '#8FBF2F',
  venue: pavillon,
  cover: photo(30, 'Des jeunes délégués au pavillon de la Francophonie'),
  publics: [],
  support: 'Avec le soutien financier du gouvernement du Québec, dans le cadre du Plan pour une économie verte 2030.',
  partenaires: [
    partenaire(O.oif, 'organizer'),
    partenaire(O.quebec, 'financial_support'),
    partenaire(O.qjm, 'partner'),
  ],
  feature: null,
  documents: [],
  parties: [
    { cle: 'inspirer', label: "S'inspirer", title: null, summary: 'Ouverture et dix minutes qui inspirent' },
    { cle: 'creer', label: 'Créer et jouer', title: null, summary: 'Hackathon, Info ou Intox, ClimArtistes' },
    { cle: 'agir', label: "S'engager", title: null, summary: 'Compétences vertes et leadership' },
  ],
  activites: [
    {
      n: 22, jour: 14, debut: '09:20', fin: '10:20', titre: '« Les 10 minutes qui inspirent » : pitchs des organisations',
      orgs: [O.greenOpenLab, O.jveRca, O.youthCenter, O.airEtMoi, O.youthVoices, O.dakiFarm], partie: 'inspirer',
    },
    { n: 23, jour: 14, debut: '10:20', fin: '11:20', titre: "Hackathon francophone sur l'IA appliquée au climat, à la biodiversité et à la finance durable", orgs: [O.greenOpenLab], partie: 'creer', tempsFort: true },
    { n: 26, jour: 14, debut: '13:00', fin: '14:00', titre: 'Des compétences vertes au leadership des jeunes et des femmes pour une transition climatique juste et inclusive', orgs: [O.cegepGaspesie, O.genreClimatSn], partie: 'agir' },
  ],
  moments: [
    { n: 21, jour: 14, debut: '08:00', fin: '09:00', libelle: 'Arrivée des jeunes et des participant-e-s, animation', partie: 'inspirer' },
    { n: 22, jour: 14, debut: '09:00', fin: '09:20', libelle: 'Ouverture officielle', partie: 'inspirer', points: ['OIF', 'Gouvernement du Québec'] },
    { n: 23, jour: 14, debut: '11:20', fin: '12:00', libelle: 'Jeu concours « Info ou Intox Climat » : qui sera le Champion ou la Championne CdP 31 / OIF ?', partie: 'creer' },
    { n: 24, jour: 14, debut: '12:00', fin: '13:00', libelle: 'Les ClimArtistes en scène', partie: 'creer', points: ['Expression artistique'] },
  ],
}

const SOURCES = [atelier, finance, jeunesse]

const fin = (e: PublicTrackEntry): IsoDateTime => e.ends_at ?? e.starts_at
const min = (v: IsoDateTime[]): IsoDateTime | null => (v.length ? [...v].sort()[0]! : null)
const max = (v: IsoDateTime[]): IsoDateTime | null => (v.length ? [...v].sort().at(-1)! : null)

const deroule = (s: Source): PublicTrackEntry[] => {
  const piste = { slug: s.slug, title: s.title, color: s.color, kind: s.kind }
  const entrees: PublicTrackEntry[] = [
    ...s.moments.map((m): PublicTrackEntry => ({
      kind: 'moment',
      starts_at: a(m.jour, m.debut),
      ends_at: m.fin ? a(m.jour, m.fin) : null,
      part_id: m.partie ? partId(s, m.partie) : null,
      moment: { id: TRACK_PAGE_MOMENT(m.n), label: fr(m.libelle), description: m.points ? fr(m.points.join('\n')) : null },
    })),
    ...s.activites.map((x): PublicTrackEntry => ({
      kind: 'activity',
      starts_at: a(x.jour, x.debut),
      ends_at: a(x.jour, x.fin),
      part_id: x.partie ? partId(s, x.partie) : null,
      activity: {
        session: ligne(x, piste),
        is_highlight: x.tempsFort ?? false,
        speakers: intervenants(x),
        organizations: organisations(x),
      },
    })),
  ]
  return entrees.sort((g, d) =>
    g.starts_at === d.starts_at ? (g.kind === 'moment' ? -1 : 1) : g.starts_at < d.starts_at ? -1 : 1,
  )
}

function partId(s: Source, cle: string): string {
  return TRACK_PAGE_PART(SOURCES.indexOf(s) * 10 + s.parties.findIndex((p) => p.cle === cle) + 1)
}

const resume = (s: Source, courant: Source | null): PublicTrackSummary => {
  const e = deroule(s)
  return {
    slug: s.slug,
    kind: s.kind,
    title: s.title,
    color_hex: s.color,
    starts_on: `2026-11-${s.jour}`.replace(/-(\d)$/, '-0$1'),
    ends_on: `2026-11-${s.jour}`.replace(/-(\d)$/, '-0$1'),
    starts_at: min(e.map((x) => x.starts_at)),
    ends_at: max(e.map(fin)),
    venue_name: s.venue.name,
    is_off_site: s.venue.kind !== 'pavilion',
    is_current: s === courant,
  }
}

const page = (s: Source): PublicTrackPage => {
  const agenda = deroule(s)
  const activites = agenda.filter((e) => e.kind === 'activity').map((e) => e.starts_at)
  const debut = min(activites) ?? min(agenda.map((e) => e.starts_at))
  const premierMoment = min(agenda.filter((e) => e.kind === 'moment').map((e) => e.starts_at))
  const parts: PublicTrackPart[] = s.parties.map((p, i) => {
    const contenu = agenda.filter((e) => e.part_id === partId(s, p.cle))
    return {
      id: partId(s, p.cle),
      number: i + 1,
      label: p.label ? fr(p.label) : null,
      title: p.title ? fr(p.title) : null,
      summary: p.summary ? fr(p.summary) : null,
      starts_at: min(contenu.map((e) => e.starts_at)),
      ends_at: max(contenu.map(fin)),
    }
  })
  return {
    edition,
    track: {
      id: s.id,
      slug: s.slug,
      code: s.code,
      kind: s.kind,
      title: s.title,
      subtitle: s.subtitle,
      description: s.description,
      tagline: s.tagline,
      starts_on: resume(s, s).starts_on,
      ends_on: resume(s, s).ends_on,
      color_hex: s.color,
      themes: [],
    },
    venue: s.venue,
    is_off_site: s.venue.kind !== 'pavilion',
    cover: s.cover,
    documents: s.documents,
    feature: s.feature,
    target_audiences: s.publics.map(fr),
    support_note: s.support ? fr(s.support) : null,
    partners: s.partenaires,
    parts,
    agenda,
    starts_at: debut,
    ends_at: max(agenda.map(fin)),
    opens_at: premierMoment && debut && premierMoment < debut ? premierMoment : null,
    other_tracks: SOURCES.map((x) => resume(x, s)),
  }
}

/** `PublicTrackPage` par slug du fil. */
export const trackPages: Record<string, PublicTrackPage> = Object.fromEntries(
  SOURCES.map((s) => [s.slug, page(s)]),
)

/** `TrackPageSettings` par slug du fil : ce que le back-office lit. */
export const trackPageSettings: Record<string, TrackPageSettings> = Object.fromEntries(
  SOURCES.map((s) => {
    const p = trackPages[s.slug]!
    return [
      s.slug,
      {
        track_id: s.id,
        event_id: TRACK_PAGE.edition,
        venue_id: s.venue.kind === 'pavilion' ? null : s.venue.id,
        tagline: s.tagline,
        target_audiences: p.target_audiences,
        support_note: p.support_note,
        feature_kicker: s.feature?.kicker ?? null,
        feature_title: s.feature?.title ?? null,
        feature_body: s.feature?.body ?? null,
        feature_link_label: s.feature?.link_label ?? null,
        feature_link_url: s.feature?.link_url ?? null,
        parts: p.parts.map((x) => ({
          id: x.id,
          track_id: s.id,
          label: x.label,
          title: x.title,
          summary: x.summary,
          sort_order: x.number,
        })),
        moments: p.agenda.flatMap((e) =>
          e.kind === 'moment'
            ? [{ id: e.moment.id, track_id: s.id, part_id: e.part_id, starts_at: e.starts_at, ends_at: e.ends_at, label: e.moment.label, description: e.moment.description }]
            : [],
        ),
        partners: s.partenaires.map((x, i) => ({ ...x, sort_order: i + 1 })),
        sessions: p.agenda.flatMap((e, i) =>
          e.kind === 'activity'
            ? [{
                session_id: e.activity.session.id,
                slug: e.activity.session.slug,
                title: e.activity.session.title,
                starts_at: e.activity.session.starts_at,
                ends_at: e.activity.session.ends_at,
                status: e.activity.session.status,
                is_published: true,
                part_id: e.part_id,
                is_highlight: e.activity.is_highlight,
                sort_order: i,
              }]
            : [],
        ),
      } satisfies TrackPageSettings,
    ]
  }),
)

/** Les vingt-quatre organisations du jeu, pour un sélecteur de partenaire. */
export const trackPageOrganizations = Object.values(O)

// --- La programmation de l'édition ------------------------------------------

/** Activités d'aucun fil : elles restent des lignes ordinaires, sous le cadre d'une journée. */
const HORS_FIL: Activite[] = [
  { n: 31, jour: 10, debut: '10:00', fin: '11:00', titre: "Lancement du rapport francophone sur l'adaptation des villes côtières", orgs: [O.airf], partie: null },
  { n: 32, jour: 11, debut: '14:00', fin: '15:00', titre: 'Guide Négo : prise en main pour les délégations francophones', orgs: [O.oif], partie: null },
  { n: 33, jour: 13, debut: '18:30', fin: '19:30', titre: 'Dialogue des bailleurs et des collectivités sur la finance locale', orgs: [O.sakete], partie: null },
  { n: 34, jour: 14, debut: '15:00', fin: '16:00', titre: "Femmes et transition énergétique en Afrique de l'Ouest", orgs: [O.genreClimatSn], partie: null },
]

/** `GET /schedule?event_id=` de la CdP31 d'Antalya : les activités des trois fils et celles d'aucun fil. */
export const trackPagesSchedule: PublicScheduleRow[] = [
  ...Object.values(trackPages).flatMap((p) => p.agenda.flatMap((e) => (e.kind === 'activity' ? [e.activity.session] : []))),
  ...HORS_FIL.map((x) => ({ ...ligne(x, { slug: '', title: fr(''), color: null, kind: 'special_day' }), tracks: [] })),
].sort((g, d) => g.starts_at.localeCompare(d.starts_at))

/** `GET /schedule/tracks?event_id=` de la CdP31 d'Antalya. */
export const trackPagesScheduleTracks: PublicScheduleTrack[] = SOURCES.map((s) => ({
  ...resume(s, null),
  moments: s.moments
    .map((m) => ({ starts_at: a(m.jour, m.debut), ends_at: m.fin ? a(m.jour, m.fin) : null, label: fr(m.libelle) }))
    .sort((g, d) => g.starts_at.localeCompare(d.starts_at)),
}))
