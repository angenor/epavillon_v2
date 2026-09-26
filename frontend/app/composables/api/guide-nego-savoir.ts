/**
 * Le savoir de Guide Négo — FAQ, parcours, lexique — : sa part de `useApi()`.
 *
 * Un seul paquet, public, relu par différence : avec le `served_at` de la lecture
 * gardée, l'API ne rend que ce qui a changé ; avec son empreinte, un `304` si rien
 * n'a bougé. Termes favoris, coches du parcours, retours et signalements demandent une session ;
 * les questions aux experts, l'accès négociateur.
 */
import type {
  FaqFeedback,
  FaqFeedbackInput,
  FaqReportInput,
  FaqReportReceipt,
  KnowledgeBundle,
  MyFaqFeedback,
  MyGlossaryFavorites,
  MyPathway,
  MyQuestion,
  MyQuestionInput,
  MyQuestionList,
} from '~/types/negotiation-savoir'
import type { IsoDateTime, Uuid } from '~/types/shared'
import type { Primitives } from './guide-nego'
import type { AvecEmpreinte } from './http'

type Deps = Pick<Primitives, 'lireEtiquete' | 'send'>

export interface DepuisLaGarde {
  since: IsoDateTime
  empreinte: string | null
}

const exemples = () => import('~/mocks/negotiation-savoir')
const questions = () => import('~/mocks/negotiation-questions')

const parametres = (depuis: DepuisLaGarde | null): string =>
  depuis ? `?since=${encodeURIComponent(depuis.since)}` : ''

export function createGuideNegoSavoirApi({ lireEtiquete, send }: Deps) {
  const { $i18n } = useNuxtApp()
  const langue = (): string => String($i18n.locale.value)

  return {
    /** Sans garde : tout le publié. Avec : la différence depuis sa lecture, ou `304`. */
    paquet: (depuis: DepuisLaGarde | null) =>
      lireEtiquete<KnowledgeBundle>(
        '/negotiation/knowledge' + parametres(depuis),
        async () => (await exemples()).paquetDuSavoir(langue(), depuis?.since ?? null),
        depuis?.empreinte ?? null,
      ),

    mesTermesFavoris: (): Promise<AvecEmpreinte<MyGlossaryFavorites>> =>
      lireEtiquete('/negotiation/me/glossary-favorites', async () => (await exemples()).mesTermesFavoris()),

    /** Idempotent. Entrée inconnue ou en brouillon : 404. */
    poserUnTermeFavori: (entryId: Uuid): Promise<void> =>
      send(
        `/negotiation/me/glossary-favorites/${entryId}`,
        {},
        async () => (await exemples()).poserUnTermeFavori(entryId),
        'PUT',
      ),

    monParcours: (): Promise<AvecEmpreinte<MyPathway>> =>
      lireEtiquete('/negotiation/me/pathway', async () => (await exemples()).monParcours()),

    /** Idempotent : le dernier geste reçu l'emporte. Étape inconnue ou non publiée : 404. */
    cocherUneEtape: (stepId: Uuid): Promise<void> =>
      send(`/negotiation/me/pathway/${stepId}`, {}, async () => (await exemples()).cocherUneEtape(stepId), 'PUT'),

    /** Idempotent, même sur une étape retirée. */
    decocherUneEtape: (stepId: Uuid): Promise<void> =>
      send(`/negotiation/me/pathway/${stepId}`, {}, async () => (await exemples()).decocherUneEtape(stepId), 'DELETE'),

    /** Compteur des « plus lues », sans auteur ; hors file : une lecture perdue ne coûte rien (R12). */
    lireUneEntreeDeFaq: (entryId: Uuid): Promise<void> =>
      send(`/negotiation/faq/${entryId}/read`, {}, () => undefined),

    /** Idempotent, même si le favori n'existe pas. */
    retirerUnTermeFavori: (entryId: Uuid): Promise<void> =>
      send(
        `/negotiation/me/glossary-favorites/${entryId}`,
        {},
        async () => (await exemples()).retirerUnTermeFavori(entryId),
        'DELETE',
      ),

    /** « Cette réponse vous a-t-elle aidée ? » : la dernière voix compte. */
    voterSurUneEntree: (entryId: Uuid, entree: FaqFeedbackInput): Promise<FaqFeedback> =>
      send(
        `/negotiation/faq/${entryId}/feedback`,
        entree,
        async () => (await exemples()).voterSurUneEntree(entryId, entree),
        'PUT',
      ),

    /** « Dépassé ou faux ». Rejoué avec le même `client_ref` : le même reçu. */
    signalerUneEntree: (entryId: Uuid, entree: FaqReportInput): Promise<FaqReportReceipt> =>
      send(`/negotiation/faq/${entryId}/reports`, entree, async () => (await exemples()).signalerUneEntree(entryId, entree)),

    mesRetoursSurLaFaq: (): Promise<AvecEmpreinte<MyFaqFeedback>> =>
      lireEtiquete('/negotiation/me/faq-feedback', async () => (await exemples()).mesRetoursSurLaFaq()),

    /** Rejouée avec le même `client_ref` : la même question. Sans l'accès : 403. */
    poserUneQuestion: (entree: MyQuestionInput): Promise<MyQuestion> =>
      send('/negotiation/me/questions', entree, async () => (await questions()).poserUneQuestion(entree)),

    mesQuestions: (): Promise<AvecEmpreinte<MyQuestionList>> =>
      lireEtiquete('/negotiation/me/questions', async () => (await questions()).mesQuestions()),
  }
}
