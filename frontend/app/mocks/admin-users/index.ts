/**
 * UTILISATEURS ET RÔLES (A12) — point d'entrée des données simulées. Ne contient
 * AUCUNE donnée : il ne fait que ré-exporter.
 *
 * Quatre fichiers, découpés par nature comme `admin-organizations/` :
 *
 *   session.ts  le journal d'écritures de la démonstration — attributions,
 *               révocations, changements de statut, demandes traitées
 *   core.ts     la RÉSOLUTION DES PORTÉES (une jointure applicative : `scope_id`
 *               n'a aucune clé étrangère), la liste et son filtrage par
 *               périmètre, le catalogue des rôles offerts au panneau
 *   detail.ts   la fiche, l'historique des attributions et des révocations, les
 *               permissions effectives « et où », la file RGPD
 *   writes.ts   les quatre écritures et les cinq refus qu'elles portent
 */

export {
  activeAssignmentsOf,
  allAssignmentsOf,
  assignableRoles,
  resolveScope,
  roleAssignmentOptions,
  roleView,
  userListRow,
  userListScreen,
} from './core'

export {
  assignmentHistoryOf,
  effectivePermissionsView,
  privacyQueue,
  privacyRequestView,
  privacyRequestsOf,
  userDetail,
} from './detail'

export { grantRole, handlePrivacyRequest, revokeRole, setPersonStatus } from './writes'
