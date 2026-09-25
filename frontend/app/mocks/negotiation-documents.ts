/**
 * Les documents de Guide Négo, sans API — les cinq de la maquette
 * (`docs/AppNego/design/ecrans/03-documents.html`), et le Guide des négociations
 * mis en forme lisible autour de sa page 59 (`negotiation-documents-guide.ts`).
 *
 * **Le jeu sert deux choses seulement** : les tests, et le travail hors ligne.
 * `tests/guide-nego/documents-exemples.test.ts` fixe ce qu'il fait comme l'API.
 *
 * UNE SEULE RÉSERVE DE FICHES, lue des deux côtés : la bibliothèque, la lecture
 * et la recherche se déduisent de ce que le back-office modifie. Publier ici
 * fait paraître le document là, comme en base.
 *
 * Les libellés du vocabulaire sont ceux du semis (`020_reference.sql`), les COP
 * celles de `mocks/event.ts`. Titres et libellés se résolvent dans la `langue`
 * demandée, repli sur le français, comme l'API selon `Accept-Language`. Les
 * messages de refus sont ceux du catalogue du noyau et des services Rust, pour
 * que l'écran hors ligne dise la même chose que l'écran branché.
 */

import type { I18nText, IsoDate, IsoDateTime, Uuid } from '~/types/shared'
import type {
  Block,
  CorrectionNoteList,
  DocumentBookmark,
  DocumentBookmarkList,
  DocumentLibrary,
  DocumentReading,
  DocumentTextHits,
  LibraryDocument,
  OutlineEntry,
  ReadingPage,
  Span,
  Successor,
  TextHit,
  VocabularyCop,
  VocabularyTerm,
} from '~/types/negotiation-documents'
import type {
  AdminCorrectionNote,
  AdminCorrectionNoteList,
  AdminDocument,
  AdminDocumentFile,
  AdminDocumentInput,
  AdminDocumentList,
  AdminDocumentPreview,
  AdminDocumentState,
  CorrectionNoteInput,
  DocumentLink,
  ExtractionQuality,
  ExtractionState,
  PersonLink,
} from '~/types/admin-negotiation-documents'
import type { ApiErrorCode } from '~/types/api-error'
import { ApiRequestError } from '~/utils/api-error'
import { resolveI18nText } from '~/utils/i18n-text'
import { events } from './event'
import { EVENT } from './ids'
import { monAcces } from './negotiation-access'
import { PAGES_DU_GUIDE, SOMMAIRE_DU_GUIDE, entree, para, s, titresDeLaPage } from './negotiation-documents-guide'

/** La forme d'`AvecEmpreinte`, redite ici : un jeu d'exemple n'importe pas un composable. */
interface Etiquete<T> {
  valeur: T
  empreinte: string | null
}

export const DOCUMENT_NEGO = {
  guideCop31: '0199b010-0000-7000-8000-000000000001',
  resumeCop31: '0199b010-0000-7000-8000-000000000002',
  bulletin11Novembre: '0199b010-0000-7000-8000-000000000003',
  auNomDeMaDelegation: '0199b010-0000-7000-8000-000000000004',
  noteBilanCop30: '0199b010-0000-7000-8000-000000000005',
} as const

const EXPERT: PersonLink = { id: '0199b013-0000-7000-8000-000000000001', name: 'Dr Koffi Mensah' }

type Libelles = Record<string, { label: I18nText; sort_order: number }>

const TYPES: Libelles = {
  negotiation_guide: { label: { fr: 'Guide', en: 'Guide' }, sort_order: 10 },
  summary: { label: { fr: 'Résumé', en: 'Summary' }, sort_order: 15 },
  technical_note: { label: { fr: 'Note technique', en: 'Technical note' }, sort_order: 20 },
  bulletin: { label: { fr: 'Bulletin', en: 'Bulletin' }, sort_order: 25 },
  relevant_document: { label: { fr: 'Document de référence', en: 'Reference document' }, sort_order: 30 },
  presentation: { label: { fr: 'Présentation', en: 'Presentation' }, sort_order: 40 },
  report: { label: { fr: 'Rapport', en: 'Report' }, sort_order: 50 },
  other: { label: { fr: 'Autre', en: 'Other' }, sort_order: 90 },
}

const THEMES: Libelles = {
  adaptation: { label: { fr: 'Adaptation', en: 'Adaptation' }, sort_order: 10 },
  mitigation: { label: { fr: 'Atténuation', en: 'Mitigation' }, sort_order: 20 },
  finance: { label: { fr: 'Finance', en: 'Finance' }, sort_order: 30 },
  loss_and_damage: { label: { fr: 'Pertes et préjudices', en: 'Loss and damage' }, sort_order: 40 },
  article_6: { label: { fr: 'Article 6', en: 'Article 6' }, sort_order: 50 },
  transparency: { label: { fr: 'Transparence', en: 'Transparency' }, sort_order: 60 },
  gender: { label: { fr: 'Genre', en: 'Gender' }, sort_order: 70 },
  just_transition: { label: { fr: 'Transition juste', en: 'Just transition' }, sort_order: 80 },
  agriculture: { label: { fr: 'Agriculture', en: 'Agriculture' }, sort_order: 90 },
  technology: { label: { fr: 'Technologie', en: 'Technology' }, sort_order: 100 },
}

const ilYA = (jours: number): IsoDateTime => new Date(Date.now() - jours * 86_400_000).toISOString()
const jourDe = (instant: IsoDateTime): IsoDate => instant.slice(0, 10)
const maintenant = (): IsoDateTime => new Date().toISOString()

/** L'ordre de `ORDER BY` sur un code ou un identifiant. */
const ordre = (a: string, b: string): number => (a < b ? -1 : a > b ? 1 : 0)
const recent = (a: IsoDateTime, b: IsoDateTime): number => Date.parse(b) - Date.parse(a)

function refus(code: ApiErrorCode, status: number, message: string, field?: string): ApiRequestError {
  return new ApiRequestError({ code, message, field }, status)
}
const introuvable = () => refus('NEGOTIATION_DOCUMENT_NOT_FOUND', 404, "Ce document n'existe pas, ou n'est plus publié.")
const figee = (field?: string) =>
  refus('NEGOTIATION_DOCUMENT_FILE_LOCKED', 409, "Le fichier d'un document publié ne change pas. Publiez une nouvelle version.", field)
const invalide = (message: string, field: string) => refus('VALIDATION_FAILED', 422, message, field)
// Traduit de `ck_documents_source_at_most_one` : le champ est celui du lien, quel que soit le geste.
const lesDeux = () =>
  refus('NEGOTIATION_DOCUMENT_SOURCE_BOTH', 422, 'Un document est un fichier ou un lien, jamais les deux.', 'external_url')

/** Une empreinte stable : le même état donne la même chaîne. */
function empreinte(valeur: unknown): string {
  let h = 5381
  for (const c of JSON.stringify(valeur)) h = ((h << 5) + h + c.charCodeAt(0)) | 0
  return `"${(h >>> 0).toString(16)}"`
}

const aLAcces = (): boolean => monAcces().state === 'granted'
const sansAccents = (texte: string): string => texte.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase()

// ---------------------------------------------------------------------------
// La réserve
// ---------------------------------------------------------------------------

interface Fiche {
  id: Uuid
  slug: string
  title: I18nText
  summary: I18nText | null
  type: string
  themes: string[]
  cop: Uuid | null
  version: string
  issued_on: IsoDate | null
  publisher: string | null
  locale: string
  asset_id: Uuid | null
  file: AdminDocumentFile | null
  external_url: string | null
  supersedes_id: Uuid | null
  restricted: boolean
  rag_eligible: boolean
  published_at: IsoDateTime | null
  unpublished_at: IsoDateTime | null
  extraction: ExtractionState | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

type Note = AdminCorrectionNote

/** La règle de `negotiation.document_reading_modes()` : ici, toute extraction prête a du texte. */
const avecLesModes = (e: Omit<ExtractionState, 'has_text' | 'large_text'>): ExtractionState => {
  const has_text = e.status === 'ready'
  return { ...e, has_text, large_text: has_text && (e.large_text_choice ?? e.is_reflowable ?? false) }
}

const extraite = (pages: number, octets: number, le: IsoDateTime): ExtractionState =>
  avecLesModes({
    status: 'ready',
    page_count: pages,
    is_reflowable: true,
    large_text_choice: null,
    failure_reason: null,
    reading_bytes: octets,
    extracted_at: le,
  })

const enFile = (): ExtractionState =>
  avecLesModes({
    status: 'pending',
    page_count: null,
    is_reflowable: null,
    large_text_choice: null,
    failure_reason: null,
    reading_bytes: null,
    extracted_at: null,
  })

/** Un PDF déposé et extrait : `n` numérote l'objet média, `pages` et `octets` la forme lisible. */
const pdf = (n: number, nom: string, pages: number, octets: number, le: IsoDateTime) => ({
  asset_id: `0199b012-0000-7000-8000-${String(n).padStart(12, '0')}`,
  file: { filename: nom, byte_size: Math.round(octets * 1.05), mime_type: 'application/pdf' },
  extraction: extraite(pages, octets, le),
})

type Graine = Pick<Fiche, 'id' | 'slug' | 'title' | 'type'> & Partial<Fiche>

function graine(g: Graine): Fiche {
  const le = g.published_at ?? maintenant()
  return {
    summary: null,
    themes: [],
    cop: null,
    version: '1',
    issued_on: null,
    publisher: null,
    locale: 'fr',
    asset_id: null,
    file: null,
    external_url: null,
    supersedes_id: null,
    restricted: false,
    rag_eligible: false,
    published_at: null,
    unpublished_at: null,
    extraction: null,
    created_at: le,
    updated_at: le,
    ...g,
  }
}

function semis(): Fiche[] {
  const guide = ilYA(10)
  return [
    graine({
      ...pdf(1, 'guide-des-negociations-cdp31.pdf', 92, 7_000_000, guide),
      id: DOCUMENT_NEGO.guideCop31,
      slug: 'guide-des-negociations-cdp31',
      title: { fr: 'Guide des négociations — CdP31', en: 'Negotiations guide — COP31' },
      summary: {
        fr: "Le décryptage des résultats de la CdP30, l'état des négociations aux intersessions de Bonn, et les enjeux de chaque thématique en route vers la CdP31.",
      },
      type: 'negotiation_guide',
      themes: ['adaptation', 'mitigation', 'finance', 'loss_and_damage', 'article_6', 'gender', 'just_transition'],
      cop: EVENT.cop31,
      version: '1.0',
      issued_on: jourDe(guide),
      publisher: 'IFDD',
      supersedes_id: DOCUMENT_NEGO.noteBilanCop30,
      rag_eligible: true,
      published_at: guide,
    }),
    graine({
      ...pdf(2, 'resume-decideurs-cdp31.pdf', 12, 1_800_000, ilYA(2)),
      id: DOCUMENT_NEGO.resumeCop31,
      slug: 'resume-pour-les-decideurs-cdp31',
      title: { fr: 'Résumé pour les décideurs — CdP31' },
      summary: {
        fr: 'Les positions à défendre et les lignes rouges, thématique par thématique, pour les cheffes et chefs de délégation.',
      },
      type: 'summary',
      themes: ['adaptation', 'finance', 'loss_and_damage'],
      cop: EVENT.cop31,
      version: '1.0',
      issued_on: jourDe(ilYA(2)),
      publisher: 'IFDD',
      restricted: true,
      published_at: ilYA(2),
    }),
    graine({
      id: DOCUMENT_NEGO.bulletin11Novembre,
      slug: 'bulletin-des-negociations-de-la-terre-11-novembre',
      title: { fr: 'Bulletin des négociations de la Terre, 11 novembre' },
      summary: {
        fr: "Compte rendu quotidien des négociations, en français, par l'IISD. Numéro du mercredi 11 novembre : plénières d'ouverture, premières consultations informelles sur l'objectif mondial d'adaptation.",
      },
      type: 'bulletin',
      cop: EVENT.cop31,
      issued_on: jourDe(ilYA(1)),
      publisher: 'IISD',
      external_url: 'https://enb.iisd.org/fr/cop31-bulletin-11-novembre',
      rag_eligible: true,
      published_at: ilYA(1),
    }),
    graine({
      ...pdf(4, 'au-nom-de-ma-delegation.pdf', 148, 9_400_000, '2025-10-06T08:40:00Z'),
      id: DOCUMENT_NEGO.auNomDeMaDelegation,
      slug: 'au-nom-de-ma-delegation',
      title: { fr: '« Au nom de ma délégation »' },
      summary: {
        fr: 'Guide pratique pour une négociatrice qui prend la parole pour la première fois : préparer une intervention, suivre un texte entre crochets, travailler avec son groupe. Deuxième édition, en français.',
      },
      type: 'negotiation_guide',
      version: 'Deuxième édition',
      issued_on: '2024-03-01',
      publisher: 'IISD',
      rag_eligible: true,
      published_at: '2025-10-06T09:00:00Z',
    }),
    graine({
      ...pdf(5, 'note-technique-bilan-cdp30.pdf', 24, 3_100_000, '2025-12-16T09:30:00Z'),
      id: DOCUMENT_NEGO.noteBilanCop30,
      slug: 'note-technique-bilan-de-la-cdp30',
      title: { fr: 'Note technique — Bilan de la CdP30' },
      summary: {
        fr: "Ce que la CdP30 de Belém a décidé sur l'adaptation et le financement, et ce qui restait ouvert pour la CdP31. Le Guide des négociations — CdP31 reprend et met à jour ce bilan.",
      },
      type: 'technical_note',
      themes: ['adaptation', 'finance'],
      cop: EVENT.cop30,
      version: '1.2',
      issued_on: '2025-12-15',
      publisher: 'IFDD',
      rag_eligible: true,
      published_at: '2025-12-16T10:00:00Z',
    }),
  ]
}

/** La note de la maquette (`04-lecteur`, écran 11b), et une note retirée pour l'historique. */
const semisDesNotes = (): Note[] => [
  {
    id: '0199b011-0000-7000-8000-000000000001',
    document_id: DOCUMENT_NEGO.guideCop31,
    page_index: 59,
    passage: "L'enjeu principal est la finalisation et l'adoption des 100 indicateurs du GGA",
    body: {
      fr: 'Les 100 indicateurs du GGA ont été adoptés à la CdP30. À la CdP31, la négociation porte sur leur mise en œuvre et sur le financement de leur suivi.',
      en: 'The 100 GGA indicators were adopted at COP30. At COP31, negotiations turn to their implementation and to funding their monitoring.',
    },
    author: EXPERT,
    posted_at: ilYA(1),
    withdrawn_at: null,
    withdrawn_by: null,
  },
  {
    id: '0199b011-0000-7000-8000-000000000002',
    document_id: DOCUMENT_NEGO.guideCop31,
    page_index: 19,
    passage: null,
    body: { fr: 'Le calendrier du deuxième bilan mondial a été avancé.' },
    author: EXPERT,
    posted_at: ilYA(6),
    withdrawn_at: ilYA(5),
    withdrawn_by: EXPERT,
  },
]

/** Les deux favoris de la maquette (`03-documents`, écran 06a). */
const semisDesFavoris = (): DocumentBookmark[] => [
  { document_id: DOCUMENT_NEGO.guideCop31, created_at: ilYA(1) },
  { document_id: DOCUMENT_NEGO.auNomDeMaDelegation, created_at: ilYA(3) },
]

let fiches: Fiche[] = semis()
let notes: Note[] = semisDesNotes()
let favoris: DocumentBookmark[] = semisDesFavoris()
let creees = 0
let posees = 0

export function reinitialiserLesDocuments(): void {
  fiches = semis()
  notes = semisDesNotes()
  favoris = semisDesFavoris()
  creees = 0
  posees = 0
}

const fiche = (id: Uuid): Fiche | undefined => fiches.find((f) => f.id === id)
const publiee = (id: Uuid): Fiche | undefined => fiches.find((f) => f.id === id && f.published_at !== null)
const prete = (f: Fiche): boolean => f.extraction?.status === 'ready'

// ---------------------------------------------------------------------------
// La forme lisible
// ---------------------------------------------------------------------------

/** Les pages recomposées : ce que l'extraction a gardé, et ce que la recherche lit. */
function pagesExtraites(f: Fiche): { outline: OutlineEntry[]; pages: ReadingPage[] } {
  const guide = f.id === DOCUMENT_NEGO.guideCop31
  const outline = guide ? SOMMAIRE_DU_GUIDE : [entree(f.title.fr, 1, 1)]
  const pages = Array.from({ length: f.extraction?.page_count ?? 0 }, (_, i): ReadingPage => {
    const index = i + 1
    // La page 59 porte son 3.6.1 avec le terme anglais : le titre nu du sommaire ferait doublon.
    const titres = titresDeLaPage(outline, index).filter((b) => !(guide && index === 59 && b.level === 3))
    const propres = guide ? PAGES_DU_GUIDE[index] : index === 1 && f.summary ? [para(s(f.summary.fr))] : undefined
    const blocks = [...titres, ...(propres ?? [para(s(`Texte de démonstration, page ${index}.`))])]
    return { index, label: String(index), blocks }
  })
  return { outline, pages }
}

function empreinteDeLecture(f: Fiche): string | null {
  return prete(f) ? empreinte([f.id, f.asset_id, f.extraction?.extracted_at, f.extraction?.large_text_choice]) : null
}

function lectureDe(f: Fiche): DocumentReading {
  const { outline, pages } = pagesExtraites(f)
  return {
    id: f.id,
    version: f.version,
    has_text: f.extraction?.has_text ?? false,
    large_text: f.extraction?.large_text ?? false,
    page_count: pages.length,
    outline,
    pages,
  }
}

const segmentsDe = (b: Block): Span[] => (b.kind === 'origin' ? (b.text ?? []) : b.spans)
const texteDePage = (p: ReadingPage): string =>
  p.blocks.map((b) => segmentsDe(b).map((x) => x.text).join('')).filter(Boolean).join('\n')

// ---------------------------------------------------------------------------
// Les lectures publiques
// ---------------------------------------------------------------------------

/** Le bout publié de la chaîne : un brouillon ne remplace encore rien, et interrompt la chaîne. */
function successeur(f: Fiche, langue: string): Successor | null {
  let bout: Successor | null = null
  const vus = new Set([f.id])
  let suivant = fiches.find((x) => x.supersedes_id === f.id && x.published_at)
  while (suivant?.published_at && !vus.has(suivant.id)) {
    const courant: Fiche = suivant
    vus.add(courant.id)
    bout = {
      id: courant.id,
      title: resolveI18nText(courant.title, langue),
      published_at: suivant.published_at,
      page_count: courant.extraction?.page_count ?? null,
    }
    suivant = fiches.find((x) => x.supersedes_id === courant.id && x.published_at)
  }
  return bout
}

/** Comme `domain::documents::hote` : ni schéma, ni compte, ni port, ni `www.`. */
function hote(url: string): string | null {
  const reste = url.includes('://') ? url.slice(url.indexOf('://') + 3) : url
  const nom = (reste.split(/[/?#]/)[0] ?? '').split('@').pop()?.split(':')[0]?.replace(/^(www\.)+/, '') ?? ''
  return nom ? nom.toLowerCase() : null
}

type Publiee = Fiche & { published_at: IsoDateTime }
const estPubliee = (f: Fiche): f is Publiee => f.published_at !== null

function enBibliotheque(f: Publiee, acces: boolean, langue: string): LibraryDocument {
  const ouvert = !f.restricted || acces
  const fichier = f.asset_id !== null
  const rendu = fichier && prete(f) ? f.extraction : null
  return {
    id: f.id,
    slug: f.slug,
    version: f.version,
    title: resolveI18nText(f.title, langue),
    summary: ouvert && f.summary ? resolveI18nText(f.summary, langue) : null,
    type: f.type,
    themes: ouvert ? [...f.themes] : [],
    themes_hidden: !ouvert,
    cop: f.cop,
    issued_on: f.issued_on,
    published_at: f.published_at,
    publisher: f.publisher,
    locale: f.locale,
    source: fichier ? 'file' : 'link',
    // Pour un lien, l'adresse est le contenu (SC-007) : un réservé la tait, son hôte reste.
    external_url: ouvert ? f.external_url : null,
    link_host: f.external_url ? hote(f.external_url) : null,
    restricted: f.restricted,
    accessible: ouvert,
    page_count: rendu?.page_count ?? null,
    reading_bytes: rendu?.reading_bytes ?? null,
    has_text: rendu?.has_text ?? false,
    large_text: rendu?.large_text ?? false,
    superseded_by: successeur(f, langue),
    reading_etag: rendu ? empreinteDeLecture(f) : null,
  }
}

const vocabulaire = (table: Libelles, cites: Set<string>, langue: string): VocabularyTerm[] =>
  Object.entries(table)
    .filter(([code]) => cites.has(code))
    .sort(([a, x], [b, y]) => x.sort_order - y.sort_order || ordre(a, b))
    .map(([code, t]) => ({ code, label: resolveI18nText(t.label, langue), sort_order: t.sort_order }))

/** Comme `repo::documents::cops` : le libellé d'édition, le sigle ou le titre, la plus récente d'abord. */
const copsCitees = (ids: Set<Uuid>, langue: string): VocabularyCop[] =>
  events
    .filter((e) => ids.has(e.id))
    .sort((a, b) => recent(a.starts_at, b.starts_at))
    .map((e) => ({ id: e.id, label: e.edition_label ?? e.acronym ?? resolveI18nText(e.title, langue), city: e.city }))

export function bibliothequeDeDocuments(langue = 'fr'): Etiquete<DocumentLibrary> {
  const acces = aLAcces()
  const documents = fiches
    .filter(estPubliee)
    .sort((a, b) => recent(a.published_at, b.published_at) || ordre(a.id, b.id))
    .map((f) => enBibliotheque(f, acces, langue))
  const vocabulary = {
    types: vocabulaire(TYPES, new Set(documents.map((d) => d.type)), langue),
    themes: vocabulaire(THEMES, new Set(documents.flatMap((d) => d.themes)), langue),
    cops: copsCitees(new Set(documents.flatMap((d) => (d.cop ? [d.cop] : []))), langue),
  }
  // `served_at` hors de l'empreinte, comme côté API : il change à chaque lecture.
  return { valeur: { documents, vocabulary, served_at: maintenant() }, empreinte: empreinte({ documents, vocabulary }) }
}

/** Le document lisible par cette personne, dans l'ordre des refus de l'API. */
function lisible(id: Uuid): Fiche {
  const f = publiee(id)
  if (!f) throw introuvable()
  if (f.restricted && !aLAcces()) throw reserve()
  if (!f.asset_id) {
    throw refus('NEGOTIATION_DOCUMENT_NOT_READABLE', 409, "Ce document ne se lit pas dans l'application : ouvrez-le dans le navigateur.")
  }
  if (!prete(f)) throw introuvable()
  return f
}

const reserve = () =>
  refus(
    'NEGOTIATION_DOCUMENT_RESTRICTED',
    403,
    "Ce document est réservé aux négociatrices et négociateurs. Saisissez votre code d'invitation pour l'ouvrir.",
  )

export function lectureDuDocument(id: Uuid): Etiquete<DocumentReading> {
  const f = lisible(id)
  return { valeur: lectureDe(f), empreinte: empreinteDeLecture(f) }
}

/** Un lien se compte comme un fichier : seuls le non-publié et le réservé refusent. */
export function compterUnTelechargement(id: Uuid): void {
  const f = publiee(id)
  if (!f) throw introuvable()
  if (f.restricted && !aLAcces()) throw reserve()
}

/** Les mots que la configuration `french` ignore : une requête qui ne porte qu'eux ne trouve rien. */
const MOTS_VIDES = new Set(
  'a au aux ce ces d dans de des du en et l la le les ou par pour sur un une'.split(' '),
)

/** Les racines d'un texte, à la manière de `to_tsvector('french', unaccent(…))` : ni accent, ni casse, ni pluriel. */
const racinesDe = (texte: string): string[] =>
  sansAccents(texte)
    .split(/[^a-z0-9]+/)
    .filter((m) => m && !MOTS_VIDES.has(m))
    .map((m) => (m.length > 3 ? m.replace(/[sx]$/, '') : m))

/** Vingt-quatre mots au plus autour du premier passage, sans marque, comme `ts_headline`. */
function extrait(texte: string, cherchees: Set<string>): string {
  const mots = texte.split(/\s+/).filter(Boolean)
  const k = Math.max(0, mots.findIndex((m) => racinesDe(m).some((r) => cherchees.has(r))))
  const debut = Math.max(0, k - 8)
  return mots.slice(debut, debut + 24).join(' ')
}

/**
 * Tous les mots de la requête, où qu'ils soient dans la page, comme
 * `websearch_to_tsquery`. Le texte cherché est celui des pages extraites, quel
 * que soit le mode : « Texte agrandi » change la lecture, pas l'index.
 */
export function rechercherDansLesDocuments(q: string): DocumentTextHits {
  const requete = q.trim()
  if (!requete) throw invalide('Saisissez un mot à chercher.', 'q')
  const cherchees = new Set(racinesDe(requete))
  const acces = aLAcces()
  const trouvees = fiches
    .filter((f) => f.published_at && f.asset_id && prete(f) && cherchees.size > 0)
    .flatMap((f) =>
      pagesExtraites(f).pages.flatMap((p) => {
        const racines = racinesDe(texteDePage(p))
        const toutes = [...cherchees].every((r) => racines.includes(r))
        return toutes ? [{ f, p, rang: racines.filter((r) => cherchees.has(r)).length }] : []
      }),
    )
    .sort((a, b) => b.rang - a.rang || ordre(a.f.id, b.f.id) || a.p.index - b.p.index)
  const hits = new Map<Uuid, TextHit>()
  for (const { f, p } of trouvees) {
    const hit = hits.get(f.id) ?? { document_id: f.id, pages: [] }
    hits.set(f.id, hit)
    // Un réservé sans accès se dit trouvé, sans rien montrer de ce qu'il contient.
    if ((!f.restricted || acces) && hit.pages.length < 5) {
      hit.pages.push({ index: p.index, label: p.label, excerpt: extrait(texteDePage(p), cherchees) })
    }
  }
  return { query: requete, hits: [...hits.values()] }
}

/** Comme `texte_dans` côté API : la langue demandée si la note l'a, le français sinon. */
const texteDans = (texte: I18nText, langue: string): string => texte[langue] ?? texte.fr ?? ''
const parPage = (a: Note, b: Note): number =>
  a.page_index - b.page_index || -recent(a.posted_at, b.posted_at) || ordre(a.id, b.id)

export function notesDeCorrection(langue = 'fr'): Etiquete<CorrectionNoteList> {
  const acces = aLAcces()
  const visible = (n: Note): boolean => {
    const f = publiee(n.document_id)
    return !n.withdrawn_at && f !== undefined && (!f.restricted || acces)
  }
  const valeur: CorrectionNoteList = {
    notes: notes
      .filter(visible)
      .sort((a, b) => ordre(a.document_id, b.document_id) || parPage(a, b))
      .map((n) => ({
        id: n.id,
        document_id: n.document_id,
        page_index: n.page_index,
        passage: n.passage,
        body: texteDans(n.body, langue),
        author_name: n.author.name,
        posted_at: n.posted_at,
      })),
  }
  return { valeur, empreinte: empreinte(valeur) }
}

export function mesFavorisDeDocuments(): Etiquete<DocumentBookmarkList> {
  const bookmarks = [...favoris]
    .sort((a, b) => recent(a.created_at, b.created_at) || ordre(a.document_id, b.document_id))
    .map((b) => ({ ...b }))
  return { valeur: { bookmarks }, empreinte: empreinte(bookmarks.map((b) => b.document_id)) }
}

export function poserUnFavori(id: Uuid): void {
  if (!publiee(id)) throw introuvable()
  if (!favoris.some((b) => b.document_id === id)) favoris = [...favoris, { document_id: id, created_at: maintenant() }]
}

export function retirerUnFavori(id: Uuid): void {
  favoris = favoris.filter((b) => b.document_id !== id)
}

// ---------------------------------------------------------------------------
// Le back-office
// ---------------------------------------------------------------------------

const etat = (f: Fiche): AdminDocumentState => (f.published_at ? 'published' : f.unpublished_at ? 'unpublished' : 'draft')
const lien = (f: Fiche | undefined, langue: string): DocumentLink | null =>
  f ? { id: f.id, title: resolveI18nText(f.title, langue), version: f.version } : null
const dejaPubliee = (f: Fiche): boolean => f.published_at !== null || f.unpublished_at !== null

/** L'extraction simulée se termine à la relecture qui suit le dépôt : l'écran qui la guette la voit passer. */
function ficheOuRefus(id: Uuid): Fiche {
  const f = fiche(id)
  if (!f) throw introuvable()
  if (f.extraction?.status === 'pending' || f.extraction?.status === 'extracting') {
    f.extraction = extraite(12, 1_200_000, maintenant())
  }
  return f
}

function vue(f: Fiche, langue: string): AdminDocument {
  const { supersedes_id: remplace, ...reste } = structuredClone(f)
  return {
    ...reste,
    source: f.asset_id ? 'file' : f.external_url ? 'link' : null,
    supersedes: lien(remplace ? fiche(remplace) : undefined, langue),
    superseded_by: lien(fiches.find((x) => x.supersedes_id === f.id), langue),
    state: etat(f),
    file_locked: dejaPubliee(f),
    can_publish: true,
    can_correct: true,
  }
}

export function ficheDuDocument(id: Uuid, langue = 'fr'): AdminDocument {
  return vue(ficheOuRefus(id), langue)
}

export function listeDesDocuments(langue = 'fr'): AdminDocumentList {
  const lignes = [...fiches].sort((a, b) => recent(a.updated_at, b.updated_at) || ordre(a.id, b.id))
  return {
    documents: lignes.map((f) => {
      const d = ficheDuDocument(f.id, langue)
      const { id, type, version, state, source, restricted, published_at, updated_at, supersedes, superseded_by, extraction } = d
      const title = resolveI18nText(d.title, langue)
      return { id, title, type, version, state, source, restricted, published_at, updated_at, supersedes, superseded_by, extraction }
    }),
    can_publish: true,
    can_correct: true,
  }
}

/** Vrai si `cible` se trouve en remontant les remplacés depuis `depart`, lui compris. */
function remonte(depart: Uuid, cible: Uuid): boolean {
  const vus = new Set<Uuid>()
  for (let c = fiche(depart); c && !vus.has(c.id); c = c.supersedes_id ? fiche(c.supersedes_id) : undefined) {
    if (c.id === cible) return true
    vus.add(c.id)
  }
  return false
}

function texteValide(texte: I18nText, champ: string): void {
  if (!texte.fr?.trim()) throw invalide('Le texte en français est obligatoire.', champ)
}

/** Le refus d'un second remplaçant nomme celui qui existe déjà, par sa version : le titre ne change pas. */
const dejaRemplace = (successeurDirect: Fiche) =>
  refus(
    'NEGOTIATION_DOCUMENT_ALREADY_SUPERSEDED',
    409,
    `Ce document est déjà remplacé par la version ${successeurDirect.version}.`,
    'supersedes_id',
  )

/**
 * Un champ absent ne change rien ; `null` vide un champ facultatif — les clés
 * d'entrée sont celles de la fiche. Tout se vérifie avant d'écrire, dans
 * l'ordre de l'API : service, déclencheurs, contraintes, index, thématiques.
 */
function appliquer(f: Fiche, e: AdminDocumentInput, langue: string): void {
  if (e.type !== undefined && !TYPES[e.type]) {
    throw refus('NEGOTIATION_DOCUMENT_UNKNOWN_TYPE', 400, "Ce type de document n'existe pas.", 'type')
  }
  if (e.cop && !events.some((x) => x.id === e.cop)) throw invalide("Cette COP n'existe pas.", 'cop')
  if (e.supersedes_id && remonte(e.supersedes_id, f.id)) {
    throw refus('NEGOTIATION_DOCUMENT_SUPERSEDE_CYCLE', 409, 'Ce remplacement formerait une boucle.', 'supersedes_id')
  }
  const url = e.external_url === undefined ? f.external_url : e.external_url
  if (url && f.asset_id) throw lesDeux()
  const autre = e.supersedes_id ? fiches.find((x) => x.supersedes_id === e.supersedes_id && x.id !== f.id) : undefined
  if (autre) throw dejaRemplace(autre)
  const inconnue = e.themes?.find((code) => !THEMES[code])
  if (inconnue !== undefined) {
    throw refus('NEGOTIATION_DOCUMENT_UNKNOWN_THEME', 400, `Cette thématique n'existe pas : ${inconnue}.`, 'themes')
  }
  const { themes, version, ...reste } = structuredClone(e)
  const definis = Object.fromEntries(Object.entries(reste).filter(([, v]) => v !== undefined))
  Object.assign(
    f,
    definis,
    themes ? { themes: [...new Set(themes)] } : {},
    version?.trim() ? { version } : {},
    { updated_at: maintenant() },
  )
}

const slugDe = (titre: string): string => sansAccents(titre).replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'document'

export function creerUnDocument(e: AdminDocumentInput, langue = 'fr'): AdminDocument {
  if (!e.title) throw invalide('Le titre est obligatoire.', 'title')
  texteValide(e.title, 'title')
  if (e.summary) texteValide(e.summary, 'summary')
  if (!e.type) throw invalide('Le type est obligatoire.', 'type')
  const id = `0199b010-0000-7000-8000-${String(901 + creees).padStart(12, '0')}`
  // Le slug suit le titre, suffixé d'un morceau de l'identifiant, comme `repo::documents::creer`.
  const slug = `${slugDe(e.title.fr)}-${id.replace(/-/g, '').slice(24, 32)}`
  const f = graine({ id, slug, title: e.title, type: e.type })
  appliquer(f, e, langue)
  creees += 1
  fiches = [...fiches, f]
  return vue(f, langue)
}

export function modifierLeDocument(id: Uuid, e: AdminDocumentInput, langue = 'fr'): AdminDocument {
  if (e.title) texteValide(e.title, 'title')
  if (e.summary) texteValide(e.summary, 'summary')
  const f = ficheOuRefus(id)
  // Le champ présent suffit, même inchangé : un formulaire qui renvoie tout échoue ici comme en ligne.
  if (e.external_url !== undefined && dejaPubliee(f)) throw figee('external_url')
  appliquer(f, e, langue)
  return vue(f, langue)
}

export function supprimerLeDocument(id: Uuid): void {
  if (dejaPubliee(ficheOuRefus(id))) {
    throw refus('NEGOTIATION_DOCUMENT_PUBLISHED_UNDELETABLE', 409, 'Un document publié ne se supprime pas : dépubliez-le.')
  }
  fiches = fiches.filter((x) => x.id !== id)
  notes = notes.filter((n) => n.document_id !== id)
}

export function attacherLeFichier(id: Uuid, assetId: Uuid, langue = 'fr'): AdminDocument {
  const f = ficheOuRefus(id)
  if (dejaPubliee(f)) throw figee('asset_id')
  if (f.external_url) throw lesDeux()
  Object.assign(f, {
    asset_id: assetId,
    file: { filename: null, byte_size: 0, mime_type: 'application/pdf' },
    extraction: enFile(),
    updated_at: maintenant(),
  })
  return vue(f, langue)
}

export function relancerLExtraction(id: Uuid): void {
  const f = ficheOuRefus(id)
  if (dejaPubliee(f)) throw figee()
  if (!f.asset_id) throw invalide("Ce document n'a pas de fichier à extraire.", 'asset_id')
  f.extraction = enFile()
}

/** Toute extraction du document se règle, prête ou non : c'est le recours quand elle échoue. */
export function choisirLeTexteAgrandi(id: Uuid, choice: boolean | null, langue = 'fr'): AdminDocument {
  const f = ficheOuRefus(id)
  if (!f.extraction) throw invalide("Ce document n'a pas encore de fichier extrait.", 'choice')
  f.extraction = avecLesModes({ ...f.extraction, large_text_choice: choice })
  f.updated_at = maintenant()
  return vue(f, langue)
}

export function publierLeDocument(id: Uuid, langue = 'fr'): AdminDocument {
  const f = ficheOuRefus(id)
  if (f.published_at) return vue(f, langue)
  if (f.asset_id && !prete(f)) {
    throw refus('NEGOTIATION_DOCUMENT_NOT_READY', 409, "L'extraction n'est pas terminée. Attendez-la avant de publier.")
  }
  if (!f.asset_id && !f.external_url) {
    throw refus('NEGOTIATION_DOCUMENT_SOURCE_MISSING', 422, 'Déposez un fichier ou indiquez un lien avant de publier.')
  }
  Object.assign(f, { published_at: maintenant(), unpublished_at: null, updated_at: maintenant() })
  return vue(f, langue)
}

/** Un brouillon jamais publié reste un brouillon : rien ne se dépublie qui n'est pas publié. */
export function depublierLeDocument(id: Uuid, langue = 'fr'): AdminDocument {
  const f = ficheOuRefus(id)
  if (f.published_at) Object.assign(f, { published_at: null, unpublished_at: maintenant(), updated_at: maintenant() })
  return vue(f, langue)
}

export function nouvelleVersion(id: Uuid, langue = 'fr'): AdminDocument {
  const a = ficheOuRefus(id)
  const deja = fiches.find((x) => x.supersedes_id === id)
  if (deja) throw dejaRemplace(deja)
  const { title, summary, type, themes, cop, publisher, locale, restricted, rag_eligible } = structuredClone(a)
  const cree = creerUnDocument(
    {
      title, summary, type, themes, cop, publisher, locale, restricted, rag_eligible,
      version: `${a.version} (nouvelle version)`,
      supersedes_id: a.id,
    },
    langue,
  )
  const f = fiche(cree.id)
  if (!f) throw introuvable()
  f.slug = a.slug
  return vue(f, langue)
}

function qualite(pages: ReadingPage[], outline: OutlineEntry[]): ExtractionQuality {
  const blocs = pages.flatMap((p) => p.blocks)
  const compter = (voulu: (b: Block) => boolean) => blocs.filter(voulu).length
  const entrees = (liste: OutlineEntry[]): number => liste.reduce((n, e) => n + 1 + entrees(e.children), 0)
  return {
    pages: pages.length,
    pages_avec_texte: pages.filter((p) => texteDePage(p).length > 0).length,
    pages_a_origine: pages.filter((p) => p.blocks.some((b) => b.kind === 'origin')).length,
    tableaux: compter((b) => b.kind === 'origin' && b.reason === 'table'),
    figures: compter((b) => b.kind === 'origin' && b.reason === 'figure'),
    notes: compter((b) => b.kind === 'note'),
    titres: compter((b) => b.kind === 'heading'),
    entrees_du_sommaire: entrees(outline),
    sommaire_depuis_signets: false,
    italiques_maigres: 0,
    termes: blocs.flatMap(segmentsDe).filter((x) => x.term).length,
    cesures_gardees: 0,
    cesures_recollees: 0,
  }
}

export function apercuDuDocument(id: Uuid): AdminDocumentPreview {
  const f = ficheOuRefus(id)
  const { outline, pages } = prete(f) ? pagesExtraites(f) : { outline: [], pages: [] }
  return {
    id: f.id,
    extraction: f.extraction ? { ...f.extraction } : null,
    quality: prete(f) ? qualite(pages, outline) : null,
    extractor: prete(f) ? "pdfium-render 0.9.4 / pdfium chromium-7881 / règles (jeu d'exemple)" : null,
    outline,
    pages: pages.map((p) => {
      const origine = p.blocks.some((b) => b.kind === 'origin')
      // L'extraction rend l'image de chaque page, pour l'aperçu seul.
      const image = `/admin/negotiation/documents/${f.id}/pages/${p.index}/image`
      return { index: p.index, label: p.label, blocks: p.blocks, image, has_origin_block: origine }
    }),
  }
}

export function notesDuDocument(id: Uuid): AdminCorrectionNoteList {
  ficheOuRefus(id)
  const du = notes.filter((n) => n.document_id === id).sort(parPage)
  return { notes: structuredClone(du), can_post: true, can_withdraw: true }
}

export function poserUneNote(id: Uuid, e: CorrectionNoteInput): AdminCorrectionNote {
  if (!e.body.fr?.trim()) throw invalide('Le texte de la note en français est obligatoire.', 'body')
  const f = ficheOuRefus(id)
  if (e.page_index < 1 || e.page_index > (prete(f) ? (f.extraction?.page_count ?? 0) : 0)) {
    throw refus('NEGOTIATION_CORRECTION_PAGE_UNKNOWN', 422, "Cette page n'existe pas dans le document.", 'page_index')
  }
  posees += 1
  const note: Note = {
    id: `0199b011-0000-7000-8000-${String(900 + posees).padStart(12, '0')}`,
    document_id: id,
    page_index: e.page_index,
    passage: e.passage?.trim() || null,
    body: { ...e.body },
    author: EXPERT,
    posted_at: maintenant(),
    withdrawn_at: null,
    withdrawn_by: null,
  }
  notes = [...notes, note]
  return structuredClone(note)
}

/** Idempotent : retirer deux fois garde la première date. */
export function retirerUneNote(noteId: Uuid): AdminCorrectionNote {
  const note = notes.find((n) => n.id === noteId)
  if (!note) throw refus('NOT_FOUND', 404, 'La ressource demandée est introuvable.')
  if (!note.withdrawn_at) Object.assign(note, { withdrawn_at: maintenant(), withdrawn_by: EXPERT })
  return structuredClone(note)
}
