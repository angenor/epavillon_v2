/**
 * ACCEPTATION D'UNE INVITATION (B7) — sa part de `useApi()`.
 *
 * Un seul appel, et un fichier pour lui : `useApi.ts` a franchi le garde-fou de
 * mille lignes, et la règle du projet est de faire sortir l'écran suivant plutôt
 * que d'épaissir le fichier commun. Le découpage reste celui des écrans, comme
 * pour `api/planner.ts` ou `api/admin-organizations.ts`.
 *
 * ── CE QUI REND CET APPEL DIFFÉRENT DE TOUS LES AUTRES ──────────────────────
 *
 * IL NE PREND AUCUN IDENTIFIANT DE PERSONNE, et n'exige aucune session. Le jeton
 * du lien EST la preuve d'adresse — c'est la même règle que la vérification
 * d'adresse de B1. La personne qu'une invitation vise n'a le plus souvent pas
 * encore de compte : exiger une session avant d'accepter reviendrait à lui
 * demander ce que l'invitation est précisément censée déclencher.
 *
 * Si une session existe malgré tout, elle doit désigner la même personne :
 * l'API refuse alors par `ORG_INVITATION_NOT_YOURS`, et l'écran le rend comme
 * une erreur ordinaire — c'est le cas de quelqu'un de connecté qui suit le lien
 * reçu par un collègue.
 *
 * L'ADRESSE EST MARQUÉE VÉRIFIÉE par l'API dans la même transaction : le lien
 * vient de la prouver, et redemander un second courriel de vérification serait
 * une formalité vide.
 */

import type { AcceptInvitationResult } from '~/types/organization-workspace'
import type { ApiTransport } from './proposal-review'

export function createInvitationApi({ send }: ApiTransport) {
  return {
    /**
     * `POST /organizations/invitations/accept`, corps `{ token, job_title }`.
     *
     * La fonction est EXIGÉE : l'adhésion devient active, et une adhésion active
     * porte toujours celle de la personne. C'est l'invitée qui la déclare, pas
     * le référent qui l'a invitée.
     */
    accept: (token: string, jobTitle: string): Promise<AcceptInvitationResult> =>
      send('/organizations/invitations/accept', { token, job_title: jobTitle }, (m) =>
        m.acceptInvitation(token),
      ),
  }
}
