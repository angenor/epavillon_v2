/**
 * Point d'entrée des données simulées. Ne contient AUCUNE donnée : il ne fait
 * que ré-exporter.
 *
 * AUCUNE PAGE N'IMPORTE CE FICHIER. Les écrans passent par `useApi()`, qui lit
 * ces mocks aujourd'hui et appellera l'API réelle demain, sans qu'un composant
 * change d'une ligne. C'est ce composable, et lui seul, qui importe ici.
 *
 * Contenu, et où le trouver :
 *
 *   ids.ts               les identifiants partagés, déclarés une seule fois
 *   reference.ts         pays, locales, taxonomies, rattachement aux thématiques
 *   org.ts               13 fiches d'organisation, dont DEUX EN DOUBLON
 *   memberships.ts       les adhésions, avec un cas en attente et un révoqué
 *   organization-search.ts  `find_similar_organizations()` rejouée, la lecture
 *                        des domaines et les deux écritures de l'écran A2
 *   people.ts            27 personnes, leurs rôles, dont une administratrice
 *                        limitée à la seule COP31
 *   auth.ts              les comptes, les jetons de courriel et la logique de
 *                        connexion que l'API portera (prompt A1)
 *   event.ts             la série, l'édition COP31, ses douze jours
 *   rooms.ts             le pavillon, ses trois salles, son canal de diffusion
 *   tracks.ts            les deux journées spéciales
 *   calls.ts             l'appel à propositions ouvert et son comité
 *   criteria.ts          les six critères pondérés, dont un éliminatoire
 *   proposals/           40 dossiers, découpés par statut
 *   reviews.ts           les revues, leurs notes par critère et les affectations
 *   sessions/            30 séances, dont deux conflits volontaires
 *   registration-form.ts les formulaires d'inscription configurables
 *   registrations.ts     60 inscriptions, canaux d'acquisition variés
 *   views.ts             les deux vues du modèle, reconstituées
 */

export * from './ids'

export { locales, countries, taxonomies, taxonomyTerms, entityTerms } from './reference'
export { organizations, organizationNames, organizationDomains, duplicateCandidates } from './org'
export { memberships } from './memberships'
export { people, roleAssignments } from './people'

export {
  publicEmailDomains,
  normalizeLabel,
  extractDomain,
  activeMemberCount,
  organizationsWithSession,
  organizationById,
  findSimilarOrganizations,
  organizationForEmail,
  membershipOf,
  membershipsOfPerson,
  joinOrganization,
  createOrganization,
} from './organization-search'

export {
  accounts,
  oneTimeTokens,
  DEMO_PASSWORD,
  authenticate,
  registerPerson,
  verifyEmailToken,
  checkPasswordResetToken,
  resetPassword,
} from './auth'
export { eventSeries, events, eventDays } from './event'
export { venues, rooms, broadcastChannels } from './rooms'
export { programmeTracks } from './tracks'
export { callsForProposals, callReviewers } from './calls'
export { reviewCriteria, maxWeightedScore } from './criteria'

export {
  allProposals,
  acceptedProposals,
  draftProposals,
  reviewedProposals,
  submittedProposals,
  proposalOrganizations,
  proposalSpeakers,
  proposalAssets,
  proposalDocuments,
  proposalComments,
  proposalTransitions,
} from './proposals'

export { reviews, reviewScores, reviewAssignments } from './reviews'

export {
  allSessions,
  publishedSessions,
  plannedSessions,
  sessionSpeakers,
  sessionTracks,
  sessionOrganizations,
} from './sessions'

export { coverAssets, coverAttachments, attachedImage } from './covers'
export { registrationForms, registrationFormFields } from './registration-form'
export { registrations } from './registrations'
export { publicSchedule, proposalDashboard } from './views'
export { detectConflicts, publicationReadiness } from './conflicts'
