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
 * La fabrique reçoit `call` et `send` — les deux mêmes fonctions que le reste du
 * composable —, si bien que la bascule vers l'API réelle, la latence simulée et
 * l'en-tête `Accept-Language` valent ici comme ailleurs, sans être redéclarés.
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

/** Les deux primitives de `useApi()` : une lecture, une écriture. */
export interface ApiTransport {
  call: <T>(path: string, fromMocks: (m: Mocks) => T | Promise<T>, query?: Record<string, unknown>) => Promise<T>
  send: <T>(
    path: string,
    body: object,
    fromMocks: (m: Mocks) => T | Promise<T>,
    method?: 'POST' | 'PUT' | 'PATCH' | 'DELETE',
  ) => Promise<T>
}

export function createProposalReviewApi({ call, send }: ApiTransport) {
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
     */
    desk: (proposalId: Uuid, personId: Uuid | null): Promise<ReviewDeskScreen | null> =>
      call(`/proposals/${proposalId}/review-desk`, (m) => m.reviewDesk(proposalId, personId)),

    /**
     * ENREGISTRER OU DÉPOSER SA REVUE.
     *
     * Un seul appel pour les deux gestes (`submit`), parce que la base n'en fait
     * qu'un : une revue est une ligne de `programme.reviews` dont `submitted_at`
     * est nul tant qu'elle est en brouillon. La réponse rend les agrégats
     * recalculés par `refresh_proposal_score()` — moyenne, rang, élimination —
     * pour que l'en-tête change sans recharger la page.
     */
    save: (personId: Uuid, payload: SaveReviewPayload): Promise<SaveReviewResult> =>
      send(`/proposals/${payload.proposal_id}/reviews`, payload, (m) => m.saveReview(personId, payload), 'PUT'),

    /**
     * SE DÉPORTER, EN DÉCLARANT SON LIEN AVEC L'ORGANISATION.
     *
     * Le motif n'est pas facultatif : `review_assignments.recusal_reason` existe
     * pour que l'impartialité du comité se relise, et un déport sans motif ne
     * prouve rien.
     */
    recuse: (personId: Uuid, payload: RecusalPayload): Promise<ReviewAssignment> =>
      send(`/proposals/${payload.proposal_id}/recusal`, payload, (m) =>
        m.recuseFromProposal(personId, payload),
      ),

    /**
     * ÉCRIRE SUR LE DOSSIER — avec sa visibilité.
     *
     * `visibility` voyage dans la charge utile parce que c'est un ÉTAT du modèle
     * (`programme.comment_visibility`) et non un réglage d'affichage. Une demande
     * de correction est forcée en `submitter` à la source : une demande que le
     * déposant ne verrait pas bloquerait son dossier sans qu'il sache pourquoi.
     */
    comment: (personId: Uuid, payload: PostCommentPayload): Promise<ProposalComment> =>
      send(`/proposals/${payload.proposal_id}/comments`, payload, (m) =>
        m.postProposalComment(personId, payload),
      ),

    /**
     * DÉCIDER — retenir, demander des corrections, rejeter.
     *
     * Les refus de la machine à états (`proposal_transitions_allowed`) sont des
     * RÉPONSES, pas des erreurs de réseau : transition impossible depuis cet
     * état, ou motif manquant. L'écran les rend comme telles.
     */
    decide: (personId: Uuid | null, payload: DecisionPayload): Promise<DecisionResult> =>
      send(`/proposals/${payload.proposal_id}/decision`, payload, (m) =>
        m.decideProposal(personId, payload),
      ),
  }
}
