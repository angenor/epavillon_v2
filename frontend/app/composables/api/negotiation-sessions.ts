/**
 * Les sessions de négociation officielles, « Mon groupe » et « Mon agenda » —
 * leur part de `useApi()`. Contrat : `specs/014-guide-nego-sessions-agenda/contracts/api-sessions.md`.
 *
 * Les lectures portent une empreinte : la liste de la COP (~150 Ko) se relit en
 * `304` quand rien n'a changé, et les groupes renvoient la leur en `If-Match`.
 */
import type { MyAgenda, MyGroups, OfficialSessions } from '~/types/negotiation-sessions'
import type { Uuid } from '~/types/shared'
import type { Primitives } from './guide-nego'

type Deps = Pick<Primitives, 'send' | 'lireEtiquete' | 'ecrireEtiquete'>

const exemples = () => import('~/mocks/negotiation-sessions')

// Le chemin reste un littéral à part : `check-api-contract` le compare au contrat.
const edition = (slug: string) => `?edition=${encodeURIComponent(slug)}`

export function createNegotiationSessionsApi({ send, lireEtiquete, ecrireEtiquete }: Deps) {
  return {
    /**
     * Toute la COP en une réponse. Coupée, `sessions` est vide ; édition
     * inconnue : `404 NEGOTIATION_EDITION_UNKNOWN`.
     */
    sessions: (slug: string, siDifferent: string | null) =>
      lireEtiquete<OfficialSessions>(
        '/negotiation/sessions' + edition(slug),
        async () => (await exemples()).sessionsOfficielles(),
        siDifferent,
      ),

    mesGroupes: () => lireEtiquete<MyGroups>('/negotiation/me/groups', async () => (await exemples()).mesGroupes()),

    /** Remplace la liste entière ; périmée en `If-Match` : `412 NEGOTIATION_GROUPS_STALE`. */
    suivreDesGroupes: (groups: string[], empreinte: string | null) =>
      ecrireEtiquete<MyGroups>(
        '/negotiation/me/groups',
        { groups },
        async () => (await exemples()).suivreDesGroupes(groups),
        empreinte,
      ),

    monAgenda: (siDifferent: string | null) =>
      lireEtiquete<MyAgenda>('/negotiation/me/agenda', async () => (await exemples()).monAgenda(), siDifferent),

    /** Idempotent : ajoute, ou change le rappel. Annulée et absente : `409 NEGOTIATION_SESSION_CANCELLED`. */
    garderUneSession: (sessionId: Uuid, remind: boolean): Promise<void> =>
      send(
        `/negotiation/me/agenda/${sessionId}`,
        { remind },
        async () => (await exemples()).garderUneSession(sessionId, remind),
        'PUT',
      ),

    /** Idempotent : `204` même si la ligne n'existait pas. */
    retirerUneSession: (sessionId: Uuid): Promise<void> =>
      send(
        `/negotiation/me/agenda/${sessionId}`,
        {},
        async () => (await exemples()).retirerUneSession(sessionId),
        'DELETE',
      ),

    /** Une réunion non annoncée. Idempotent ; retirée ou non publiée : `404 NEGOTIATION_SESSION_UNKNOWN`. */
    garderUneReunion: (id: Uuid, remind: boolean): Promise<void> =>
      send(
        `/negotiation/me/agenda/network/${id}`,
        { remind },
        async () => (await exemples()).garderUneReunion(id, remind),
        'PUT',
      ),

    retirerUneReunion: (id: Uuid): Promise<void> =>
      send(
        `/negotiation/me/agenda/network/${id}`,
        {},
        async () => (await exemples()).retirerUneReunion(id),
        'DELETE',
      ),
  }
}
