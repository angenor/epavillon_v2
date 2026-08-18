/**
 * ORGANISATIONS ET FUSION DES DOUBLONS (A11) — point d'entrée des données
 * simulées. Ne contient AUCUNE donnée : il ne fait que ré-exporter.
 *
 * Cinq fichiers, découpés par nature et non par écran — c'est le cas où la règle
 * de découpage du projet s'applique à un dossier plutôt qu'à un fichier, comme
 * `mocks/admin-events/` :
 *
 *   session.ts     le journal d'écritures de la démonstration et la REDIRECTION
 *                  de fusion (`resolve_organization`), lu par les quatre autres
 *   core.ts        `mv_organization_scorecard` rejouée, et l'écran de la liste
 *   duplicates.ts  la file des doublons, la comparaison champ par champ et le
 *                  décompte de transfert lu dans `organization_references`
 *   detail.ts      la fiche : dénominations, domaines, membres, activités,
 *                  historique
 *   writes.ts      les cinq écritures — fusion, arbitrage d'une paire, sceau,
 *                  vérification d'un domaine, confirmation d'une dénomination
 */

export { organizationListScreen, organizationScorecards } from './core'
export { duplicateQueue, mergePreview, pendingDuplicatesOf } from './duplicates'
export { organizationDetail } from './detail'
export {
  decideDuplicatePair,
  mergeOrganizations,
  setDomainVerification,
  setNameConfirmation,
  setOrganizationVerification,
} from './writes'
