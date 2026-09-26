/**
 * LE BACK-OFFICE DU SAVOIR DE GUIDE NÉGO — FAQ, lexique, parcours — : sa part de `useApi()`.
 *
 * Même règle que les documents : **portée globale, sans périmètre d'édition**.
 * Deux permissions se partagent ces routes : `negotiation.knowledge.publish`
 * rédige et publie ; `negotiation.knowledge.review` date la vérification d'une
 * réponse. Les lectures s'ouvrent à l'une ou l'autre ; les réponses portent
 * `can_publish` et `can_review`, qui montrent les gestes sans décider à la place
 * de l'API.
 */
import type {
  AdminFaqEntry,
  AdminFaqInput,
  AdminFaqList,
  AdminFaqReport,
  AdminFaqVerifyInput,
  AdminGlossaryEntry,
  AdminGlossaryInput,
  AdminGlossaryList,
  AdminKnowledgeStatus,
  AdminPathway,
  AdminPathwayGroupInput,
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
} from '~/types/admin-negotiation-queue'
import type { Uuid } from '~/types/shared'
import type { ApiTransport } from './proposal-review'

type Deps = Pick<ApiTransport, 'call' | 'send'>

export interface FiltreFaq {
  q?: string
  section?: string
  status?: AdminKnowledgeStatus
}

export interface FiltreLexique {
  q?: string
  family?: string
  status?: AdminKnowledgeStatus
}

const exemples = () => import('~/mocks/admin-negotiation-savoir')

/** Les filtres vides ne partent pas : `?q=` vide filtrerait sur rien. */
const requete = (filtre: object): Record<string, unknown> =>
  Object.fromEntries(Object.entries(filtre).filter(([, v]) => v !== undefined && v !== ''))

export function createAdminNegotiationSavoirApi({ call, send }: Deps) {
  return {
    // --- FAQ -------------------------------------------------------------
    faq: (filtre: FiltreFaq = {}): Promise<AdminFaqList> =>
      call('/admin/negotiation/faq', async () => (await exemples()).listeFaq(filtre), requete(filtre)),

    entreeFaq: (id: Uuid): Promise<AdminFaqEntry> =>
      call(`/admin/negotiation/faq/${id}`, async () => (await exemples()).ficheFaq(id)),

    /** Naît en brouillon. */
    creerFaq: (entree: AdminFaqInput): Promise<AdminFaqEntry> =>
      send('/admin/negotiation/faq', entree, async () => (await exemples()).creerFaq(entree)),

    /** Sources et liées se remplacent en bloc. */
    modifierFaq: (id: Uuid, entree: AdminFaqInput): Promise<AdminFaqEntry> =>
      send(`/admin/negotiation/faq/${id}`, entree, async () => (await exemples()).modifierFaq(id, entree), 'PATCH'),

    /** L'expert seul. Une entrée « À revoir » revient publiée. */
    verifierFaq: (id: Uuid, entree: AdminFaqVerifyInput = {}): Promise<AdminFaqEntry> =>
      send(`/admin/negotiation/faq/${id}/verify`, entree, async () => (await exemples()).verifierFaq(id, entree)),

    /** Sans vérification datée : `NEGOTIATION_FAQ_UNVERIFIED`. */
    publierFaq: (id: Uuid): Promise<AdminFaqEntry> =>
      send(`/admin/negotiation/faq/${id}/publish`, {}, async () => (await exemples()).changerFaq(id, 'publish')),

    aRevoirFaq: (id: Uuid): Promise<AdminFaqEntry> =>
      send(`/admin/negotiation/faq/${id}/to-review`, {}, async () => (await exemples()).changerFaq(id, 'to-review')),

    depublierFaq: (id: Uuid): Promise<AdminFaqEntry> =>
      send(`/admin/negotiation/faq/${id}/unpublish`, {}, async () => (await exemples()).changerFaq(id, 'unpublish')),

    /** Un brouillon jamais publié seulement. */
    supprimerFaq: (id: Uuid): Promise<void> =>
      send(`/admin/negotiation/faq/${id}`, {}, async () => (await exemples()).supprimerFaq(id), 'DELETE'),

    // --- Lexique ---------------------------------------------------------
    lexique: (filtre: FiltreLexique = {}): Promise<AdminGlossaryList> =>
      call('/admin/negotiation/glossary', async () => (await exemples()).listeLexique(filtre), requete(filtre)),

    terme: (id: Uuid): Promise<AdminGlossaryEntry> =>
      call(`/admin/negotiation/glossary/${id}`, async () => (await exemples()).ficheTerme(id)),

    creerTerme: (entree: AdminGlossaryInput): Promise<AdminGlossaryEntry> =>
      send('/admin/negotiation/glossary', entree, async () => (await exemples()).creerTerme(entree)),

    modifierTerme: (id: Uuid, entree: AdminGlossaryInput): Promise<AdminGlossaryEntry> =>
      send(
        `/admin/negotiation/glossary/${id}`,
        entree,
        async () => (await exemples()).modifierTerme(id, entree),
        'PATCH',
      ),

    publierTerme: (id: Uuid): Promise<AdminGlossaryEntry> =>
      send(`/admin/negotiation/glossary/${id}/publish`, {}, async () => (await exemples()).changerTerme(id, 'publish')),

    aRevoirTerme: (id: Uuid): Promise<AdminGlossaryEntry> =>
      send(`/admin/negotiation/glossary/${id}/to-review`, {}, async () => (await exemples()).changerTerme(id, 'to-review')),

    depublierTerme: (id: Uuid): Promise<AdminGlossaryEntry> =>
      send(`/admin/negotiation/glossary/${id}/unpublish`, {}, async () => (await exemples()).changerTerme(id, 'unpublish')),

    supprimerTerme: (id: Uuid): Promise<void> =>
      send(`/admin/negotiation/glossary/${id}`, {}, async () => (await exemples()).supprimerTerme(id), 'DELETE'),

    // --- Parcours --------------------------------------------------------
    parcours: (): Promise<AdminPathway> =>
      call('/admin/negotiation/pathway', async () => (await exemples()).parcours()),

    creerGroupe: (entree: AdminPathwayGroupInput): Promise<AdminPathway> =>
      send('/admin/negotiation/pathway/groups', entree, async () => (await exemples()).creerGroupe(entree)),

    modifierGroupe: (id: Uuid, entree: AdminPathwayGroupInput): Promise<AdminPathway> =>
      send(
        `/admin/negotiation/pathway/groups/${id}`,
        entree,
        async () => (await exemples()).modifierGroupe(id, entree),
        'PATCH',
      ),

    /** Refusé tant qu'il porte des étapes. */
    supprimerGroupe: (id: Uuid): Promise<void> =>
      send(
        `/admin/negotiation/pathway/groups/${id}`,
        {},
        async () => (await exemples()).supprimerGroupe(id),
        'DELETE',
      ),

    creerEtape: (entree: AdminPathwayStepInput): Promise<AdminPathway> =>
      send('/admin/negotiation/pathway/steps', entree, async () => (await exemples()).creerEtape(entree)),

    modifierEtape: (id: Uuid, entree: AdminPathwayStepInput): Promise<AdminPathway> =>
      send(
        `/admin/negotiation/pathway/steps/${id}`,
        entree,
        async () => (await exemples()).modifierEtape(id, entree),
        'PATCH',
      ),

    /** Refusé pour une étape déjà cochée : elle se dépublie. */
    supprimerEtape: (id: Uuid): Promise<void> =>
      send(
        `/admin/negotiation/pathway/steps/${id}`,
        {},
        async () => (await exemples()).supprimerEtape(id),
        'DELETE',
      ),

    ordonnerParcours: (entree: AdminPathwayOrderInput): Promise<AdminPathway> =>
      send('/admin/negotiation/pathway/order', entree, async () => (await exemples()).ordonner(entree), 'PUT'),

    // --- File des experts ------------------------------------------------
    /** L'expert seul : `negotiation.knowledge.review`. */
    fileDesExperts: (kind: ExpertQueueKind = 'reports'): Promise<ExpertQueue> =>
      call('/admin/negotiation/queue', async () => (await exemples()).fileDesExperts(kind), { kind }),

    /** Ne modifie jamais l'entrée. Déjà clos : `NEGOTIATION_QUEUE_ITEM_CLOSED`. */
    cloreUnSignalement: (id: Uuid, entree: AdminFaqReportCloseInput): Promise<AdminFaqReport> =>
      send(
        `/admin/negotiation/queue/reports/${id}/close`,
        entree,
        async () => (await exemples()).cloreUnSignalement(id, entree),
      ),

    /** Met en file le courriel de l'auteure. Déjà répondue : `NEGOTIATION_QUEUE_ITEM_CLOSED`. */
    repondreAUneQuestion: (id: Uuid, entree: AdminQuestionAnswerInput): Promise<AdminQuestion> =>
      send(
        `/admin/negotiation/queue/questions/${id}/answer`,
        entree,
        async () => (await exemples()).repondreAUneQuestion(id, entree),
      ),

    /** Brouillon de FAQ sans auteur. Sans consentement : `NEGOTIATION_QUESTION_NO_CONSENT`. */
    promouvoirUneQuestion: (id: Uuid, entree: AdminQuestionPromoteInput): Promise<AdminFaqEntry> =>
      send(
        `/admin/negotiation/queue/questions/${id}/promote`,
        entree,
        async () => (await exemples()).promouvoirUneQuestion(id, entree),
      ),
  }
}
