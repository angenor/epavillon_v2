/**
 * Le Pavillon de la Francophonie dans Guide Négo — sa part de `useApi()`.
 * Contrat : `specs/017-guide-nego-pavillon/contracts/api-programme.md`.
 *
 * Les routes sont celles du site, employées telles quelles ; seules celles qui
 * n'avaient pas encore d'appel côté client vivent ici. Les lieux et le formulaire
 * se lisent par `events.venues` et `registrations.form`, déjà montés.
 */
import type {
  AskQuestionPayload,
  CancelRegistrationResult,
  PublicSessionQuestion,
  Registration,
  RegistrationResult,
  SessionRegisterPayload,
} from '~/types/programme/registration'
import type { PublicScheduleRow, PublicSessionDetail } from '~/types/views'
import type { Uuid } from '~/types/shared'
import type { AppelsEtiquetes } from './etiquete'
import type { Primitives } from './guide-nego'

type Mocks = typeof import('~/mocks')

interface Deps extends Pick<Primitives, 'send'> {
  lireEtiquete: AppelsEtiquetes['lireEtiquete']
  callOrNull: <T>(path: string, fromMocks: (m: Mocks) => T | null | Promise<T | null>) => Promise<T | null>
}

const exemples = () => import('~/mocks/pavillon')

export function createPavillonApi({ callOrNull, send, lireEtiquete }: Deps) {
  return {
    /** Toute l'édition, sans plafond ; un programme non paru rend une liste vide. */
    edition: (eventId: Uuid, siDifferent: string | null) =>
      lireEtiquete<PublicScheduleRow[]>(
        `/schedule?event_id=${encodeURIComponent(eventId)}`,
        async () => (await exemples()).programme(eventId),
        siDifferent,
      ),

    /** Nulle : adresse inconnue ou séance non publiée, indiscernables. */
    activite: (eventId: Uuid, slug: string) =>
      callOrNull<PublicSessionDetail>(
        `/events/${eventId}/sessions/${encodeURIComponent(slug)}`,
        async () => (await exemples()).activite(eventId, slug),
      ),

    /** Six issues en 200 ; `REGISTRATION_NOT_ACCEPTED` en 422 : l'activité ne prend pas d'inscription. */
    inscrire: (sessionId: Uuid, payload: SessionRegisterPayload): Promise<RegistrationResult> =>
      send(`/sessions/${sessionId}/registrations`, payload, async () => (await exemples()).inscrire(sessionId, payload)),

    /** `404` : déjà partie ; `REGISTRATION_LOCKED` (422) : la base refuse. */
    annuler: (registrationId: Uuid): Promise<CancelRegistrationResult> =>
      send(`/registrations/${registrationId}/cancel`, {}, async () => (await exemples()).annuler(registrationId)),

    /** Visibles seulement, les plus soutenues d'abord ; `404` : séance inconnue ou non publiée. */
    questions: (sessionId: Uuid, siDifferent: string | null = null) =>
      lireEtiquete<PublicSessionQuestion[]>(
        `/sessions/${sessionId}/questions`,
        async () => (await exemples()).questions(sessionId),
        siDifferent,
      ),

    /** `409` : la séance ne prend pas de questions ; `422` : hors de 3 à 2000 caractères. */
    poserQuestion: (sessionId: Uuid, payload: AskQuestionPayload): Promise<PublicSessionQuestion> =>
      send(`/sessions/${sessionId}/questions`, payload, async () => (await exemples()).poserQuestion(sessionId, payload)),

    /** `409` : déjà soutenue, ou séance fermée aux questions. */
    voter: (sessionId: Uuid, questionId: Uuid): Promise<PublicSessionQuestion> =>
      send(`/sessions/${sessionId}/questions/${questionId}/vote`, {}, async () => (await exemples()).voter(sessionId, questionId)),

    /** Sans effet si la personne ne la soutenait pas. */
    retirerVote: (sessionId: Uuid, questionId: Uuid): Promise<PublicSessionQuestion> =>
      send(
        `/sessions/${sessionId}/questions/${questionId}/vote`,
        {},
        async () => (await exemples()).retirerVote(sessionId, questionId),
        'DELETE',
      ),

    /** Annulations comprises ; propre à la personne, jamais mise en cache partagé. */
    mesInscriptions: (siDifferent: string | null) =>
      lireEtiquete<Registration[]>(
        '/registrations/mine',
        async () => (await exemples()).mesInscriptions(),
        siDifferent,
      ),
  }
}
