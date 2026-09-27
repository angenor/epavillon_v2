/**
 * Le back-office du savoir de Guide Négo, sans API. Amorcé depuis
 * `negotiation-savoir.ts` ; les refus reprennent les codes de l'API.
 */
import type { I18nText, IsoDate, IsoDateTime, Uuid } from '~/types/shared'
import type {
  AdminFaqEntry,
  AdminFaqFeedback,
  AdminFaqInput,
  AdminFaqList,
  AdminFaqReport,
  AdminFaqVerifyInput,
  AdminGlossaryEntry,
  AdminGlossaryInput,
  AdminGlossaryList,
  AdminKnowledgeRef,
  AdminKnowledgeSource,
  AdminKnowledgeSourceInput,
  AdminKnowledgeStatus,
  AdminPathway,
  AdminPathwayGroupInput,
  AdminPathwayLink,
  AdminPathwayOrderInput,
  AdminPathwayStepInput,
} from '~/types/admin-negotiation-savoir'
import type {
  AdminFaqReportCloseInput,
  AdminQuestion,
  AdminQuestionAnswerInput,
  AdminQuestionPromoteInput,
  ExpertQueue,
  ExpertQueueKind,
  ExpertQueueReportGroup,
} from '~/types/admin-negotiation-queue'
import type { KnowledgeSource } from '~/types/negotiation-savoir'
import type { FiltreFaq, FiltreLexique } from '~/composables/api/admin-negotiation-savoir'
import type { ApiErrorCode } from '~/types/api-error'
import { ApiRequestError } from '~/utils/api-error'
import { resolveI18nText } from '~/utils/i18n-text'
import { normalizeLabelLike } from '~/utils/slug'
import { slugDe } from '~/utils/guide-nego/lexique'
import { FAQ, LEXIQUE, PARCOURS } from './negotiation-savoir'
import { listeDesDocuments } from './negotiation-documents'
import {
  aPromouvoir,
  marquerPromue,
  questionsDeLaFile,
  questionsEnAttente,
  repondreAUneQuestion as repondre,
} from './negotiation-questions'
import { aAccepter, marquerAcceptee, propositionsDeLaFile, propositionsEnAttente } from './negotiation-propositions'

type Changement = 'publish' | 'to-review' | 'unpublish'

interface Faq {
  id: Uuid
  section_code: string
  question: I18nText
  answer: I18nText | null
  status: AdminKnowledgeStatus
  verified_on: IsoDate | null
  verified_by_name: string | null
  editorial_rank: number | null
  sources: AdminKnowledgeSourceInput[]
  related_ids: Uuid[]
  feedback: AdminFaqFeedback
  reports: AdminFaqReport[]
  first_published_at: IsoDateTime | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

interface Terme {
  id: Uuid
  slug: string
  family_code: string
  term: string
  acronym: string | null
  variants: string[]
  translation: I18nText
  definition: I18nText
  heard_in_room: string | null
  status: AdminKnowledgeStatus
  sources: AdminKnowledgeSourceInput[]
  related_ids: Uuid[]
  first_published_at: IsoDateTime | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

interface Etape {
  id: Uuid
  label: I18nText
  detail: I18nText | null
  origin_label: I18nText | null
  link: Omit<AdminPathwayLink, 'target_label'> | null
  is_published: boolean
  checks: number
}

interface Groupe {
  id: Uuid
  label: I18nText
  is_published: boolean
  steps: Etape[]
}

const EXPERT = 'Aminata Sow'
const AMORCE: IsoDateTime = '2026-09-20T08:00:00Z'
const SANS_RETOUR: AdminFaqFeedback = { helpful: 0, not_helpful: 0, too_vague: 0, off_topic: 0, outdated: 0 }

const maintenant = (): IsoDateTime => new Date().toISOString()
const aujourdhuiAParis = (): IsoDate =>
  new Intl.DateTimeFormat('en-CA', { timeZone: 'Europe/Paris' }).format(new Date())
const fr = (texte: string): I18nText => ({ fr: texte })
const texteOuNul = (texte: string | null | undefined): I18nText | null => (texte ? fr(texte) : null)

const refus = (code: ApiErrorCode, status: number, message: string, field?: string) =>
  new ApiRequestError({ code, message, field }, status)
const invalide = (message: string, field: string) => refus('VALIDATION_FAILED', 422, message, field)

function sourceDe(s: KnowledgeSource): AdminKnowledgeSourceInput {
  return {
    document_id: s.document_id ?? null,
    external_title: s.external_title ?? null,
    external_url: s.external_url ?? null,
    section_label: s.section_label ?? null,
    page_from: s.page_from ?? null,
    page_to: s.page_to ?? null,
    quote: s.quote ?? null,
  }
}

// ---------------------------------------------------------------------------
// L'état, amorcé une fois
// ---------------------------------------------------------------------------

let faq: Faq[] = [
  ...FAQ.map(
    (e): Faq => ({
      id: e.id,
      section_code: e.section_code,
      question: fr(e.question),
      answer: fr(e.answer),
      status: e.status,
      verified_on: e.verified_on,
      verified_by_name: EXPERT,
      editorial_rank: 1,
      sources: e.sources.map(sourceDe),
      related_ids: [...e.related_ids],
      feedback: { helpful: 14, not_helpful: 3, too_vague: 1, off_topic: 0, outdated: 2 },
      reports: [
        {
          id: '00000000-0013-7000-8000-e00000000001',
          reasons: ['rule_changed'],
          from_feedback: false,
          details: 'Depuis la CdP30, les observateurs sont admis dans les consultations informelles sauf objection.',
          status: 'open',
          outcome: null,
          created_at: '2026-09-24T15:12:00Z',
          handled_at: null,
        },
        {
          id: '00000000-0013-7000-8000-e00000000002',
          reasons: [],
          from_feedback: true,
          details: null,
          status: 'open',
          outcome: null,
          created_at: '2026-09-23T09:40:00Z',
          handled_at: null,
        },
      ],
      first_published_at: AMORCE,
      created_at: AMORCE,
      updated_at: e.updated_at,
    }),
  ),
  ...FAQ.flatMap((e) => e.related_ids).map(
    (id, i): Faq => ({
      id,
      section_code: i === 0 ? 'process' : 'first_cop',
      question: fr(i === 0 ? "Qu'est-ce qu'un document L ?" : 'Comment se repérer dans les salles de négociation ?'),
      answer: null,
      status: 'draft',
      verified_on: null,
      verified_by_name: null,
      editorial_rank: null,
      sources: [],
      related_ids: [],
      feedback: { ...SANS_RETOUR },
      reports: [],
      first_published_at: null,
      created_at: AMORCE,
      updated_at: AMORCE,
    }),
  ),
]

let lexique: Terme[] = LEXIQUE.map((e) => ({
  id: e.id,
  slug: e.slug,
  family_code: e.family_code,
  term: e.term,
  acronym: e.acronym,
  variants: [...e.variants],
  translation: fr(e.translation),
  definition: fr(e.definition),
  heard_in_room: e.heard_in_room,
  status: e.status,
  sources: e.sources.map(sourceDe),
  related_ids: [...e.related_ids],
  first_published_at: AMORCE,
  created_at: AMORCE,
  updated_at: e.updated_at,
}))

let groupes: Groupe[] = PARCOURS.map((g) => ({
  id: g.id,
  label: fr(g.label),
  is_published: true,
  steps: g.steps.map((s, i) => ({
    id: s.id,
    label: fr(s.label),
    detail: texteOuNul(s.detail),
    origin_label: texteOuNul(s.origin_label),
    link: s.link
      ? { kind: s.link.kind, target_id: s.link.target_id, page: s.link.page, section: s.link.section, label: texteOuNul(s.link.label) }
      : null,
    is_published: true,
    checks: i < 2 ? 5 - i * 2 : 0,
  })),
}))

// ---------------------------------------------------------------------------
// Commun : sources et liées
// ---------------------------------------------------------------------------

function titreDuDocument(id: Uuid): string | null {
  return listeDesDocuments().documents.find((d) => d.id === id)?.title ?? null
}

function sourcesRendues(sources: AdminKnowledgeSourceInput[]): AdminKnowledgeSource[] {
  return sources.map((s) => ({ ...s, document_title: s.document_id ? titreDuDocument(s.document_id) : null }))
}

function sourcesValides(sources: AdminKnowledgeSourceInput[]): AdminKnowledgeSourceInput[] {
  for (const s of sources) {
    const titre = s.external_title?.trim() ?? ''
    if (Boolean(s.document_id) === Boolean(titre)) {
      throw refus(
        'NEGOTIATION_SOURCE_TARGET_INVALID',
        422,
        'Une source est un document de la bibliothèque ou une référence extérieure titrée, jamais les deux.',
        'sources',
      )
    }
    if (s.page_from !== null && s.page_to !== null && s.page_to < s.page_from) {
      throw invalide('La dernière page ne peut précéder la première.', 'sources')
    }
  }
  return sources.map((s) => ({ ...s }))
}

function lieesValides(soi: Uuid | null, ids: Uuid[], existe: (id: Uuid) => boolean): Uuid[] {
  if (soi && ids.includes(soi)) {
    throw refus('NEGOTIATION_RELATED_SELF', 422, 'Une entrée ne peut pas être liée à elle-même.', 'related_ids')
  }
  if (ids.some((id) => !existe(id))) throw invalide('Une entrée liée est introuvable.', 'related_ids')
  return [...new Set(ids)]
}

function texteFrancais(texte: I18nText | null | undefined, champ: string): I18nText {
  if (!texte?.fr?.trim()) throw invalide('Le texte en français est requis.', champ)
  return { ...texte }
}

function transition(
  entree: { status: AdminKnowledgeStatus; first_published_at: IsoDateTime | null; updated_at: IsoDateTime },
  changement: Changement,
): void {
  if (changement === 'publish') {
    entree.status = 'published'
    entree.first_published_at ??= maintenant()
  } else if (changement === 'to-review') {
    if (entree.status === 'draft') throw refus('CONFLICT', 409, 'Seule une entrée publiée peut être mise « À revoir ».')
    entree.status = 'to_review'
  } else {
    entree.status = 'draft'
  }
  entree.updated_at = maintenant()
}

const indelebile = () =>
  refus('NEGOTIATION_KNOWLEDGE_PUBLISHED_UNDELETABLE', 409, 'Une entrée déjà publiée ne se supprime pas : dépubliez-la.')

// ---------------------------------------------------------------------------
// FAQ
// ---------------------------------------------------------------------------

function faqOuRefus(id: Uuid): Faq {
  const e = faq.find((x) => x.id === id)
  if (!e) throw refus('NEGOTIATION_FAQ_NOT_FOUND', 404, "Cette question n'existe pas, ou n'est plus publiée.")
  return e
}

function refFaq(id: Uuid): AdminKnowledgeRef[] {
  const e = faq.find((x) => x.id === id)
  return e ? [{ id, label: resolveI18nText(e.question, 'fr'), status: e.status }] : []
}

function vueFaq(e: Faq): AdminFaqEntry {
  const { related_ids, sources, ...reste } = e
  return {
    ...structuredClone(reste),
    origin_question_id: null,
    sources: sourcesRendues(sources),
    related: related_ids.flatMap(refFaq),
    can_publish: true,
    can_review: true,
  }
}

export function listeFaq(filtre: FiltreFaq): AdminFaqList {
  const q = normalizeLabelLike(filtre.q)
  const entries = faq
    .filter(
      (e) =>
        (!filtre.section || e.section_code === filtre.section) &&
        (!filtre.status || e.status === filtre.status) &&
        (!q || normalizeLabelLike(resolveI18nText(e.question, 'fr')).includes(q)),
    )
    .sort((a, b) => Date.parse(b.updated_at) - Date.parse(a.updated_at))
    .map((e) => ({
      id: e.id,
      section_code: e.section_code,
      question: resolveI18nText(e.question, 'fr'),
      status: e.status,
      verified_on: e.verified_on,
      has_answer: Boolean(e.answer?.fr),
      open_reports: e.reports.filter((r) => r.status === 'open').length,
      first_published_at: e.first_published_at,
      updated_at: e.updated_at,
    }))
  return { entries, can_publish: true, can_review: true }
}

export function ficheFaq(id: Uuid): AdminFaqEntry {
  return vueFaq(faqOuRefus(id))
}

function appliquerFaq(e: Faq, entree: AdminFaqInput): void {
  if (entree.section_code !== undefined) {
    if (!entree.section_code) throw invalide('La rubrique est requise.', 'section_code')
    e.section_code = entree.section_code
  }
  if (entree.question !== undefined) e.question = texteFrancais(entree.question, 'question')
  if (entree.answer !== undefined) {
    if (entree.answer === null && e.status !== 'draft') throw invalide('Une entrée publiée garde sa réponse.', 'answer')
    e.answer = entree.answer ? texteFrancais(entree.answer, 'answer') : null
  }
  if (entree.editorial_rank !== undefined) e.editorial_rank = entree.editorial_rank
  if (entree.sources !== undefined) e.sources = sourcesValides(entree.sources)
  if (entree.related_ids !== undefined) {
    e.related_ids = lieesValides(e.id, entree.related_ids, (id) => faq.some((x) => x.id === id))
  }
  e.updated_at = maintenant()
}

export function creerFaq(entree: AdminFaqInput): AdminFaqEntry {
  if (!entree.section_code) throw invalide('La rubrique est requise.', 'section_code')
  if (!entree.question) throw invalide('La question est requise.', 'question')
  const e: Faq = {
    id: crypto.randomUUID(),
    section_code: entree.section_code,
    question: fr(''),
    answer: null,
    status: 'draft',
    verified_on: null,
    verified_by_name: null,
    editorial_rank: null,
    sources: [],
    related_ids: [],
    feedback: { ...SANS_RETOUR },
    reports: [],
    first_published_at: null,
    created_at: maintenant(),
    updated_at: maintenant(),
  }
  appliquerFaq(e, entree)
  faq = [...faq, e]
  return vueFaq(e)
}

export function modifierFaq(id: Uuid, entree: AdminFaqInput): AdminFaqEntry {
  const e = faqOuRefus(id)
  appliquerFaq(e, entree)
  return vueFaq(e)
}

export function verifierFaq(id: Uuid, entree: AdminFaqVerifyInput): AdminFaqEntry {
  const e = faqOuRefus(id)
  e.verified_on = entree.verified_on ?? aujourdhuiAParis()
  e.verified_by_name = EXPERT
  if (e.status === 'to_review') e.status = 'published'
  e.updated_at = maintenant()
  return vueFaq(e)
}

export function changerFaq(id: Uuid, changement: Changement): AdminFaqEntry {
  const e = faqOuRefus(id)
  if (changement === 'publish') {
    if (!e.verified_on) {
      throw refus(
        'NEGOTIATION_FAQ_UNVERIFIED',
        422,
        "Une réponse ne se publie qu'avec la date de sa vérification par un expert.",
      )
    }
    if (!e.answer?.fr) throw invalide('Une entrée publiée porte sa réponse.', 'answer')
  }
  transition(e, changement)
  return vueFaq(e)
}

export function supprimerFaq(id: Uuid): void {
  const e = faqOuRefus(id)
  if (e.first_published_at) throw indelebile()
  faq = faq.filter((x) => x.id !== id).map((x) => ({ ...x, related_ids: x.related_ids.filter((r) => r !== id) }))
}

// ---------------------------------------------------------------------------
// File des experts
// ---------------------------------------------------------------------------

const ISSUES: AdminFaqReportCloseInput['outcome'][] = ['revised', 'confirmed', 'dismissed']

export function fileDesExperts(kind: ExpertQueueKind): ExpertQueue {
  const groupes: ExpertQueueReportGroup[] = faq
    .map((e) => ({
      e,
      ouverts: e.reports
        .filter((r) => r.status === 'open')
        .sort((a, b) => a.created_at.localeCompare(b.created_at)),
    }))
    .filter(({ ouverts }) => ouverts.length > 0)
    .sort((a, b) => (a.ouverts[0]?.created_at ?? '').localeCompare(b.ouverts[0]?.created_at ?? ''))
    .map(({ e, ouverts }) => ({
      entry: { id: e.id, question: resolveI18nText(e.question, 'fr'), status: e.status, verified_on: e.verified_on },
      feedback: { ...e.feedback },
      reports: structuredClone(ouverts),
    }))
  return {
    kind,
    counts: {
      reports: groupes.reduce((n, g) => n + g.reports.length, 0),
      questions: questionsEnAttente(),
      proposals: propositionsEnAttente(),
    },
    reports: kind === 'reports' ? groupes : [],
    questions: kind === 'questions' ? questionsDeLaFile() : [],
    proposals: kind === 'proposals' ? propositionsDeLaFile() : [],
  }
}

/** Le brouillon naît du formulaire du lexique ; le terme proposé tient lieu de terme absent. */
export function accepterUneProposition(id: Uuid, entree: AdminGlossaryInput): AdminGlossaryEntry {
  const term = aAccepter(id)
  const brouillon = creerTerme({ ...entree, term: entree.term?.trim() || term })
  marquerAcceptee(id, brouillon)
  return brouillon
}

export function repondreAUneQuestion(id: Uuid, entree: AdminQuestionAnswerInput): AdminQuestion {
  return repondre(id, entree.answer)
}

/** Brouillon sans auteur, dans la rubrique choisie. */
export function promouvoirUneQuestion(id: Uuid, entree: AdminQuestionPromoteInput): AdminFaqEntry {
  const { body, answer } = aPromouvoir(id)
  const brouillon = creerFaq({ section_code: entree.section_code, question: fr(body), answer: fr(answer) })
  marquerPromue(id)
  return { ...brouillon, origin_question_id: id }
}

/** Ne touche jamais l'entrée. */
export function cloreUnSignalement(id: Uuid, entree: AdminFaqReportCloseInput): AdminFaqReport {
  if (!ISSUES.includes(entree.outcome)) throw invalide("L'issue du signalement est inconnue.", 'outcome')
  const signalement = faq.flatMap((e) => e.reports).find((r) => r.id === id)
  if (!signalement) throw refus('NOT_FOUND', 404, "Ce signalement n'existe pas.")
  if (signalement.status === 'closed')
    throw refus('NEGOTIATION_QUEUE_ITEM_CLOSED', 409, 'Cet élément de la file a déjà été traité.')
  signalement.status = 'closed'
  signalement.outcome = entree.outcome
  signalement.handled_at = maintenant()
  return structuredClone(signalement)
}

// ---------------------------------------------------------------------------
// Lexique
// ---------------------------------------------------------------------------

function termeOuRefus(id: Uuid): Terme {
  const e = lexique.find((x) => x.id === id)
  if (!e) throw refus('NEGOTIATION_GLOSSARY_NOT_FOUND', 404, "Ce terme n'existe pas, ou n'est plus publié.")
  return e
}

function vueTerme(e: Terme): AdminGlossaryEntry {
  const { related_ids, sources, ...reste } = e
  return {
    ...structuredClone(reste),
    sources: sourcesRendues(sources),
    related: related_ids.flatMap((id) => {
      const lie = lexique.find((x) => x.id === id)
      return lie ? [{ id, label: lie.term, status: lie.status }] : []
    }),
    can_publish: true,
    can_review: true,
  }
}

export function listeLexique(filtre: FiltreLexique): AdminGlossaryList {
  const q = normalizeLabelLike(filtre.q)
  const entries = lexique
    .filter(
      (e) =>
        (!filtre.family || e.family_code === filtre.family) &&
        (!filtre.status || e.status === filtre.status) &&
        (!q ||
          [e.term, e.acronym ?? '', resolveI18nText(e.translation, 'fr'), ...e.variants].some((x) =>
            normalizeLabelLike(x).includes(q),
          )),
    )
    .sort((a, b) => a.term.localeCompare(b.term, 'en'))
    .map((e) => ({
      id: e.id,
      slug: e.slug,
      family_code: e.family_code,
      term: e.term,
      acronym: e.acronym,
      translation: resolveI18nText(e.translation, 'fr'),
      status: e.status,
      first_published_at: e.first_published_at,
      updated_at: e.updated_at,
    }))
  return { entries, can_publish: true, can_review: true }
}

export function ficheTerme(id: Uuid): AdminGlossaryEntry {
  return vueTerme(termeOuRefus(id))
}

function appliquerTerme(e: Terme, entree: AdminGlossaryInput): void {
  if (entree.family_code !== undefined) {
    if (!entree.family_code) throw invalide('La famille est requise.', 'family_code')
    e.family_code = entree.family_code
  }
  if (entree.term !== undefined) {
    if (!entree.term.trim()) throw invalide('Le terme est requis.', 'term')
    e.term = entree.term.trim()
  }
  if (entree.acronym !== undefined) e.acronym = entree.acronym?.trim() || null
  if (entree.variants !== undefined) e.variants = entree.variants.map((v) => v.trim()).filter(Boolean)
  if (entree.translation !== undefined) e.translation = texteFrancais(entree.translation, 'translation')
  if (entree.definition !== undefined) e.definition = texteFrancais(entree.definition, 'definition')
  if (entree.heard_in_room !== undefined) e.heard_in_room = entree.heard_in_room?.trim() || null
  if (entree.sources !== undefined) e.sources = sourcesValides(entree.sources)
  if (entree.related_ids !== undefined) {
    e.related_ids = lieesValides(e.id, entree.related_ids, (id) => lexique.some((x) => x.id === id))
  }
  e.updated_at = maintenant()
}

export function creerTerme(entree: AdminGlossaryInput): AdminGlossaryEntry {
  if (!entree.family_code) throw invalide('La famille est requise.', 'family_code')
  if (!entree.term?.trim()) throw invalide('Le terme est requis.', 'term')
  if (!entree.translation) throw invalide('La traduction est requise.', 'translation')
  if (!entree.definition) throw invalide('La définition est requise.', 'definition')
  const slug = slugDe(entree.term)
  if (lexique.some((x) => x.slug === slug)) {
    throw refus('NEGOTIATION_GLOSSARY_SLUG_TAKEN', 409, 'Un terme du lexique porte déjà cette désignation.', 'term')
  }
  const e: Terme = {
    id: crypto.randomUUID(),
    slug,
    family_code: entree.family_code,
    term: '',
    acronym: null,
    variants: [],
    translation: fr(''),
    definition: fr(''),
    heard_in_room: null,
    status: 'draft',
    sources: [],
    related_ids: [],
    first_published_at: null,
    created_at: maintenant(),
    updated_at: maintenant(),
  }
  appliquerTerme(e, entree)
  lexique = [...lexique, e]
  return vueTerme(e)
}

export function modifierTerme(id: Uuid, entree: AdminGlossaryInput): AdminGlossaryEntry {
  const e = termeOuRefus(id)
  appliquerTerme(e, entree)
  return vueTerme(e)
}

export function changerTerme(id: Uuid, changement: Changement): AdminGlossaryEntry {
  const e = termeOuRefus(id)
  transition(e, changement)
  return vueTerme(e)
}

export function supprimerTerme(id: Uuid): void {
  const e = termeOuRefus(id)
  if (e.first_published_at) throw indelebile()
  lexique = lexique
    .filter((x) => x.id !== id)
    .map((x) => ({ ...x, related_ids: x.related_ids.filter((r) => r !== id) }))
}

// ---------------------------------------------------------------------------
// Parcours
// ---------------------------------------------------------------------------

function cibleDuLien(link: Etape['link']): string | null {
  if (!link) return null
  if (link.kind === 'document') return titreDuDocument(link.target_id)
  if (link.kind === 'faq') return refFaq(link.target_id)[0]?.label ?? null
  return lexique.find((x) => x.id === link.target_id)?.term ?? null
}

export function parcours(): AdminPathway {
  return {
    groups: groupes.map((g, gi) => ({
      id: g.id,
      label: { ...g.label },
      sort_order: (gi + 1) * 10,
      is_published: g.is_published,
      steps: g.steps.map((s, si) => ({
        ...structuredClone(s),
        group_id: g.id,
        sort_order: (si + 1) * 10,
        link: s.link ? { ...structuredClone(s.link), target_label: cibleDuLien(s.link) } : null,
      })),
    })),
    can_publish: true,
  }
}

function groupeOuRefus(id: Uuid): Groupe {
  const g = groupes.find((x) => x.id === id)
  if (!g) throw refus('NOT_FOUND', 404, "Ce groupe n'existe pas.")
  return g
}

function etapeOuRefus(id: Uuid): { groupe: Groupe; etape: Etape } {
  for (const groupe of groupes) {
    const etape = groupe.steps.find((s) => s.id === id)
    if (etape) return { groupe, etape }
  }
  throw refus('NEGOTIATION_PATHWAY_STEP_NOT_FOUND', 404, "Cette étape n'existe pas.")
}

export function creerGroupe(entree: AdminPathwayGroupInput): AdminPathway {
  groupes = [
    ...groupes,
    {
      id: crypto.randomUUID(),
      label: texteFrancais(entree.label, 'label'),
      is_published: entree.is_published ?? false,
      steps: [],
    },
  ]
  return parcours()
}

export function modifierGroupe(id: Uuid, entree: AdminPathwayGroupInput): AdminPathway {
  const g = groupeOuRefus(id)
  if (entree.label !== undefined) g.label = texteFrancais(entree.label, 'label')
  if (entree.is_published !== undefined) g.is_published = entree.is_published
  return parcours()
}

export function supprimerGroupe(id: Uuid): void {
  const g = groupeOuRefus(id)
  if (g.steps.length > 0) {
    throw refus(
      'NEGOTIATION_PATHWAY_GROUP_NOT_EMPTY',
      409,
      'Ce groupe porte encore des étapes : déplacez-les avant de le supprimer.',
    )
  }
  groupes = groupes.filter((x) => x.id !== id)
}

function lienValide(entree: AdminPathwayStepInput['link']): Etape['link'] {
  if (!entree) return null
  const existe =
    entree.kind === 'document'
      ? titreDuDocument(entree.target_id) !== null
      : entree.kind === 'faq'
        ? faq.some((x) => x.id === entree.target_id)
        : lexique.some((x) => x.id === entree.target_id)
  const pageOuSection = entree.page != null || Boolean(entree.section)
  if (!existe || (entree.kind !== 'document' && pageOuSection)) {
    throw refus('NEGOTIATION_PATHWAY_LINK_INVALID', 422, "Le lien de l'étape ne correspond pas à sa cible.", 'link')
  }
  return {
    kind: entree.kind,
    target_id: entree.target_id,
    page: entree.page ?? null,
    section: entree.section ?? null,
    label: entree.label ?? null,
  }
}

function appliquerEtape(s: Etape, entree: AdminPathwayStepInput): void {
  if (entree.label !== undefined) s.label = texteFrancais(entree.label, 'label')
  if (entree.detail !== undefined) s.detail = entree.detail
  if (entree.origin_label !== undefined) s.origin_label = entree.origin_label
  if (entree.link !== undefined) s.link = lienValide(entree.link)
  if (entree.is_published !== undefined) s.is_published = entree.is_published
}

export function creerEtape(entree: AdminPathwayStepInput): AdminPathway {
  if (!entree.group_id) throw invalide('Le groupe est requis.', 'group_id')
  const g = groupeOuRefus(entree.group_id)
  const s: Etape = {
    id: crypto.randomUUID(),
    label: texteFrancais(entree.label, 'label'),
    detail: null,
    origin_label: null,
    link: null,
    is_published: false,
    checks: 0,
  }
  appliquerEtape(s, entree)
  g.steps = [...g.steps, s]
  return parcours()
}

export function modifierEtape(id: Uuid, entree: AdminPathwayStepInput): AdminPathway {
  const { groupe, etape } = etapeOuRefus(id)
  appliquerEtape(etape, entree)
  if (entree.group_id && entree.group_id !== groupe.id) {
    const cible = groupeOuRefus(entree.group_id)
    groupe.steps = groupe.steps.filter((s) => s.id !== id)
    cible.steps = [...cible.steps, etape]
  }
  return parcours()
}

export function supprimerEtape(id: Uuid): void {
  const { groupe, etape } = etapeOuRefus(id)
  if (etape.checks > 0) {
    throw refus(
      'NEGOTIATION_KNOWLEDGE_PUBLISHED_UNDELETABLE',
      409,
      'Des comptes ont déjà coché cette étape : dépubliez-la plutôt que de la supprimer.',
    )
  }
  groupe.steps = groupe.steps.filter((s) => s.id !== id)
}

export function ordonner(entree: AdminPathwayOrderInput): AdminPathway {
  const etapes = new Map(groupes.flatMap((g) => g.steps.map((s) => [s.id, s] as const)))
  const annonces = entree.groups.flatMap((g) => g.step_ids)
  const complet =
    entree.groups.length === groupes.length &&
    entree.groups.every((g) => groupes.some((x) => x.id === g.id)) &&
    annonces.length === etapes.size &&
    new Set(annonces).size === etapes.size &&
    annonces.every((id) => etapes.has(id))
  if (!complet) throw invalide("L'ordre doit nommer chaque groupe et chaque étape une fois.", 'groups')
  groupes = entree.groups.map((g) => ({
    ...groupeOuRefus(g.id),
    steps: g.step_ids.flatMap((id) => etapes.get(id) ?? []),
  }))
  return parcours()
}
