/**
 * Les huit dossiers DÉPOSÉS mais pas encore engagés dans l'évaluation, plus les
 * deux sorties de parcours.
 *
 *   - six `submitted` : reçus, en attente d'affectation à des membres du comité.
 *     Aucune note, aucun révisionniste désigné ;
 *   - un `withdrawn` : retiré par l'organisation elle-même, avec son motif ;
 *   - un `cancelled` : annulé par l'IFDD, motif obligatoire côté base.
 *
 * Ce sont les dossiers que le tableau de bord (A6) compte comme « à affecter » :
 * l'écran doit distinguer un dossier qui attend l'équipe d'un dossier qui attend
 * le comité.
 */

import type { Proposal } from '~/types/programme/proposal'
import { COUNTRY, ORG, PERSON, PROPOSAL } from '../ids'
import { proposal } from './_shared'

export const submittedProposals = [
  proposal({
    id: PROPOSAL.transportFluvial,
    ref: 28,
    organization: ORG.cofemac,
    submittedBy: PERSON.ngoBassong,
    title: {
      fr: 'Décarboner le transport fluvial sur le fleuve Congo',
      en: 'Decarbonising river transport on the Congo River',
    },
    slug: 'decarboner-transport-fluvial-congo',
    summary: {
      fr: "Le fleuve porte l'essentiel du fret de la région ; ses moteurs hors-bord sont parmi les plus polluants du continent.",
    },
    objectives: {
      fr: "Présenter les premiers résultats du remplacement des moteurs deux-temps par des motorisations électriques sur trois axes fluviaux, et le modèle de financement associé.",
    },
    presentation: {
      fr: "Le transport fluvial assure la majorité des échanges entre Kinshasa et l'amont du fleuve. Le parc de pirogues motorisées fonctionne avec des moteurs deux-temps consommant un mélange de qualité médiocre. Le projet a équipé quarante embarcations de motorisations électriques rechargées par des stations solaires en berge. La session présentera les consommations mesurées, l'acceptation par les piroguiers et l'équation économique, encore fragile sans subvention.",
    },
    outcomes: {
      fr: "Un cahier des charges reproductible pour les stations de recharge en berge et une estimation des économies de carburant par embarcation.",
    },
    audience: {
      fr: "Autorités portuaires, bailleurs, opérateurs de transport fluvial, collectivités riveraines.",
    },
    format: 'in_person',
    category: 'field_project',
    country: COUNTRY.cd,
    preferredStart: '2027-11-17T14:00:00-03:00',
    preferredEnd: '2027-11-17T15:30:00-03:00',
    duration: 90,
    status: 'submitted',
    submittedAt: '2026-08-04T09:12:00Z',
    createdAt: '2026-07-29T15:30:00Z',
    updatedAt: '2026-08-04T09:12:00Z',
    viewCount: 12,
  }),
  proposal({
    id: PROPOSAL.observatoireCdn,
    ref: 29,
    organization: ORG.cudcm,
    submittedBy: PERSON.gagnon,
    contact: PERSON.lemoine,
    title: {
      fr: 'Un observatoire francophone des contributions déterminées au niveau national',
      en: 'A French-speaking observatory of nationally determined contributions',
    },
    slug: 'observatoire-francophone-cdn',
    summary: {
      fr: "Suivre, en français, ce que les États francophones ont promis et ce qu'ils ont fait.",
    },
    objectives: {
      fr: "Lancer publiquement l'observatoire, en exposer la méthode de notation et recueillir les critiques des délégations concernées avant la mise en ligne définitive.",
    },
    presentation: {
      fr: "Les contributions déterminées au niveau national sont publiées dans des formats hétérogènes, souvent en anglais, rarement comparables. L'observatoire les traduit dans une grille commune : cibles chiffrées, année de référence, conditionnalité au financement international, dispositif de suivi. La session présentera la méthode, les premiers résultats sur douze pays francophones, et les limites assumées de l'exercice.",
    },
    outcomes: {
      fr: "Mise en ligne de la base et engagement de quatre États à en corriger les données les concernant.",
    },
    audience: { fr: "Négociateurs, points focaux CCNUCC, chercheurs, journalistes." },
    format: 'hybrid',
    category: 'results_sharing',
    languages: ['fr', 'en'],
    country: COUNTRY.ca,
    preferredStart: '2027-11-18T10:00:00-03:00',
    preferredEnd: '2027-11-18T11:30:00-03:00',
    duration: 90,
    status: 'submitted',
    submittedAt: '2026-08-06T13:40:00Z',
    createdAt: '2026-07-30T10:10:00Z',
    updatedAt: '2026-08-06T13:40:00Z',
    viewCount: 7,
  }),
  proposal({
    id: PROPOSAL.pmeAgroalimentaires,
    ref: 30,
    organization: ORG.verdeo,
    submittedBy: PERSON.moreau,
    title: {
      fr: 'Financer la transition des PME agroalimentaires francophones',
      en: 'Financing the transition of French-speaking agri-food SMEs',
    },
    slug: 'financer-transition-pme-agroalimentaires',
    objectives: {
      fr: "Confronter trois dispositifs de financement de la sobriété énergétique dans l'agroalimentaire et dire lequel a effectivement décaissé.",
    },
    presentation: {
      fr: "Les PME agroalimentaires représentent une part notable des consommations industrielles d'énergie au Maghreb et en Afrique de l'Ouest. Trois dispositifs les ciblent : lignes de crédit bancaires bonifiées, subvention à l'audit énergétique, tiers-financement par une société de services. La session compare les montants réellement décaissés, les délais et les taux de défaut.",
    },
    audience: { fr: "Banques de développement, ministères de l'industrie, fédérations professionnelles." },
    format: 'online',
    category: 'best_practices',
    country: COUNTRY.fr,
    duration: 60,
    status: 'submitted',
    submittedAt: '2026-08-08T08:20:00Z',
    createdAt: '2026-08-01T14:00:00Z',
    updatedAt: '2026-08-08T08:20:00Z',
    viewCount: 4,
  }),
  proposal({
    id: PROPOSAL.savoirsAutochtones,
    ref: 31,
    organization: ORG.roac,
    submittedBy: PERSON.mbayeNdiaye,
    contact: PERSON.sowFall,
    title: {
      fr: 'Savoirs locaux et gestion communautaire des forêts littorales',
      en: 'Local knowledge and community management of coastal forests',
    },
    slug: 'savoirs-locaux-forets-littorales',
    summary: {
      fr: "Ce que les calendriers de coupe traditionnels disent de la régénération des mangroves, et ce que les plans d'aménagement en ignorent.",
    },
    objectives: {
      fr: "Documenter la convergence entre règles coutumières de prélèvement et rythmes de régénération observés, et proposer leur reconnaissance dans les plans d'aménagement.",
    },
    presentation: {
      fr: "Dans quatre estuaires d'Afrique de l'Ouest, les règles coutumières de prélèvement du bois de mangrove reposent sur des calendriers et des interdits de zone. Le suivi écologique conduit sur six ans montre que ces règles produisent des cycles de régénération comparables à ceux des aires protégées, à un coût de gestion très inférieur. La session propose d'en tirer les conséquences réglementaires.",
    },
    outcomes: { fr: "Une note de position adressée aux directions des eaux et forêts des quatre pays." },
    audience: { fr: "Administrations forestières, ONG de conservation, chercheurs, organisations communautaires." },
    format: 'in_person',
    category: 'concertation',
    country: COUNTRY.sn,
    preferredStart: '2027-11-13T15:00:00-03:00',
    preferredEnd: '2027-11-13T16:30:00-03:00',
    duration: 90,
    status: 'submitted',
    submittedAt: '2026-08-10T11:05:00Z',
    createdAt: '2026-08-02T09:45:00Z',
    updatedAt: '2026-08-10T11:05:00Z',
    viewCount: 9,
  }),
  proposal({
    id: PROPOSAL.hydrogeneVert,
    ref: 32,
    organization: ORG.imre,
    submittedBy: PERSON.elFassi,
    title: {
      fr: "Hydrogène vert en Afrique du Nord : promesse ou mirage ?",
      en: 'Green hydrogen in North Africa: promise or mirage?',
    },
    slug: 'hydrogene-vert-afrique-du-nord',
    summary: {
      fr: "Les projets annoncés mobilisent une ressource en eau que la région n'a pas.",
    },
    objectives: {
      fr: "Mettre en regard les volumes d'eau nécessaires aux projets d'hydrogène annoncés et les disponibilités réelles des bassins concernés.",
    },
    presentation: {
      fr: "Les projets d'hydrogène vert annoncés au Maghreb totalisent plusieurs gigawatts d'électrolyse. À production nominale, leurs besoins en eau déminéralisée entrent en concurrence directe avec l'irrigation et l'eau potable, dans des bassins déjà déficitaires. La session présente le bilan hydrique de trois projets et les conditions — dessalement adossé, réutilisation d'eaux usées traitées — auxquelles ils resteraient soutenables.",
    },
    outcomes: { fr: "Une grille d'analyse hydrique utilisable par les autorités qui instruisent ces projets." },
    audience: { fr: "Ministères de l'énergie et de l'eau, développeurs de projets, agences de bassin." },
    format: 'hybrid',
    category: 'results_sharing',
    languages: ['fr', 'en'],
    country: COUNTRY.ma,
    preferredStart: '2027-11-11T11:00:00-03:00',
    preferredEnd: '2027-11-11T12:30:00-03:00',
    duration: 90,
    status: 'submitted',
    submittedAt: '2026-08-12T15:25:00Z',
    createdAt: '2026-08-05T08:30:00Z',
    updatedAt: '2026-08-12T15:25:00Z',
    viewCount: 15,
  }),
  proposal({
    id: PROPOSAL.sobrieteEnergetique,
    ref: 33,
    organization: ORG.osedSigle,
    submittedBy: PERSON.compaore,
    title: {
      fr: 'Sobriété énergétique : les campagnes qui changent vraiment les usages',
      en: 'Energy sufficiency: campaigns that actually change behaviour',
    },
    slug: 'sobriete-energetique-campagnes',
    summary: {
      fr: "Quatre campagnes de sensibilisation, deux qui ont modifié les consommations, deux qui n'ont rien changé.",
    },
    objectives: {
      fr: "Comparer quatre campagnes de sobriété conduites au Sahel et identifier ce qui distingue celles dont l'effet est mesurable.",
    },
    presentation: {
      fr: "Les campagnes de sensibilisation à la sobriété énergétique sont rarement évaluées. Quatre d'entre elles ont fait l'objet d'un suivi de consommation sur échantillon apparié. Deux ont produit une baisse durable, deux n'ont eu aucun effet mesurable au-delà du premier mois. La session expose les protocoles, les résultats et les hypothèses explicatives.",
    },
    format: 'online',
    category: 'awareness',
    country: COUNTRY.bf,
    duration: 60,
    status: 'submitted',
    submittedAt: '2026-08-14T10:00:00Z',
    createdAt: '2026-08-07T16:20:00Z',
    updatedAt: '2026-08-14T10:00:00Z',
    viewCount: 3,
  }),

  // --- Sorties de parcours --------------------------------------------------

  proposal({
    // RETIRÉE par l'organisation : le financement du déplacement n'a pas été
    // obtenu. Le motif vient du soumissionnaire, pas de l'IFDD.
    id: PROPOSAL.energiesMarines,
    ref: 34,
    organization: ORG.cvdmf,
    submittedBy: PERSON.tranVanMinh,
    title: {
      fr: 'Coopération Sud-Sud sur les énergies marines renouvelables',
      en: 'South-South cooperation on marine renewable energy',
    },
    slug: 'cooperation-sud-sud-energies-marines',
    objectives: {
      fr: "Rapprocher les programmes d'énergie marine du delta du Mékong et du golfe de Guinée autour d'un transfert de savoir-faire.",
    },
    presentation: {
      fr: "Deux régions deltaïques travaillent séparément sur l'hydrolien de faible profondeur. La session devait organiser leur mise en relation et l'esquisse d'un programme d'échange d'ingénieurs.",
    },
    format: 'in_person',
    category: 'technological_innovation',
    country: COUNTRY.vn,
    duration: 90,
    status: 'withdrawn',
    submittedAt: '2026-07-15T04:30:00Z',
    decidedAt: '2026-08-09T03:15:00Z',
    decisionReason:
      "Retrait à l'initiative de l'organisation : le financement du déplacement de la délégation n'a pas été obtenu. Un dépôt est envisagé pour l'édition suivante.",
    decidedBy: PERSON.tranVanMinh,
    createdAt: '2026-07-08T05:00:00Z',
    updatedAt: '2026-08-09T03:15:00Z',
    viewCount: 21,
  }),
  proposal({
    // ANNULÉE par l'IFDD : le format demandé ne tient pas dans le pavillon. Le
    // motif est obligatoire — la transition serait refusée sans lui.
    id: PROPOSAL.salonSolutions,
    ref: 35,
    organization: ORG.verdeo,
    submittedBy: PERSON.moreau,
    title: {
      fr: 'Salon des solutions climatiques francophones',
      en: 'Francophone climate solutions fair',
    },
    slug: 'salon-solutions-climatiques',
    objectives: {
      fr: "Réunir une trentaine d'exposants sur deux jours au sein du pavillon.",
    },
    presentation: {
      fr: "Le projet prévoyait un espace d'exposition permanent avec stands, démonstrations et rendez-vous d'affaires, occupant le pavillon deux jours pleins.",
    },
    format: 'in_person',
    category: 'technological_innovation',
    country: COUNTRY.fr,
    duration: 480,
    requestedSessions: 4,
    constraints: "Occupation continue de l'espace principal sur deux journées.",
    status: 'cancelled',
    submittedAt: '2026-07-20T08:00:00Z',
    decidedAt: '2026-08-11T14:45:00Z',
    decisionReason:
      "Format incompatible avec les moyens du pavillon : un seul stand, deux salles, et une programmation partagée entre une trentaine d'organisations. Une présentation de 90 minutes reste possible et a été proposée à l'organisation.",
    decidedBy: PERSON.bakayoko,
    createdAt: '2026-07-14T11:30:00Z',
    updatedAt: '2026-08-11T14:45:00Z',
    viewCount: 33,
  }),
] satisfies Proposal[]
