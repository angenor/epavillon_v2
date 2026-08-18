/**
 * ESPACE ORGANISATION (A5) — sa part de `useApi()`.
 *
 * TROIS LECTURES ET TROIS ÉCRITURES, et la ligne de partage est toujours la
 * même : l'organisation voit CE QU'ELLE A DÉPOSÉ et ce qu'on attend d'elle,
 * jamais ce que le comité s'écrit ni qui s'est inscrit à ses séances.
 *
 * AUCUNE VUE DU MODÈLE NE RÉPOND ICI. `v_proposal_dashboard` est faite pour le
 * comité — notes, rang, revues manquantes —, et l'espace organisation n'en
 * montrerait rien. Ces compositions appartiendront donc à l'API (prompt B4), pas
 * à une vue SQL supplémentaire.
 *
 * SORTI DE `useApi.ts` AU PROMPT A12, sans qu'une ligne change : ce fichier
 * atteignait mille lignes en montant la fabrique des utilisateurs, et l'espace
 * organisation est un écran entier — donc l'unité de découpage du projet. Les
 * pages continuent d'appeler `api.workspace.overview(…)`.
 */

import type {
  ProposalFile,
  ReplyToCommentPayload,
  ResolveCommentPayload,
  WorkspaceOverview,
} from '~/types/organization-workspace'
import type { Uuid } from '~/types/shared'
import type { ApiTransport } from './proposal-review'

export function createOrganizationWorkspaceApi({ call, send }: ApiTransport) {
  return {
    /**
     * TOUT L'ÉCRAN D'ACCUEIL EN UNE RÉPONSE : l'organisation, l'adhésion de la
     * personne connectée, ses dossiers avec leur avancement, ses membres, ce
     * qui attend une action, et l'appel en cours pour l'état vide.
     *
     * Rend `null` quand la personne n'a pas d'adhésion ACTIVE : l'écran refuse
     * alors l'accès plutôt que d'afficher une page vide, qui laisserait croire
     * à une organisation sans dossier.
     */
    overview: (organizationId: Uuid, personId: Uuid): Promise<WorkspaceOverview | null> =>
      call(`/organizations/${organizationId}/workspace`, (m) =>
        m.workspaceOverview(organizationId, personId),
      ),

    /** Le détail d'un dossier : suivi, fil partagé, historique. */
    proposalFile: (proposalId: Uuid, organizationId: Uuid): Promise<ProposalFile | null> =>
      call(`/proposals/${proposalId}/file`, (m) => m.proposalFile(proposalId, organizationId), {
        organization_id: organizationId,
      }),

    /** Éditions auxquelles cette organisation a déposé, pour grouper la liste. */
    editions: (organizationId: Uuid) =>
      call(`/organizations/${organizationId}/editions`, (m) => m.workspaceEditions(organizationId)),

    /**
     * RÉPONSE À UNE DEMANDE DE CORRECTION. Toujours `submitter` : le fil
     * partagé est le seul auquel l'organisation ait accès, et une réponse
     * n'est jamais elle-même une demande de correction.
     */
    reply: (personId: Uuid, payload: ReplyToCommentPayload) =>
      send(`/proposals/${payload.proposal_id}/comments`, payload, (m) =>
        m.replyToComment(personId, payload),
      ),

    /**
     * MARQUAGE « RÉSOLU », et son retrait. Le modèle porte `resolved_at` sans
     * dire qui l'écrit : l'écran l'ouvre au soumissionnaire et le laisse
     * revenir en arrière. Obligation d'API — c'est une règle d'autorisation.
     */
    resolve: (personId: Uuid, payload: ResolveCommentPayload) =>
      send(
        `/proposal-comments/${payload.comment_id}/resolution`,
        payload,
        (m) => m.resolveComment(personId, payload),
        payload.resolved ? 'POST' : 'DELETE',
      ),
  }
}
