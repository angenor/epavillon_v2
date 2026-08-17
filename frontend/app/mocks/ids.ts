/**
 * Identifiants partagés des données simulées — DÉCLARÉS ICI, ET NULLE PART AILLEURS.
 *
 * C'est ce fichier qui garantit la cohérence de l'ensemble : une proposition
 * pointe vers une organisation qui existe, une session vers une proposition qui
 * existe, une inscription vers une session qui existe. Aucun autre fichier de
 * `app/mocks/` n'écrit un identifiant en clair ; tous importent d'ici.
 *
 * FORME DES IDENTIFIANTS. Ce sont des UUID valides et conformes à la version 7
 * (`platform.uuid_v7()`) : quatrième groupe commençant par `7`, cinquième par
 * `8`. Leur construction est DÉTERMINISTE et lisible à l'œil — la famille
 * d'entité se lit dans le quatrième groupe, le numéro d'ordre dans le dernier.
 * Rien n'est tiré au sort : deux exécutions donnent les mêmes valeurs, et un
 * identifiant rencontré dans une console se retrouve ici par simple recherche.
 *
 *   0198c1a0-0000-7040-8000-000000000017
 *                    ^^^^              ^^  famille 7040 = proposition, n° 17
 *
 * Les clés sont PARLANTES (`ORG.roac`, `PROPOSAL.adaptationCotiere`) : c'est ce
 * qui rend lisibles les renvois d'un fichier à l'autre. Un numéro d'ordre seul
 * (`p17`) obligerait à revenir ici pour savoir de quoi on parle.
 */

import type { Uuid } from '~/types/shared'

/** Compose un UUID v7 valide et stable pour une famille d'entités donnée. */
const uuid = (family: string, n: number): Uuid =>
  `0198c1a0-0000-${family}-8000-${String(n).padStart(12, '0')}`

// ---------------------------------------------------------------------------
// Référentiel — 020_reference.sql
// ---------------------------------------------------------------------------

/** `reference.countries` — sous-ensemble des pays semés par `900_seed.sql`. */
export const COUNTRY = {
  ca: uuid('7010', 1),
  fr: uuid('7010', 2),
  be: uuid('7010', 3),
  sn: uuid('7010', 4),
  ci: uuid('7010', 5),
  cm: uuid('7010', 6),
  bj: uuid('7010', 7),
  bf: uuid('7010', 8),
  ml: uuid('7010', 9),
  cd: uuid('7010', 10),
  mg: uuid('7010', 11),
  ma: uuid('7010', 12),
  ht: uuid('7010', 13),
  vn: uuid('7010', 14),
  br: uuid('7010', 15),
  ne: uuid('7010', 16),
  td: uuid('7010', 17),
} as const

/**
 * `reference.taxonomy_terms`. Les CODES sont ceux de `020_reference.sql` ; ces
 * identifiants ne servent qu'à `reference.entity_terms`, qui référence un terme
 * par sa clé primaire et non par son code.
 */
export const TERM = {
  // activity_theme
  mitigation: uuid('7011', 1),
  adaptation: uuid('7011', 2),
  climateAmbitionNdc: uuid('7011', 3),
  lossAndDamage: uuid('7011', 4),
  waterFisheries: uuid('7011', 5),
  renewableEnergyLand: uuid('7011', 6),
  healthSolidarity: uuid('7011', 7),
  industryTransition: uuid('7011', 8),
  transportUrbanization: uuid('7011', 9),
  climateJusticeIndigenous: uuid('7011', 10),
  agricultureFood: uuid('7011', 11),
  sustainableLivestock: uuid('7011', 12),
  climateFinance: uuid('7011', 13),
  gender: uuid('7011', 14),
  transparency: uuid('7011', 15),
  biodiversity: uuid('7011', 16),
  desertification: uuid('7011', 17),
  // activity_category
  capacityBuilding: uuid('7011', 30),
  resultsSharing: uuid('7011', 31),
  technologicalInnovation: uuid('7011', 32),
  fieldProject: uuid('7011', 33),
  bestPractices: uuid('7011', 34),
  awareness: uuid('7011', 35),
  concertation: uuid('7011', 36),
  // organization_type
  publicNationalInstitution: uuid('7011', 50),
  internationalOrganization: uuid('7011', 51),
  regionalOrganization: uuid('7011', 52),
  ngoAssociation: uuid('7011', 53),
  privateSector: uuid('7011', 54),
  academicResearch: uuid('7011', 55),
  mediaOrg: uuid('7011', 56),
  // document_type
  negotiationGuide: uuid('7011', 70),
  technicalNote: uuid('7011', 71),
  relevantDocument: uuid('7011', 72),
  presentation: uuid('7011', 73),
  report: uuid('7011', 74),
  // referral_source
  ifddWebsite: uuid('7011', 80),
  ifddLinkedin: uuid('7011', 81),
  ifddFacebook: uuid('7011', 82),
  ifddX: uuid('7011', 83),
  emailNewsletter: uuid('7011', 84),
  wordOfMouth: uuid('7011', 85),
  referralOther: uuid('7011', 86),
} as const

// ---------------------------------------------------------------------------
// Organisations — 040_organizations.sql
// ---------------------------------------------------------------------------

/**
 * `org.organizations`. Treize fiches pour douze entités réelles :
 * `osed` et `osedSigle` DÉSIGNENT LA MÊME ORGANISATION, saisie une fois par son
 * nom complet et une fois par son sigle. Ce doublon est délibéré — il alimente
 * l'écran de rattachement (A2) et l'écran de fusion (A11), qui n'ont aucun sens
 * sur un référentiel propre.
 */
export const ORG = {
  /** Organisation pivot, reprise de `900_seed.sql`. */
  ifdd: uuid('7001', 1),
  roac: uuid('7001', 2),
  osed: uuid('7001', 3),
  /** DOUBLON de `osed` : même entité, fiche créée par un membre pressé. */
  osedSigle: uuid('7001', 4),
  anteb: uuid('7001', 5),
  cofemac: uuid('7001', 6),
  imre: uuid('7001', 7),
  fhrc: uuid('7001', 8),
  cudcm: uuid('7001', 9),
  cvdmf: uuid('7001', 10),
  verdeo: uuid('7001', 11),
  ujfc: uuid('7001', 12),
  mvoi: uuid('7001', 13),
} as const

/**
 * Fiche que l'API CRÉERAIT depuis l'écran de rattachement (A2). Les mocks sont
 * en lecture seule : cet identifiant nomme la réponse rendue à l'écran, pas une
 * ligne qu'on retrouverait dans `organizations`. Numéroté hors de la série des
 * treize fiches pour qu'il n'en recouvre jamais une.
 */
export const ORG_CREATED: Uuid = uuid('7001', 90)

/** `org.organization_names` — dénominations complémentaires. */
export const ORG_NAME = (n: number): Uuid => uuid('7002', n)

/** `org.organization_domains`. */
export const ORG_DOMAIN = (n: number): Uuid => uuid('7003', n)

/** `org.memberships`. */
export const MEMBERSHIP = (n: number): Uuid => uuid('7004', n)

/** `org.duplicate_candidates`. */
export const DUPLICATE = {
  osed: uuid('7007', 1),
  imreCudcm: uuid('7007', 2),
} as const

// ---------------------------------------------------------------------------
// Personnes — 030_identity.sql
// ---------------------------------------------------------------------------

/**
 * `identity.people`. Vingt-cinq personnes. Les rôles se lisent dans
 * `mocks/people.ts` (`roleAssignments`) : `perretAdmin` est l'administratrice
 * dont le périmètre se limite à la COP31 — le cas qui met à l'épreuve le
 * filtrage de toutes les listes du back-office.
 */
export const PERSON = {
  // IFDD — équipe plateforme
  adminPivot: uuid('7005', 1),
  bakayoko: uuid('7005', 2),
  tremblay: uuid('7005', 3),
  perretAdmin: uuid('7005', 4),
  nkoDiop: uuid('7005', 5),
  // Comité de sélection
  lemoine: uuid('7005', 6),
  rasoanaivo: uuid('7005', 7),
  benAmor: uuid('7005', 8),
  kabore: uuid('7005', 9),
  duchesne: uuid('7005', 10),
  // Référents et membres d'organisations
  sowFall: uuid('7005', 11),
  ouedraogo: uuid('7005', 12),
  zinsou: uuid('7005', 13),
  ngoBassong: uuid('7005', 14),
  elFassi: uuid('7005', 15),
  josephPierre: uuid('7005', 16),
  gagnon: uuid('7005', 17),
  tranVanMinh: uuid('7005', 18),
  moreau: uuid('7005', 19),
  koffi: uuid('7005', 20),
  rakotomalala: uuid('7005', 21),
  mbayeNdiaye: uuid('7005', 22),
  // Intervenants et inscrits sans rôle particulier
  compaore: uuid('7005', 23),
  lambert: uuid('7005', 24),
  ilboudo: uuid('7005', 25),
  // Inscrite au prompt A1 : compte créé, adresse pas encore vérifiée.
  nouvelInscrit: uuid('7005', 26),
  // Ajoutée au prompt A2 : adresse vérifiée, AUCUNE organisation, et un domaine
  // de messagerie qui appartient à une fiche vérifiée en rattachement
  // automatique. C'est la seule personne du jeu qui puisse démontrer la
  // proposition « votre adresse appartient à … » — sans elle, cette branche de
  // l'écran de rattachement n'aurait aucune donnée.
  sy: uuid('7005', 27),
} as const

/**
 * `identity.accounts` — un compte mot de passe par personne connectée (A1).
 *
 * Famille 7009 et non 7007 : cette dernière est celle de `DUPLICATE`, et
 * `ACCOUNT(1)` valait donc exactement `DUPLICATE.osed`. Aucune donnée n'en était
 * fausse — deux tables sans lien entre elles — mais l'en-tête de ce fichier
 * promet qu'« un identifiant rencontré dans une console se retrouve ici par
 * simple recherche », et une valeur qui désigne deux choses tenait mal cette
 * promesse. Corrigé au prompt A2.
 */
export const ACCOUNT = (n: number): Uuid => uuid('7009', n)

/**
 * `identity.one_time_tokens` — vérification d'adresse et réinitialisation de mot
 * de passe (prompt A1). Ces identifiants sont ceux des LIGNES ; le jeton porté
 * par le lien du courriel est une chaîne à part, déclarée dans `mocks/auth.ts`,
 * car la base n'en garde que l'empreinte.
 */
export const ONE_TIME_TOKEN = (n: number): Uuid => uuid('7008', n)

/** `identity.role_assignments`. */
export const ROLE_ASSIGNMENT = (n: number): Uuid => uuid('7006', n)

// ---------------------------------------------------------------------------
// Événement — 060_events.sql
// ---------------------------------------------------------------------------

/** `event.event_series` — série reprise de `900_seed.sql`. */
export const SERIES = { copClimate: uuid('7020', 1) } as const

/** `event.events`. */
export const EVENT = { cop31: uuid('7021', 1) } as const

/** `event.event_days` — clé : le quantième de novembre 2027. */
export const EVENT_DAY = {
  nov09: uuid('7022', 9),
  nov10: uuid('7022', 10),
  nov11: uuid('7022', 11),
  nov12: uuid('7022', 12),
  nov13: uuid('7022', 13),
  nov14: uuid('7022', 14),
  nov15: uuid('7022', 15),
  nov16: uuid('7022', 16),
  nov17: uuid('7022', 17),
  nov18: uuid('7022', 18),
  nov19: uuid('7022', 19),
  nov20: uuid('7022', 20),
} as const

/** `event.venues`. */
export const VENUE = {
  pavillon: uuid('7023', 1),
  enLigne: uuid('7023', 2),
} as const

/** `event.rooms` — la salle `atelier` est virtuelle : pas d'exclusivité. */
export const ROOM = {
  baobab: uuid('7024', 1),
  fromager: uuid('7024', 2),
  atelier: uuid('7024', 3),
} as const

/** `event.broadcast_channels`. */
export const CHANNEL = { cop31: uuid('7025', 1) } as const

/** `event.programme_tracks` — les journées spéciales. */
export const TRACK = {
  financeDurable: uuid('7026', 1),
  jeunesseClimat: uuid('7026', 2),
} as const

// ---------------------------------------------------------------------------
// Appel à propositions — 060_events.sql § 5 et 6
// ---------------------------------------------------------------------------

/** `event.calls_for_proposals` — un seul appel par édition. */
export const CALL = { cop31: uuid('7030', 1) } as const

/** `event.review_criteria` — codes de `event.seed_default_criteria()`. */
export const CRITERION = {
  relevance: uuid('7031', 1),
  quality: uuid('7031', 2),
  impact: uuid('7031', 3),
  innovation: uuid('7031', 4),
  inclusiveness: uuid('7031', 5),
  feasibility: uuid('7031', 6),
} as const

// ---------------------------------------------------------------------------
// Propositions — 070_programme_proposals.sql
// ---------------------------------------------------------------------------

/**
 * `programme.proposals`. Quarante dossiers, numérotés dans l'ordre de leur
 * `reference_code` (`COP31-00001` … `COP31-00040`).
 */
export const PROPOSAL = {
  // Retenues
  adaptationCotiere: uuid('7040', 1),
  alertePrecoce: uuid('7040', 2),
  miniReseaux: uuid('7040', 3),
  agroecologie: uuid('7040', 4),
  pertesPrejudices: uuid('7040', 5),
  transitionJuste: uuid('7040', 6),
  rapportsBiennaux: uuid('7040', 7),
  villesMekong: uuid('7040', 8),
  contentieuxClimatique: uuid('7040', 9),
  releveJeunesse: uuid('7040', 10),
  mangroves: uuid('7040', 11),
  bassinNiger: uuid('7040', 12),
  article6: uuid('7040', 13),
  accesFondsVert: uuid('7040', 14),
  chaleurHopitaux: uuid('7040', 15),
  pastoralisme: uuid('7040', 16),
  // Rejetées
  captageAir: uuid('7040', 17),
  creditsVolontaires: uuid('7040', 18),
  retrospectiveCop: uuid('7040', 19),
  // En cours d'évaluation
  budgetsGenre: uuid('7040', 20),
  cartographieCotonou: uuid('7040', 21),
  interpretation: uuid('7040', 22),
  reboisementUrbain: uuid('7040', 23),
  assuranceParametrique: uuid('7040', 24),
  // Corrections demandées
  numeriqueResponsable: uuid('7040', 25),
  dechetsPlastiques: uuid('7040', 26),
  ecolesResilientes: uuid('7040', 27),
  // Déposées, en attente d'affectation
  transportFluvial: uuid('7040', 28),
  observatoireCdn: uuid('7040', 29),
  pmeAgroalimentaires: uuid('7040', 30),
  savoirsAutochtones: uuid('7040', 31),
  hydrogeneVert: uuid('7040', 32),
  sobrieteEnergetique: uuid('7040', 33),
  // Retirée par l'organisation, annulée par l'IFDD
  energiesMarines: uuid('7040', 34),
  salonSolutions: uuid('7040', 35),
  // Brouillons
  lacTchad: uuid('7040', 36),
  batiScolaire: uuid('7040', 37),
  corridorsEcologiques: uuid('7040', 38),
  journalismeClimatique: uuid('7040', 39),
  comptabiliteCarbone: uuid('7040', 40),
} as const

/** `programme.proposal_speakers`. */
export const PROPOSAL_SPEAKER = (n: number): Uuid => uuid('7041', n)

/** `programme.proposal_documents` et `media.assets` associés. */
export const PROPOSAL_DOCUMENT = (n: number): Uuid => uuid('7042', n)
export const ASSET = (n: number): Uuid => uuid('7043', n)

/** `programme.proposal_comments`. */
export const PROPOSAL_COMMENT = (n: number): Uuid => uuid('7044', n)

/** `programme.proposal_transitions`. */
export const PROPOSAL_TRANSITION = (n: number): Uuid => uuid('7045', n)

// ---------------------------------------------------------------------------
// Évaluation — 070_programme_proposals.sql § 5
// ---------------------------------------------------------------------------

/** `programme.review_assignments`. */
export const REVIEW_ASSIGNMENT = (n: number): Uuid => uuid('7050', n)

/** `programme.reviews`. */
export const REVIEW = (n: number): Uuid => uuid('7051', n)

// ---------------------------------------------------------------------------
// Sessions — 075_programme_sessions.sql
// ---------------------------------------------------------------------------

/**
 * `programme.sessions`. Trente occurrences : vingt issues des propositions
 * retenues, dix programmées directement par l'IFDD (`proposal_id` nul).
 *
 * Deux conflits sont volontaires et alimentent le planificateur (A9) :
 * `mangroves` et `pastoralisme` occupent la même salle au même moment,
 * `article6` et `accesFondsVert` sont diffusées simultanément sur l'unique canal.
 */
export const SESSION = {
  ouverture: uuid('7060', 1),
  adaptationCotiere: uuid('7060', 2),
  alertePrecoce: uuid('7060', 3),
  miniReseaux: uuid('7060', 4),
  agroecologie: uuid('7060', 5),
  pertesPrejudices: uuid('7060', 6),
  transitionJuste: uuid('7060', 7),
  rapportsBiennaux1: uuid('7060', 8),
  rapportsBiennaux2: uuid('7060', 9),
  villesMekong: uuid('7060', 10),
  contentieuxClimatique: uuid('7060', 11),
  releveJeunesse: uuid('7060', 12),
  mangroves: uuid('7060', 13),
  bassinNiger: uuid('7060', 14),
  article6: uuid('7060', 15),
  accesFondsVert: uuid('7060', 16),
  chaleurHopitaux: uuid('7060', 17),
  pastoralisme: uuid('7060', 18),
  atelierNegociation1: uuid('7060', 19),
  atelierNegociation2: uuid('7060', 20),
  pointPresse1: uuid('7060', 21),
  pointPresse2: uuid('7060', 22),
  cafeFrancophone: uuid('7060', 23),
  bilanFinance: uuid('7060', 24),
  forumJeunesse: uuid('7060', 25),
  webinairePreparatoire: uuid('7060', 26),
  rencontreNegociateurs: uuid('7060', 27),
  releveJeunesse2: uuid('7060', 28),
  ceremonieCloture: uuid('7060', 29),
  visiteTerrain: uuid('7060', 30),
} as const

/** `programme.session_speakers`. */
export const SESSION_SPEAKER = (n: number): Uuid => uuid('7061', n)

// ---------------------------------------------------------------------------
// Inscriptions — 075_programme_sessions.sql § 3 et 4
// ---------------------------------------------------------------------------

/** `programme.registration_forms`. */
export const REGISTRATION_FORM = {
  default: uuid('7070', 1),
  cop31: uuid('7070', 2),
} as const

/** `programme.registration_form_fields`. */
export const FORM_FIELD = (n: number): Uuid => uuid('7071', n)

/** `programme.registrations` — soixante inscriptions. */
export const REGISTRATION = (n: number): Uuid => uuid('7072', n)
