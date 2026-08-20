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
 *   event.ts             les deux séries et les quatre éditions : COP31 à venir,
 *                        COP30 et COP29 passées, cycle de webinaires PACO
 *   rooms.ts             les pavillons, leurs salles, leurs canaux de diffusion
 *   tracks.ts            les trois journées spéciales
 *   calls.ts             l'appel ouvert, les deux appels clos et le comité
 *   criteria.ts          les six critères pondérés, dont un éliminatoire
 *   proposals/           41 dossiers, découpés par statut, plus l'historique
 *                        champ par champ et le dossier d'une édition passée
 *   proposal-submission.ts  la numérotation, la recevabilité et les deux
 *                        écritures du formulaire de dépôt (prompt A4)
 *   reminders.ts         la règle de rappel de chaque édition, et les quatre
 *                        rappels cumulés d'une séance, dérivés comme en base
 *   organization-workspace.ts  les compositions et les trois écritures de
 *                        l'espace organisation (prompt A5)
 *   reviews.ts           les revues, leurs notes par critère et les affectations
 *   sessions/            44 séances : 30 pour la COP31, dont deux conflits
 *                        volontaires, et 14 pour les autres éditions
 *   registration-form.ts les formulaires d'inscription configurables
 *   registrations.ts     67 inscriptions, canaux d'acquisition variés, dont
 *                        sept sur la seule séance du jeu qui se soit tenue
 *   views.ts             les deux vues du modèle, reconstituées
 *   content.ts           treize diapositives de la vitrine, les six natures de
 *                        `highlight_nature` et `content.v_showcase` rejouée —
 *                        une archivée, un brouillon, une hors fenêtre
 *   home.ts              la composition de l'accueil public (A15), l'historique
 *                        des éditions, les deux vues qu'il consomme
 *                        (`v_public_editions`, `v_edition_stats`) et le journal
 *                        d'écritures que le back-office alimente
 *   admin-showcase.ts    le back-office de la vitrine (A15) : la liste filtrée
 *                        par périmètre, le formulaire et son aperçu, l'ordre et
 *                        les quatre écritures
 *   incidents.ts         sept messages d'incident : les cinq états que
 *                        `live.event_incidents()` distingue, les cinq portées
 *   admin-incidents.ts   ce que l'écran A13 en fait — état, portée, cible
 *                        résolue, publication et dépublication
 *   analytics.ts         les projections du module Analytique, reconstituées
 *   admin-dashboard.ts   la composition du tableau de bord du back-office (A6)
 *   permissions.ts       le catalogue des permissions et ce que chaque rôle
 *                        permet — l'autorisation se teste par PERMISSION
 *   proposal-workflow.ts la machine à états des dossiers, en données
 *   proposal-reads.ts    les accusés de lecture, par dossier et par personne
 *   admin-proposals.ts   la composition de la liste des propositions (A7) et
 *                        ses actions groupées
 *   proposal-review.ts   la composition de la fiche d'évaluation (A8), le voile
 *                        de l'évaluation en aveugle et ses quatre écritures
 *   admin-planner.ts     la composition du planificateur de créneaux (A9) et ses
 *                        quatre écritures — dont AUCUNE ne refuse un
 *                        chevauchement
 *   admin-organizations/ les organisations et la fusion des doublons (A11), en
 *                        cinq fichiers — `session` le journal d'écritures et la
 *                        redirection de fusion, `core` la fiche de performance
 *                        et la liste, `duplicates` la file et le décompte de
 *                        transfert lu dans le registre, `detail` la fiche,
 *                        `writes` les cinq écritures
 *   platform.ts          les onze modules de `platform.modules` — l'écran des
 *                        permissions groupe par module, et leur nom est une
 *                        DONNÉE, pas une chaîne d'interface
 *   feature-flags.ts     les treize drapeaux de `platform.feature_flags`, tels
 *                        que le semis les pose — c'est le ROUTAGE qui les lit,
 *                        pour servir la page « En cours de maintenance » (A14)
 *   privacy.ts           les demandes RGPD et les consentements (A12) : les
 *                        échéances sont relatives à maintenant, sans quoi la
 *                        file n'aurait qu'un cas à montrer
 *   admin-users/         les utilisateurs et les rôles (A12), en quatre
 *                        fichiers — `session` le journal, `core` la résolution
 *                        des portées et la liste, `detail` la fiche,
 *                        l'historique et les permissions effectives, `writes`
 *                        les quatre écritures
 *   admin-events/        la gestion des événements (A10), en trois fichiers —
 *                        `core` le socle, `detail` la composition des six
 *                        onglets, `writes` les quinze écritures et les
 *                        contraintes de `060_events.sql` qui refusent
 */

export * from './ids'

export { locales, countries, taxonomies, taxonomyTerms, entityTerms } from './reference'
export {
  organizations,
  organizationNames,
  organizationDomains,
  organizationReferences,
  duplicateCandidates,
} from './org'
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
  allMemberships,
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
export { reviewCriteria, maxWeightedScore, maxWeightedScoreOf, seedDefaultCriteria } from './criteria'

export {
  allProposals,
  acceptedProposals,
  draftProposals,
  pastEditionProposals,
  reviewedProposals,
  submittedProposals,
  proposalOrganizations,
  proposalSpeakers,
  proposalAssets,
  proposalDocuments,
  proposalComments,
  proposalTransitions,
} from './proposals'

export {
  openCallForProposals,
  proposalFormContext,
  draftProposalOf,
  saveProposalDraft,
  submitProposal,
  lookupSpeakerByEmail,
} from './proposal-submission'

export { reviews, reviewScores, reviewAssignments } from './reviews'

export {
  allSessions,
  publishedSessions,
  plannedSessions,
  otherEditionSessions,
  sessionSpeakers,
  sessionTracks,
  sessionOrganizations,
} from './sessions'

export { proposalHistories, proposalHistory } from './proposals/history'

export {
  editableProposal,
  editedProposal,
  saveExistingProposal,
  resubmitProposal,
} from './proposal-edit'

export { reminderRules, sessionReminders, sessionReminderSchedule } from './reminders'

export {
  workspaceOverview,
  workspaceEditions,
  proposalFile,
  inviteMember,
  acceptInvitation,
  decideMembership,
  replyToComment,
  resolveComment,
} from './organization-workspace'

export {
  assetOf,
  attachedImage,
  attachmentOf,
  coverAssets,
  coverAttachments,
  showcaseAssets,
  showcaseAttachments,
} from './covers'
export { registrationForms, registrationFormFields } from './registration-form'
export { registrations } from './registrations'
export { publicSchedule, proposalDashboard } from './views'
export { detectConflicts, publicationReadiness } from './conflicts'
export { incidents } from './incidents'
export {
  activeIncidentsForEvent,
  createIncident,
  eventIncidents,
  incidentById,
  incidentListScreen,
  overrunTemplate,
  publishIncident,
  unpublishIncident,
  updateIncident,
} from './admin-incidents'

export {
  proposalFunnel,
  dailySubmissions,
  dailyRegistrations,
  reviewerWorkload,
  platformOverview,
  operationalHealth,
  lastAnalyticsRefresh,
} from './analytics'

export { adminDashboard } from './admin-dashboard'

export { permissions, rolePermissions, roles, effectivePermissions } from './permissions'
export { platformModules, moduleByCode, moduleRank } from './platform'
export { featureFlags, isFeatureEnabled } from './feature-flags'
export { privacyRequests, consents, currentConsents } from './privacy'
export { proposalTransitionsAllowed } from './proposal-workflow'
export { proposalReads } from './proposal-reads'
export {
  proposalListScreen,
  committeeOf,
  assignReviewer,
  changeProposalStatus,
} from './admin-proposals'

export {
  reviewDesk,
  organizationTrackRecord,
  saveReview,
  recuseFromProposal,
  postProposalComment,
  decideProposal,
} from './proposal-review'

export {
  plannerScreen,
  scheduleSession,
  setSessionTracks,
  setSessionBroadcast,
  publishProgramme,
} from './admin-planner'

export {
  editionListScreen,
  editionFormOptions,
  editionDetail,
  saveEdition,
  planDayGeneration,
  generateEventDays,
  saveEventDay,
  saveTrack,
  removeTrack,
  saveVenue,
  removeVenue,
  saveRoom,
  removeRoom,
  saveChannel,
  removeChannel,
  saveCall,
  defaultCriteriaGrid,
  saveCommittee,
} from './admin-events'

export {
  organizationListScreen,
  organizationScorecards,
  duplicateQueue,
  mergePreview,
  pendingDuplicatesOf,
  organizationDetail,
  decideDuplicatePair,
  mergeOrganizations,
  setDomainVerification,
  setNameConfirmation,
  setOrganizationVerification,
} from './admin-organizations'

export {
  activeAssignmentsOf,
  allAssignmentsOf,
  assignableRoles,
  assignmentHistoryOf,
  effectivePermissionsView,
  grantRole,
  handlePrivacyRequest,
  privacyQueue,
  privacyRequestView,
  privacyRequestsOf,
  resolveScope,
  revokeRole,
  roleAssignmentOptions,
  roleView,
  setPersonStatus,
  userDetail,
  userListRow,
  userListScreen,
} from './admin-users'

export {
  highlightMediaRules,
  highlightNatureTerms,
  highlightThemeBadges,
  highlights,
  showcase,
  showcaseRowOf,
} from './content'

export {
  currentEdition,
  editionHistory,
  editionStats,
  homeScreen,
  publicEditions,
} from './home'

export {
  blankShowcase,
  duplicateShowcase,
  moveShowcase,
  saveShowcase,
  setShowcaseStatus,
  showcaseById,
  showcaseForm,
  showcaseList,
} from './admin-showcase'
