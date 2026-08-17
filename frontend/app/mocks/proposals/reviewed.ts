/**
 * Les onze dossiers passés entre les mains du comité : cinq en cours
 * d'évaluation, trois renvoyés pour correction, trois écartés.
 *
 * C'est le fichier que lit la fiche d'évaluation (A8), l'écran le plus dense de
 * la plateforme. Trois situations y sont délibérément représentées :
 *
 *   - un dossier dont les trois revues exigées sont rendues et qui attend la
 *     décision (`assuranceParametrique`) ;
 *   - un dossier dont il manque encore des revues (`interpretation`, une seule
 *     sur trois) : `reviews_missing` doit sauter aux yeux ;
 *   - un dossier ÉLIMINÉ par le critère éliminatoire (`captageAir`,
 *     `is_knocked_out`), dont la moyenne n'est pas la raison du refus. Un écran
 *     qui trie par note sans afficher ce drapeau rend la décision incompréhensible.
 *
 * Les notes suivent la grille de `mocks/criteria.ts` : `weighted_score` sur 40,
 * `average_score` sur 20. Elles se retrouvent, revue par revue et critère par
 * critère, dans `mocks/reviews.ts`.
 */

import type { Proposal } from '~/types/programme/proposal'
import { COUNTRY, ORG, PERSON, PROPOSAL } from '../ids'
import { proposal } from './_shared'

export const reviewedProposals = [
  // --- En cours d'évaluation ------------------------------------------------

  proposal({
    id: PROPOSAL.budgetsGenre,
    ref: 20,
    organization: ORG.cofemac,
    submittedBy: PERSON.ngoBassong,
    title: {
      fr: 'Des budgets climat sensibles au genre : outiller les ministères des finances',
      en: 'Gender-responsive climate budgets: equipping finance ministries',
    },
    slug: 'budgets-climat-sensibles-au-genre',
    summary: {
      fr: "Un marqueur budgétaire ne suffit pas : encore faut-il que la direction du budget sache s'en servir.",
    },
    objectives: {
      fr: "Transmettre la méthode de marquage budgétaire genre-climat éprouvée dans trois pays et en montrer les limites d'exécution.",
    },
    presentation: {
      fr: "Trois ministères des finances d'Afrique centrale ont introduit un double marquage des lignes budgétaires, climat et genre. L'exercice a révélé que la difficulté n'est pas méthodologique mais organisationnelle : les directions sectorielles renseignent le marqueur en fin de cycle, sans effet sur l'arbitrage. La session présente la méthode, les résultats de trois exercices budgétaires et les corrections apportées au calendrier.",
    },
    outcomes: { fr: "Un guide de marquage adapté aux nomenclatures budgétaires francophones." },
    audience: [
      { fr: "Directions du budget" },
      { fr: "Points focaux genre" },
      { fr: "Partenaires techniques et financiers" },
    ],
    format: 'hybrid',
    category: 'capacity_building',
    country: COUNTRY.cm,
    preferredStart: '2027-11-12T14:00:00-03:00',
    preferredEnd: '2027-11-12T15:30:00-03:00',
    duration: 90,
    status: 'under_review',
    submittedAt: '2026-07-02T09:30:00Z',
    createdAt: '2026-06-24T11:00:00Z',
    updatedAt: '2026-08-05T16:20:00Z',
    averageScore: 15.25,
    weightedScore: 30.5,
    reviewCount: 2,
    viewCount: 47,
  }),
  proposal({
    id: PROPOSAL.cartographieCotonou,
    ref: 21,
    organization: ORG.anteb,
    submittedBy: PERSON.zinsou,
    title: {
      fr: "Cartographie participative des risques d'inondation à Cotonou",
      en: 'Participatory flood risk mapping in Cotonou',
    },
    slug: 'cartographie-participative-inondations-cotonou',
    summary: {
      fr: "Douze quartiers ont cartographié eux-mêmes leurs points de submersion ; la carte officielle en ignorait la moitié.",
    },
    objectives: {
      fr: "Montrer comment une cartographie produite par les habitants a corrigé le plan de prévention des risques et modifié les priorités d'investissement.",
    },
    presentation: {
      fr: "Le plan de prévention reposait sur une modélisation hydraulique alimentée par un relevé topographique ancien. Douze comités de quartier ont relevé les hauteurs d'eau observées sur cinq saisons. Le croisement des deux sources a fait apparaître des zones de stagnation absentes du modèle, liées au bâti et aux remblais. La session détaille le protocole, le traitement des relevés et l'intégration réglementaire.",
    },
    outcomes: { fr: "Un protocole de relevé communautaire transposable à d'autres villes côtières." },
    audience: [
      { fr: "Collectivités" },
      { fr: "Agences d'urbanisme" },
      { fr: "ONG de terrain" },
      { fr: "Bailleurs" },
    ],
    format: 'in_person',
    category: 'field_project',
    country: COUNTRY.bj,
    preferredStart: '2027-11-10T10:00:00-03:00',
    preferredEnd: '2027-11-10T11:30:00-03:00',
    duration: 90,
    status: 'under_review',
    submittedAt: '2026-07-04T08:15:00Z',
    createdAt: '2026-06-26T14:40:00Z',
    updatedAt: '2026-08-07T10:05:00Z',
    averageScore: 13.5,
    weightedScore: 27.0,
    reviewCount: 3,
    viewCount: 38,
  }),
  proposal({
    // Une seule revue sur les trois exigées : c'est le dossier en retard que le
    // tableau de bord doit faire remonter.
    id: PROPOSAL.interpretation,
    ref: 22,
    organization: ORG.ifdd,
    submittedBy: PERSON.tremblay,
    title: {
      fr: 'Interprétation et multilinguisme dans les négociations climatiques',
      en: 'Interpretation and multilingualism in climate negotiations',
    },
    slug: 'interpretation-multilinguisme-negociations',
    summary: {
      fr: "Ce que coûte, à une délégation francophone, une session de contact tenue sans interprétation.",
    },
    objectives: {
      fr: "Documenter l'effet de l'absence d'interprétation sur la participation effective des délégations francophones et proposer des mesures concrètes au secrétariat.",
    },
    presentation: {
      fr: "Une part importante des consultations informelles se tient sans interprétation. Le relevé conduit sur deux conférences montre la corrélation entre absence d'interprétation et retrait des délégations non anglophones des textes en discussion. La session présente ces relevés et met en débat trois mesures : interprétation à la demande, publication différée des textes, appui linguistique mutualisé entre délégations francophones.",
    },
    audience: [
      { fr: "Négociateurs" },
      { fr: "Secrétariat de la CCNUCC" },
      { fr: "Organisations d'appui aux délégations" },
    ],
    format: 'in_person',
    category: 'concertation',
    languages: ['fr', 'en'],
    country: COUNTRY.ca,
    duration: 60,
    status: 'under_review',
    submittedAt: '2026-07-09T13:00:00Z',
    createdAt: '2026-07-01T09:20:00Z',
    updatedAt: '2026-08-01T09:40:00Z',
    averageScore: 11.0,
    weightedScore: 22.0,
    reviewCount: 1,
    viewCount: 25,
  }),
  proposal({
    id: PROPOSAL.reboisementUrbain,
    ref: 23,
    organization: ORG.cvdmf,
    submittedBy: PERSON.tranVanMinh,
    title: {
      fr: "Reboisement urbain : mesurer l'effet réel sur les îlots de chaleur",
      en: 'Urban reforestation: measuring the real effect on heat islands',
    },
    slug: 'reboisement-urbain-ilots-chaleur',
    summary: {
      fr: "Trois ans de capteurs dans deux villes du delta : l'effet est réel, il est très inégal.",
    },
    objectives: {
      fr: "Livrer des mesures de terrain sur l'effet du couvert arboré urbain et distinguer les plantations qui rafraîchissent de celles qui ne changent rien.",
    },
    presentation: {
      fr: "Deux villes du delta du Mékong ont planté plusieurs milliers d'arbres d'alignement. Un réseau de capteurs a suivi les températures de surface et d'air sur trois ans. L'écart entre rues plantées et rues témoins varie fortement selon l'essence, la largeur de la voie et l'arrosage des deux premières années. La session présente les mesures et un abaque de décision pour les services techniques.",
    },
    outcomes: { fr: "Un abaque essence / largeur de voie / gain thermique attendu." },
    audience: [
      { fr: "Services techniques municipaux" },
      { fr: "Urbanistes" },
      { fr: "Chercheurs" },
    ],
    format: 'hybrid',
    category: 'results_sharing',
    country: COUNTRY.vn,
    preferredStart: '2027-11-13T09:30:00-03:00',
    preferredEnd: '2027-11-13T11:00:00-03:00',
    duration: 90,
    status: 'under_review',
    submittedAt: '2026-07-12T04:45:00Z',
    createdAt: '2026-07-03T06:15:00Z',
    updatedAt: '2026-08-06T12:30:00Z',
    averageScore: 12.75,
    weightedScore: 25.5,
    reviewCount: 2,
    viewCount: 31,
  }),
  proposal({
    // Les trois revues exigées sont rendues : le dossier attend une décision,
    // pas un révisionniste.
    id: PROPOSAL.assuranceParametrique,
    ref: 24,
    organization: ORG.osed,
    submittedBy: PERSON.ouedraogo,
    contact: PERSON.kabore,
    title: {
      fr: 'Assurance paramétrique agricole : trois ans de mise en œuvre au Sahel',
      en: 'Parametric crop insurance: three years of implementation in the Sahel',
    },
    slug: 'assurance-parametrique-agricole-sahel',
    summary: {
      fr: "Des indemnisations déclenchées par satellite, versées en quinze jours — et des producteurs qui n'ont rien touché malgré une mauvaise récolte.",
    },
    objectives: {
      fr: "Exposer sans complaisance les résultats de trois campagnes d'assurance indicielle, y compris les cas de non-déclenchement mal vécus par les producteurs.",
    },
    presentation: {
      fr: "L'assurance indicielle indemnise selon un indice satellitaire de végétation, sans expertise de terrain. Trois campagnes ont été suivies auprès de coopératives cotonnières et céréalières. Les versements sont rapides, mais l'indice s'écarte parfois du rendement observé : la session détaille ces écarts de base, leur fréquence, et les corrections apportées au produit d'assurance.",
    },
    outcomes: { fr: "Une méthode de contrôle de l'écart de base, applicable avant le lancement d'un produit." },
    audience: [
      { fr: "Assureurs" },
      { fr: "Coopératives agricoles" },
      { fr: "Ministères de l'agriculture" },
      { fr: "Bailleurs" },
    ],
    format: 'in_person',
    category: 'results_sharing',
    country: COUNTRY.bf,
    preferredStart: '2027-11-12T16:00:00-03:00',
    preferredEnd: '2027-11-12T17:30:00-03:00',
    duration: 90,
    status: 'under_review',
    submittedAt: '2026-07-16T10:20:00Z',
    createdAt: '2026-07-06T15:10:00Z',
    updatedAt: '2026-08-08T14:15:00Z',
    averageScore: 16.5,
    weightedScore: 33.0,
    reviewCount: 3,
    viewCount: 52,
  }),

  // --- Corrections demandées ------------------------------------------------
  // L'état `changes_requested` est piloté par un fil de commentaires marqué
  // `is_change_request` : voir `mocks/proposals/exchanges.ts`.

  proposal({
    id: PROPOSAL.numeriqueResponsable,
    ref: 25,
    organization: ORG.verdeo,
    submittedBy: PERSON.moreau,
    title: {
      fr: 'Numérique responsable : réduire l’empreinte des administrations',
      en: 'Responsible digital: cutting the footprint of public administrations',
    },
    slug: 'numerique-responsable-administrations',
    objectives: {
      fr: "Présenter la méthode d'inventaire du parc informatique public et les gisements d'économies identifiés dans deux administrations.",
    },
    presentation: {
      fr: "L'empreinte du numérique public est dominée par la fabrication des équipements, non par leur usage. L'inventaire conduit dans deux administrations montre un renouvellement plus rapide que nécessaire, faute de politique d'allongement de durée de vie. La session présente la méthode d'inventaire et les mesures d'allongement retenues.",
    },
    format: 'online',
    category: 'best_practices',
    country: COUNTRY.fr,
    // Créneau et durée : un dossier DÉPOSÉ les porte forcément — l'écran de
    // dépôt les exige depuis les retours du 17/08. Sans eux, ce dossier n'aurait
    // jamais pu partir, et l'organisation ne pourrait pas le renvoyer corrigé.
    preferredStart: '2027-11-16T11:00:00-03:00',
    preferredEnd: '2027-11-16T12:00:00-03:00',
    duration: 60,
    status: 'changes_requested',
    submittedAt: '2026-07-18T09:00:00Z',
    createdAt: '2026-07-10T13:25:00Z',
    updatedAt: '2026-08-10T08:30:00Z',
    averageScore: 10.5,
    weightedScore: 21.0,
    reviewCount: 2,
    viewCount: 19,
  }),
  proposal({
    id: PROPOSAL.dechetsPlastiques,
    ref: 26,
    organization: ORG.ujfc,
    submittedBy: PERSON.koffi,
    title: {
      fr: 'Déchets plastiques et climat : ce que valent vraiment les filières de valorisation',
      en: 'Plastic waste and climate: what recovery channels are really worth',
    },
    slug: 'dechets-plastiques-filieres-valorisation',
    objectives: {
      fr: "Comparer le bilan carbone réel de quatre filières de valorisation du plastique en Afrique de l'Ouest.",
    },
    presentation: {
      fr: "Quatre filières coexistent : recyclage mécanique artisanal, pyrolyse, valorisation énergétique en cimenterie et export. Leur bilan carbone diffère d'un facteur important, rarement documenté localement. La session présente les mesures conduites sur douze sites et les conséquences pour les politiques municipales de gestion des déchets.",
    },
    format: 'hybrid',
    category: 'results_sharing',
    country: COUNTRY.ci,
    duration: 90,
    status: 'changes_requested',
    submittedAt: '2026-07-21T11:40:00Z',
    createdAt: '2026-07-13T08:50:00Z',
    updatedAt: '2026-08-12T15:00:00Z',
    averageScore: 9.75,
    weightedScore: 19.5,
    reviewCount: 1,
    viewCount: 22,
  }),
  proposal({
    id: PROPOSAL.ecolesResilientes,
    ref: 27,
    organization: ORG.fhrc,
    submittedBy: PERSON.josephPierre,
    title: {
      fr: 'Écoles résilientes : adapter le bâti scolaire aux vagues de chaleur',
      en: 'Resilient schools: adapting school buildings to heatwaves',
    },
    slug: 'ecoles-resilientes-vagues-de-chaleur',
    summary: {
      fr: "Les jours de classe perdus pour cause de chaleur ne sont comptés nulle part.",
    },
    objectives: {
      fr: "Documenter les interruptions scolaires liées à la chaleur et évaluer trois solutions de rafraîchissement passif du bâti existant.",
    },
    presentation: {
      fr: "Les salles de classe en toiture métallique dépassent régulièrement les seuils de confort thermique. Trois solutions passives ont été testées sur des écoles comparables : sur-toiture ventilée, peinture réflective, plantation d'ombrage. La session présente les températures mesurées, les coûts et l'effet sur la fréquentation.",
    },
    audience: [
      { fr: "Ministères de l'éducation" },
      { fr: "Collectivités" },
      { fr: "ONG du secteur éducatif" },
    ],
    format: 'hybrid',
    category: 'field_project',
    country: COUNTRY.ht,
    duration: 90,
    status: 'changes_requested',
    submittedAt: '2026-07-24T16:10:00Z',
    createdAt: '2026-07-15T10:05:00Z',
    updatedAt: '2026-08-13T09:20:00Z',
    averageScore: 14.0,
    weightedScore: 28.0,
    reviewCount: 2,
    viewCount: 28,
  }),

  // --- Écartées -------------------------------------------------------------

  proposal({
    // ÉLIMINÉE PAR LE CRITÈRE ÉLIMINATOIRE. Sa moyenne n'est pas la raison du
    // refus : un membre du comité a mis zéro à la pertinence, et cela suffit.
    id: PROPOSAL.captageAir,
    ref: 17,
    organization: ORG.verdeo,
    submittedBy: PERSON.moreau,
    title: {
      fr: "Panorama des technologies de captage direct du CO₂ dans l'air",
      en: 'Overview of direct air capture technologies',
    },
    slug: 'captage-direct-co2-air',
    objectives: {
      fr: "Présenter l'état de l'art du captage direct et les projets industriels annoncés d'ici 2030.",
    },
    presentation: {
      fr: "Revue des procédés de captage direct, de leurs consommations énergétiques et des projets industriels annoncés en Amérique du Nord et au Moyen-Orient.",
    },
    format: 'online',
    category: 'technological_innovation',
    country: COUNTRY.fr,
    duration: 60,
    status: 'rejected',
    submittedAt: '2026-06-18T10:30:00Z',
    decidedAt: '2026-08-02T11:00:00Z',
    decisionReason:
      "Sujet sans lien avec les priorités du pavillon francophone : aucune organisation ni aucun territoire francophone n'est concerné par les projets présentés, et la session n'apporte rien aux délégations en négociation. Note éliminatoire sur le critère de pertinence.",
    decidedBy: PERSON.bakayoko,
    createdAt: '2026-06-12T14:00:00Z',
    updatedAt: '2026-08-02T11:00:00Z',
    averageScore: 7.0,
    weightedScore: 14.0,
    reviewCount: 3,
    knockedOut: true,
    viewCount: 41,
  }),
  proposal({
    id: PROPOSAL.creditsVolontaires,
    ref: 18,
    organization: ORG.verdeo,
    submittedBy: PERSON.moreau,
    title: {
      fr: "Plateforme d'échange de crédits carbone volontaires entre entreprises",
      en: 'A voluntary carbon credit exchange platform for companies',
    },
    slug: 'plateforme-credits-carbone-volontaires',
    objectives: {
      fr: "Présenter une place de marché de crédits volontaires destinée aux entreprises francophones.",
    },
    presentation: {
      fr: "Démonstration d'une plateforme d'échange de crédits volontaires, de son mécanisme de tarification et de son dispositif de vérification.",
    },
    format: 'online',
    category: 'technological_innovation',
    country: COUNTRY.fr,
    duration: 60,
    status: 'rejected',
    submittedAt: '2026-06-22T09:15:00Z',
    decidedAt: '2026-08-02T11:20:00Z',
    decisionReason:
      "Présentation à caractère commercial d'un service propriétaire, sans mise en perspective des critiques adressées aux marchés volontaires. Le pavillon n'accueille pas de démonstration produit.",
    decidedBy: PERSON.bakayoko,
    createdAt: '2026-06-17T16:45:00Z',
    updatedAt: '2026-08-02T11:20:00Z',
    averageScore: 8.75,
    weightedScore: 17.5,
    reviewCount: 3,
    viewCount: 36,
  }),
  proposal({
    id: PROPOSAL.retrospectiveCop,
    ref: 19,
    organization: ORG.mvoi,
    submittedBy: PERSON.rakotomalala,
    title: {
      fr: 'Rétrospective des dix dernières conférences des Parties',
      en: 'A retrospective of the last ten Conferences of the Parties',
    },
    slug: 'retrospective-dix-dernieres-cop',
    objectives: {
      fr: "Retracer en images les dix dernières conférences et les engagements successifs.",
    },
    presentation: {
      fr: "Projection commentée d'archives audiovisuelles couvrant dix conférences, suivie d'un échange avec la salle.",
    },
    format: 'in_person',
    category: 'awareness',
    country: COUNTRY.mg,
    duration: 60,
    status: 'rejected',
    submittedAt: '2026-06-29T07:40:00Z',
    decidedAt: '2026-08-02T11:35:00Z',
    decisionReason:
      "Contenu rétrospectif sans apport pour les négociations en cours. La rédaction est invitée à proposer ce format hors période de conférence, dans le cycle de rendez-vous de l'IFDD.",
    decidedBy: PERSON.perretAdmin,
    createdAt: '2026-06-23T12:20:00Z',
    updatedAt: '2026-08-02T11:35:00Z',
    averageScore: 8.0,
    weightedScore: 16.0,
    reviewCount: 3,
    viewCount: 30,
  }),
] satisfies Proposal[]
