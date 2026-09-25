/**
 * Le savoir de Guide Négo, sans API — les données de
 * `specs/013-guide-nego-faq-lexique/donnees-essai.sql`, avec deux écarts voulus :
 * les **dix-neuf** termes sont publiés (la recherche se mesure sur eux, SC-003),
 * et seule l'entrée de FAQ complète paraît, comme en base où les autres restent en
 * brouillon. Ses questions liées, brouillons, restent citées : le téléphone les
 * filtre.
 *
 * Les textes n'existent qu'en français, comme dans le script : en anglais, seuls
 * les libellés des vocabulaires changent, le reste retombe sur le français.
 */
import type { I18nText, IsoDateTime, Uuid } from '~/types/shared'
import type {
  FaqEntry,
  FaqSection,
  GlossaryEntry,
  GlossaryFamily,
  KnowledgeBundle,
  KnowledgeSource,
  MyGlossaryFavorites,
  PathwayGroup,
  PathwayLink,
  PathwayStep,
} from '~/types/negotiation-savoir'
import { ApiRequestError } from '~/utils/api-error'
import { resolveI18nText } from '~/utils/i18n-text'
import { slugDe } from '~/utils/guide-nego/lexique'
import { DOCUMENT_NEGO, empreinte } from './negotiation-documents'

interface Etiquete<T> {
  valeur: T
  empreinte: string | null
}

const id = (lettre: string, n: number): Uuid => `00000000-0013-7000-8000-${lettre}${String(n).padStart(11, '0')}`

export const SAVOIR = {
  faqComplete: id('f', 7),
  contactGroup: id('a', 1),
  bracketedText: id('a', 7),
  gga: id('a', 16),
} as const

const MIS_A_JOUR: IsoDateTime = '2026-09-24T08:00:00Z'
const VERIFIEE_LE = '2026-09-24'
const CHEVAUCHEMENT_MS = 5 * 60 * 1000

export const RUBRIQUES: { code: string; label: I18nText; icon: string; sort_order: number }[] = [
  { code: 'first_cop', label: { fr: 'Ma première COP', en: 'My first COP' }, icon: 'star', sort_order: 10 },
  { code: 'process', label: { fr: 'Le processus', en: 'The process' }, icon: 'toc', sort_order: 20 },
  { code: 'negotiating_groups', label: { fr: 'Les groupes de négociation', en: 'Negotiating groups' }, icon: 'user', sort_order: 30 },
  { code: 'on_site', label: { fr: 'Sur place', en: 'On site' }, icon: 'pin', sort_order: 40 },
]

export const FAMILLES: { code: string; label: I18nText; sort_order: number }[] = [
  { code: 'meetings', label: { fr: 'Réunions', en: 'Meetings' }, sort_order: 10 },
  { code: 'texts', label: { fr: 'Textes', en: 'Texts' }, sort_order: 20 },
  { code: 'themes', label: { fr: 'Thématiques', en: 'Themes' }, sort_order: 30 },
]

const GUIDE = { document_id: DOCUMENT_NEGO.guideCop31, document_title: 'Guide des négociations — CdP31' }
const IISD = '« Au nom de ma délégation… », IISD'
const IIED = 'Guide des négociations, IIED'

const pages = (de: number, a = de) => ({ page_from: de, page_to: a })
const exterieure = (titre: string, de: number, a = de): KnowledgeSource => ({ external_title: titre, ...pages(de, a) })
const duGuide = (de: number, a = de): KnowledgeSource => ({ ...GUIDE, ...pages(de, a) })

export const FAQ: FaqEntry[] = [
  {
    id: SAVOIR.faqComplete,
    section_code: 'process',
    question: 'Quelle différence entre un groupe de contact et des consultations informelles ?',
    answer:
      "Les deux sont des réunions de négociation ouvertes à toutes les Parties, et de plus en plus aux observateurs. La différence tient à l'organisation, pas à l'importance : au plus deux groupes de contact peuvent siéger en même temps, contre six consultations informelles, si bien que la plupart des points de l'ordre du jour passent en consultations informelles. Les deux peuvent aboutir à un texte convenu — un document L — soumis à l'adoption. Dans les deux cas, on parle anglais, sans interprétation. Les « informal informals » sont plus petites, se tiennent sans le Secrétariat, et rendent compte au groupe.",
    status: 'published',
    verified_on: VERIFIEE_LE,
    sources: [
      {
        ...duGuide(74),
        section_label: 'annexe A.3',
        quote:
          "Il existe une perception selon laquelle un groupe de contact aurait plus d'importance qu'une consultation informelle. Cependant, il est important de souligner que le travail mené dans le cadre d'une consultation informelle peut, au même titre qu'un groupe de contact, aboutir à un texte convenu (document L).",
      },
      { external_title: IISD, section_label: 'Le contexte de négociation', ...pages(38, 39) },
    ],
    // Deux brouillons : la liste arrive entière, le téléphone la filtre.
    related_ids: [id('f', 8), id('f', 10)],
    updated_at: MIS_A_JOUR,
  },
]

type Terme = [
  n: number,
  famille: string,
  terme: string,
  sigle: string | null,
  variantes: string[],
  traduction: string,
  definition: string,
  enSalle: string,
  sources: KnowledgeSource[],
  lies: number[],
]

const TERMES: Terme[] = [
  [1, 'meetings', 'contact group', null, ['contact groups'], 'groupe de contact',
    "Réunion établie pour négocier un point de l'ordre du jour. Ouverte à toutes les Parties et, sauf objection, aux observateurs. Peut aboutir à un texte convenu. Au plus deux groupes de contact siègent en même temps.",
    'The contact group will reconvene at 3 p.m.', [exterieure(IISD, 38), duGuide(74)], [2, 3, 5]],
  [2, 'meetings', 'informal consultations', null, ['informal consultation'], 'consultations informelles',
    "Réunion de négociation créée par les Parties, très proche d'un groupe de contact dans son fonctionnement, et de plus en plus ouverte aux observateurs. La plupart des points de l'ordre du jour passent par là. En anglais, sans interprétation.",
    'Informal consultations on the GGA are moved to room 9.', [exterieure(IISD, 38), duGuide(74)], [1]],
  [3, 'meetings', 'informal informals', null, [], '— (se dit tel quel)',
    "Quand un point bloque, les co-facilitateurs proposent aux Parties concernées de se retrouver sans facilitation, sans le Secrétariat, pour chercher un terrain d'entente. On oublie les drapeaux ; les délégués s'appellent par leur nom. Tout ce qui s'y dit est rapporté au groupe de contact.",
    "We'll continue in informal informals after the break.", [exterieure(IIED, 35), exterieure(IISD, 38)], [1]],
  [4, 'meetings', 'huddle', null, ['huddles'], 'aparté',
    "Attroupement improvisé de quelques délégués, dans un coin de salle ou un couloir, pour débloquer une phrase. Sans statut, jamais programmé : on n'en apprend l'existence que par le réseau.",
    "Let's huddle on paragraph 12.", [], []],
  [5, 'meetings', 'co-facilitators', null, ['co-facilitator', 'cofacilitators'], 'co-facilitateurs',
    "Paire de délégués nommée par la présidence pour conduire les consultations informelles sur un point : l'un d'un pays développé, l'autre d'un pays en développement.",
    'The co-facilitators will prepare a new iteration tonight.', [exterieure(IISD, 38)], [2]],
  [6, 'meetings', 'informal stocktaking plenary', null, [], 'plénière informelle de bilan',
    "En deuxième semaine, la présidence réunit toutes les Parties pour entendre où en est chaque négociation et ce qu'elle prévoit ensuite.",
    'The Presidency convenes an informal stocktaking plenary at 6 p.m.', [exterieure(IISD, 39)], []],
  [7, 'texts', 'bracketed text', null, ['bracketed', 'brackets'], 'texte entre crochets',
    "Un passage entre crochets n'a pas encore fait l'objet d'un accord : il est en cours de négociation. Les crochets se lèvent un à un ; ne jamais en céder un sans obtenir autre chose.",
    'We still have three brackets in paragraph 12.', [exterieure(IIED, 41), exterieure(IISD, 107)], [8, 11]],
  [8, 'texts', 'agreed language', null, [], 'formulation convenue',
    "Texte repris mot pour mot de la Convention, du Protocole, de l'Accord de Paris ou d'une décision antérieure, pour rédiger, garder un sujet vivant ou sortir d'une impasse.",
    'This is agreed language from decision 1/CP.21.', [exterieure(IIED, 41)], []],
  [9, 'texts', 'ad ref.', null, ['ad referendum'], 'ad referendum',
    "Un alinéa cité « ad ref. » est finalisé alors que le reste du texte se négocie encore : il n'est plus ouvert.",
    'Paragraph 5 is agreed ad ref.', [exterieure(IIED, 41)], []],
  [10, 'texts', 'PP / OP', null, ['PP', 'OP', 'preambular paragraph', 'operative paragraph'], 'alinéa du préambule / du dispositif',
    "Les alinéas du préambule se citent « PP » et ceux du dispositif « OP », suivis d'un numéro.",
    'I have a concern with OP3.', [exterieure(IIED, 41)], [11]],
  [11, 'texts', 'bis, ter, alt', null, ['bis', 'ter', 'alt'], 'bis, ter, alt',
    "Pour insérer un alinéa après l'OP3, on propose un « OP3bis » ; après lui, un « OP3ter ». « Alt » propose un texte de remplacement : un « PP5alt » supprime le PP5 et le remplace.",
    'My delegation proposes an OP3bis.', [exterieure(IIED, 41, 42)], [10]],
  [12, 'texts', 'L document', null, ['L documents', 'L doc'], 'document L',
    "Projet de conclusions ou de décision, à diffusion limitée, soumis à l'adoption par l'organe compétent ; traduit dans les six langues, parfois après l'adoption. Sa cote commence par « L. ».",
    'The L document will be issued tonight.', [exterieure(IISD, 52)], [15]],
  [13, 'texts', 'non-paper', null, ['nonpaper', 'non-papers'], 'non-papier, note informelle',
    "Texte sans statut officiel, diffusé par une présidence ou des co-facilitateurs pour faire avancer la discussion. Il n'engage personne.",
    'The co-facilitators circulated a non-paper this morning.', [], []],
  [14, 'texts', 'landing zone', null, [], "zone d'atterrissage",
    "L'espace de compromis où un accord devient possible entre les positions en présence.",
    'I think we are close to a landing zone.', [], []],
  [15, 'texts', 'conclusions', null, [], 'conclusions',
    "Résultat des négociations de l'OSMOE et de l'OSCST sur un point de leur ordre du jour. Sans force juridique, elles recommandent à la CdP et montrent l'orientation prise.",
    'The SBI adopted conclusions on this item.', [exterieure(IIED, 42)], [12]],
  [16, 'themes', 'global goal on adaptation', 'GGA', [], "objectif mondial en matière d'adaptation",
    "Objectif de l'Accord de Paris visant à élever l'adaptation au niveau de l'atténuation. À la CdP30, l'enjeu était l'adoption de cent indicateurs pour suivre les progrès collectifs.",
    'We need to finalise the GGA indicators.', [duGuide(59)], [17, 19]],
  [17, 'themes', 'nationally determined contribution', 'NDC', ['NDCs'], 'CDN — contribution déterminée au niveau national',
    "Engagement climatique que chaque pays fixe lui-même au titre de l'Accord de Paris, et révise tous les cinq ans.",
    'Our new NDC was submitted in September.', [], []],
  [18, 'themes', 'loss and damage', null, ['L&D'], 'pertes et préjudices',
    "Dommages du changement climatique que ni l'atténuation ni l'adaptation n'ont évités. Un fonds de réponse existe depuis la CdP28.",
    'The loss and damage fund needs replenishment.', [duGuide(62, 63)], [16]],
  [19, 'themes', 'global stocktake', 'GST', [], 'bilan mondial',
    "Examen collectif, tous les cinq ans, des progrès vers les objectifs de l'Accord de Paris. Le premier s'est conclu à la CdP28.",
    'The GST outcome must guide the next NDCs.', [duGuide(26)], [17]],
]

export const LEXIQUE: GlossaryEntry[] = TERMES.map(
  ([n, famille, term, acronym, variants, translation, definition, heard_in_room, sources, lies]) => ({
    id: id('a', n),
    slug: slugDe(term),
    family_code: famille,
    term,
    acronym,
    variants,
    translation,
    definition,
    heard_in_room,
    sources,
    related_ids: lies.map((l) => id('a', l)),
    status: 'published',
    updated_at: MIS_A_JOUR,
  }),
)

const versLeGuide = (section: string | null, label: string): PathwayLink => ({
  kind: 'document',
  target_id: DOCUMENT_NEGO.guideCop31,
  page: null,
  section,
  label,
})

type Etape = [n: number, label: string, detail?: string | null, origine?: string | null, lien?: PathwayLink | null]

const GROUPES: [n: number, label: string, etapes: Etape[]][] = [
  [1, 'Avant de partir', [
    [1, 'Vérifier mon accréditation et ma lettre de nomination.'],
    [2, 'Télécharger le Guide des négociations et le Résumé pour les décideurs.', null, null, versLeGuide(null, 'Documents')],
    [3, 'Choisir mes thématiques et lire leurs fiches.', null, null, versLeGuide('chapitre 3', 'Guide, chapitre 3')],
    [4, 'Trouver la coordonnatrice de mon groupe sur chaque thématique.', null, null, versLeGuide('annexe A.5', 'Guide, annexe A.5')],
    [5, 'Préparer la position de mon pays avec mon point focal.'],
  ]],
  [2, 'Le premier jour', [
    [6, 'Retirer mon badge et repérer les salles.'],
    [7, "Assister à l'atelier préparatoire des négociateurs francophones.", null, 'Réunions de la Francophonie'],
    [8, 'Assister à la coordination de mon groupe.', 'Groupe africain 8:00, PMA 13:00'],
    [9, 'Me présenter à la coordonnatrice de ma thématique.'],
    [10, 'Régler mes notifications pour mes thématiques.'],
  ]],
  [3, 'En salle', [
    [11, "Vérifier la salle et l'heure juste avant : les sessions se déplacent."],
    [12, 'Noter qui parle pour quel groupe.'],
    [13, 'Repérer les crochets et les options dans le texte.', null, null,
      { kind: 'glossary', target_id: SAVOIR.bracketedText, page: null, section: null, label: 'Lexique, « bracketed text »' }],
    [14, 'Signaler un changement constaté : annulation, déplacement, réunion non annoncée.'],
  ]],
  [4, 'Le soir', [
    [15, 'Relire mes notes et rédiger ma restitution.'],
    [16, 'Lire le Bulletin des négociations de la Terre du jour.'],
    [17, 'Vérifier les sessions de demain pour mes thématiques.'],
    [18, 'Poser à un expert la question restée sans réponse.'],
  ]],
]

export const PARCOURS: PathwayGroup[] = GROUPES.map(([n, label, etapes]) => ({
  id: id('b', n),
  label,
  sort_order: n * 10,
  steps: etapes.map(
    ([e, libelle, detail = null, origine = null, lien = null], i): PathwayStep => ({
      id: id('c', e),
      label: libelle,
      detail,
      origin_label: origine,
      link: lien,
      sort_order: (i + 1) * 10,
    }),
  ),
}))

/**
 * `GET /negotiation/knowledge` : le paquet entier, ou avec `since` les entrées
 * changées depuis `since − 5 min`. Rien n'est jamais retiré ici : `removed` reste vide.
 */
export function paquetDuSavoir(langue = 'fr', since: IsoDateTime | null = null): Etiquete<KnowledgeBundle> {
  const depuis = since ? Date.parse(since) - CHEVAUCHEMENT_MS : null
  const change = (e: { updated_at: IsoDateTime }) => depuis === null || Date.parse(e.updated_at) >= depuis
  const faq_sections: FaqSection[] = RUBRIQUES.map((r) => ({ ...r, label: resolveI18nText(r.label, langue) }))
  const glossary_families: GlossaryFamily[] = FAMILLES.map((f) => ({ ...f, label: resolveI18nText(f.label, langue) }))
  const valeur: KnowledgeBundle = {
    served_at: new Date().toISOString(),
    complete: since === null,
    faq_sections,
    glossary_families,
    faq: FAQ.filter(change),
    glossary: LEXIQUE.filter(change),
    pathway: { groups: PARCOURS },
    most_read: [SAVOIR.faqComplete],
    removed: { faq: [], glossary: [] },
  }
  return { valeur, empreinte: empreinte([langue, FAQ, LEXIQUE, PARCOURS, faq_sections, glossary_families]) }
}

// ---------------------------------------------------------------------------
// Les termes favoris — plus récent d'abord, comme l'API
// ---------------------------------------------------------------------------

let favoris: { entry_id: Uuid; created_at: IsoDateTime }[] = []

export function mesTermesFavoris(): Etiquete<MyGlossaryFavorites> {
  const entry_ids = [...favoris]
    .sort((a, b) => Date.parse(b.created_at) - Date.parse(a.created_at) || (a.entry_id < b.entry_id ? -1 : 1))
    .map((f) => f.entry_id)
  return { valeur: { entry_ids }, empreinte: empreinte(entry_ids) }
}

export function poserUnTermeFavori(id: Uuid): void {
  if (!LEXIQUE.some((e) => e.id === id)) {
    throw new ApiRequestError(
      { code: 'NEGOTIATION_GLOSSARY_NOT_FOUND', message: "Ce terme n'existe pas, ou n'est plus publié." },
      404,
    )
  }
  if (!favoris.some((f) => f.entry_id === id)) favoris = [...favoris, { entry_id: id, created_at: new Date().toISOString() }]
}

export function retirerUnTermeFavori(id: Uuid): void {
  favoris = favoris.filter((f) => f.entry_id !== id)
}
