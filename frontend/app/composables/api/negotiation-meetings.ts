/**
 * Les réunions de la Francophonie et l'inscription — leur part de `useApi()`.
 * Contrat : `specs/016-guide-nego-reunions/contracts/api-reunions.md`.
 *
 * Les deux lectures portent une empreinte ; celle des inscriptions est propre à la
 * personne et sa réponse n'est jamais mise en cache (elle porte les liens de visio).
 */
import type {
  FrancophoneMeetings,
  MeetingRegistrationPayload,
  MeetingRegistrationState,
  MyMeetingRegistrations,
} from '~/types/negotiation-meetings'
import type { Uuid } from '~/types/shared'
import type { Primitives } from './guide-nego'

type Deps = Pick<Primitives, 'send' | 'lireEtiquete'>

const exemples = () => import('~/mocks/negotiation-meetings')

const edition = (slug: string) => `?edition=${encodeURIComponent(slug)}`

export function createNegotiationMeetingsApi({ send, lireEtiquete }: Deps) {
  return {
    /** Publiques ; édition inconnue : `404 NEGOTIATION_EDITION_UNKNOWN`. */
    reunions: (slug: string, siDifferent: string | null) =>
      lireEtiquete<FrancophoneMeetings>(
        '/negotiation/meetings' + edition(slug),
        async () => (await exemples()).reunions(slug),
        siDifferent,
      ),

    mesInscriptions: (slug: string, siDifferent: string | null) =>
      lireEtiquete<MyMeetingRegistrations>(
        '/negotiation/me/meeting-registrations' + edition(slug),
        async () => (await exemples()).mesInscriptions(),
        siDifferent,
      ),

    /**
     * Un `client_ref` neuf par geste : rejoué, l'envoi rend l'état courant. Refus en `409` :
     * `NEGOTIATION_MEETING_FULL`, `_CLOSED`, `_UNAVAILABLE`.
     */
    inscrire: (id: Uuid, client_ref: Uuid): Promise<MeetingRegistrationState> =>
      send(
        `/negotiation/me/meeting-registrations/${id}`,
        { client_ref } satisfies MeetingRegistrationPayload,
        async () => (await exemples()).inscrire(id, client_ref),
        'PUT',
      ),

    /** Idempotent ; la première en attente prend la place. */
    desinscrire: (id: Uuid): Promise<void> =>
      send(
        `/negotiation/me/meeting-registrations/${id}`,
        {},
        async () => (await exemples()).desinscrire(id),
        'DELETE',
      ),
  }
}
