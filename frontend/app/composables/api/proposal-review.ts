/**
 * LA FICHE D'ÉVALUATION (A8) — sa part de `useApi()`, déclarée à part.
 *
 * POURQUOI CE FICHIER EXISTE, ET CE QU'IL NE CHANGE PAS. La règle du projet est
 * inchangée : aucune page n'importe un mock, aucune page n'appelle `$fetch` ;
 * tout passe par `useApi()`, et c'est bien lui que la page appelle
 * (`api.review.desk(…)`). Ce qui change est la place du CODE, pas le chemin des
 * données — `useApi.ts` atteignait 900 lignes et la fiche d'évaluation en ajoute
 * cent-cinquante, ce qui l'aurait porté au-delà du garde-fou de mille lignes de
 * `CLAUDE.md`. Le découpage suit donc la règle du projet : par ÉCRAN.
 *
 * La fabrique reçoit les primitives de `useApi()` — les mêmes fonctions que le
 * reste du composable —, si bien que la bascule vers l'API réelle, la latence
 * simulée et l'en-tête `Accept-Language` valent ici comme ailleurs, sans être
 * redéclarés.
 *
 * `personId` NE PART JAMAIS À L'API : elle tient l'acteur de la session, et un
 * acteur déclaré par le client ne serait pas cru. Le paramètre reste parce que
 * le mode hors ligne n'a pas de session — les données simulées ont besoin qu'on
 * leur dise qui lit et qui écrit.
 */

import type {
  DecisionPayload,
  DecisionResult,
  PostCommentPayload,
  RecusalPayload,
  ReviewDeskScreen,
  SaveReviewPayload,
  SaveReviewResult,
} from '~/types/admin-review'
import type { ProposalComment } from '~/types/programme/proposal'
import type { ReviewAssignment } from '~/types/programme/review'
import type { Uuid } from '~/types/shared'

type Mocks = typeof import('~/mocks')

/**
 * Les primitives de `useApi()` : deux lectures, une écriture, et l'attente.
 *
 * `callOrNull` est la lecture dont « rien » est une réponse — le 404 devient
 * `null` au lieu de faire basculer l'écran en erreur.
 *
 * `pending` sert les trois écrans dont l'API n'existe pas encore : rien ne part
 * sur le réseau, les données simulées répondent même API configurée, et un
 * bandeau le dit. Il est déclaré ICI et non dans chaque fabrique concernée —
 * trois déclarations de la même signature finissent par diverger.
 */
export interface ApiTransport {
  call: <T>(path: string, fromMocks: (m: Mocks) => T | Promise<T>, query?: Record<string, unknown>) => Promise<T>
  callOrNull: <T>(
    path: string,
    fromMocks: (m: Mocks) => T | null | Promise<T | null>,
    query?: Record<string, unknown>,
  ) => Promise<T | null>
  send: <T>(
    path: string,
    body: object,
    fromMocks: (m: Mocks) => T | Promise<T>,
    method?: 'POST' | 'PUT' | 'PATCH' | 'DELETE',
  ) => Promise<T>
  pending: <T>(
    path: string,
    fromMocks: (m: Mocks) => T | Promise<T>,
    kind?: 'read' | 'write',
  ) => Promise<T>
}

export function createProposalReviewApi({ callOrNull, send }: ApiTransport) {
  return {
    /**
     * TOUT L'ÉCRAN EN UNE RÉPONSE — dossier, grille, comité, échanges, droits.
     *
     * LA RÉPONSE DÉPEND DU LECTEUR, et c'est le cœur de cet écran : le voile de
     * l'évaluation en aveugle (`calls_for_proposals.blind_review`) retire les
     * revues des pairs de la RÉPONSE tant que la personne n'a pas soumis la
     * sienne, et les échanges sont filtrés selon les trois visibilités du modèle.
     * Rien de tout cela n'est laissé à un composant : ce qui n'est pas envoyé ne
     * peut pas fuiter.
     *
     * L'OUVERTURE POSE UN ACCUSÉ DE LECTURE — `programme.record_proposal_read()`
     * —, ce qui alimente le « lu par 3 membres du comité » de la liste. C'est
     * une lecture qui écrit : assumé, et c'est déjà ce que fait la fonction en
     * base.
     *
     * DOSSIER INEXISTANT ET DOSSIER HORS PÉRIMÈTRE SONT INDISCERNABLES — l'API
     * répond 404 aux deux, délibérément, pour ne pas laisser deviner qu'une
     * édition existe ailleurs. `callOrNull` en fait le `null` que l'écran rend
     * en « dossier introuvable ». Le refus qui NOMME sa raison, lui, remonte :
     * `ForbiddenError` déclenche l'écran d'accès refusé.
     */
    desk: (proposalId: Uuid, personId: Uuid | null): Promise<ReviewDeskScreen | null> =>
      callOrNull(`/proposals/${proposalId}/review-desk`, (m) => m.reviewDesk(proposalId, personId)),

    /**
     * ENREGISTRER OU DÉPOSER SA REVUE.
     *
     * Un seul appel pour les deux gestes (`submit`), parce que la base n'en fait
     * qu'un : une revue est une ligne de `programme.reviews` dont `submitted_at`
     * est nul tant qu'elle est en brouillon. La réponse rend les agrégats
     * recalculés par `refresh_proposal_score()` — moyenne, rang, élimination —
     * pour que l'en-tête change sans recharger la page.
     *
     * `proposal_id` COMPOSE L'URL et ne part pas dans le corps : le dossier est
     * déjà nommé par le chemin, et deux endroits pour une même donnée finissent
     * par se contredire.
     */
    save: (personId: Uuid, payload: SaveReviewPayload): Promise<SaveReviewResult> => {
      const { proposal_id, ...body } = payload
      return send(`/proposals/${proposal_id}/reviews`, body, (m) => m.saveReview(personId, payload), 'PUT')
    },

    /**
     * SE DÉPORTER, EN DÉCLARANT SON LIEN AVEC L'ORGANISATION.
     *
     * Le motif n'est pas facultatif : `review_assignments.recusal_reason` existe
     * pour que l'impartialité du comité se relise, et un déport sans motif ne
     * prouve rien.
     */
    recuse: (personId: Uuid, payload: RecusalPayload): Promise<ReviewAssignment> => {
      const { proposal_id, ...body } = payload
      return send(`/proposals/${proposal_id}/recusal`, body, (m) =>
        m.recuseFromProposal(personId, payload),
      )
    },

    /**
     * ÉCRIRE SUR LE DOSSIER — avec sa visibilité.
     *
     * `visibility` voyage dans la charge utile parce que c'est un ÉTAT du modèle
     * (`programme.comment_visibility`) et non un réglage d'affichage. Une demande
     * de correction est forcée en `submitter` à la source : une demande que le
     * déposant ne verrait pas bloquerait son dossier sans qu'il sache pourquoi.
     */
    comment: (personId: Uuid, payload: PostCommentPayload): Promise<ProposalComment> => {
      const { proposal_id, ...body } = payload
      return send(`/proposals/${proposal_id}/comments`, body, (m) =>
        m.postProposalComment(personId, payload),
      )
    },

    /**
     * DÉCIDER — retenir, demander des corrections, rejeter.
     *
     * Les refus de la machine à états (`proposal_transitions_allowed`) sont des
     * RÉPONSES, pas des erreurs de réseau : transition impossible depuis cet
     * état, ou motif manquant. L'écran les rend comme telles.
     */
    decide: (personId: Uuid | null, payload: DecisionPayload): Promise<DecisionResult> => {
      const { proposal_id, ...body } = payload
      return send(`/proposals/${proposal_id}/decision`, body, (m) =>
        m.decideProposal(personId, payload),
      )
    },
  }
}
