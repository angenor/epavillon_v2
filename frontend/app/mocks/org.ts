/**
 * Données simulées du schéma `org` — organisations, dénominations, domaines,
 * doublons présumés.
 *
 * LE DOUBLON EST LA DONNÉE LA PLUS IMPORTANTE DE CE FICHIER. L'Observatoire du
 * Sahel pour l'énergie durable y figure DEUX FOIS : une fois sous son nom
 * complet avec son sigle (`ORG.osed`), une fois sous son seul sigle
 * (`ORG.osedSigle`), saisi par un membre qui n'a pas trouvé la fiche existante.
 * Les deux fiches sont actives, portent le même domaine de messagerie, le même
 * pays, et ont chacune des membres et des propositions. C'est exactement la
 * situation que la v1 laissait s'installer, et ce que doivent traiter l'écran de
 * rattachement (A2) et l'écran de fusion (A11) — sans ce cas, ces deux écrans se
 * développeraient à l'aveugle.
 *
 * Les organisations sont FICTIVES, à l'exception de l'IFDD, organisation pivot
 * reprise telle quelle de `docs/database/900_seed.sql`.
 */

import type {
  DuplicateCandidate,
  Organization,
  OrganizationDomain,
  OrganizationName,
  OrganizationReference,
} from '~/types/org'
import { COUNTRY, DUPLICATE, ORG, ORG_DOMAIN, ORG_NAME, PERSON } from './ids'

// ---------------------------------------------------------------------------
// Organisations
// ---------------------------------------------------------------------------

export const organizations = [
  {
    id: ORG.ifdd,
    legal_name: 'Institut de la Francophonie pour le développement durable',
    legal_name_normalized: 'institut de la francophonie pour le developpement durable',
    acronym: 'IFDD',
    acronym_normalized: 'ifdd',
    slug: 'ifdd',
    organization_type_code: 'international_organization',
    country_id: COUNTRY.ca,
    city: 'Québec',
    description: {
      fr: "Organe subsidiaire de l'Organisation internationale de la Francophonie, l'IFDD accompagne les pays francophones sur le développement durable, l'énergie et le climat.",
      en: 'Subsidiary body of the Organisation internationale de la Francophonie, supporting French-speaking countries on sustainable development, energy and climate.',
    },
    website: 'https://www.ifdd.francophonie.org',
    contact_email: 'ifdd@francophonie.org',
    contact_phone: '+1 418 692 5727',
    status: 'active',
    merged_into_id: null,
    merged_at: null,
    verified_at: '2026-01-12T09:00:00Z',
    verified_by: PERSON.adminPivot,
    trust_score: 100,
    created_by: null,
    created_at: '2026-01-12T09:00:00Z',
    updated_at: '2026-01-12T09:00:00Z',
  },
  {
    id: ORG.roac,
    legal_name: "Réseau ouest-africain pour l'adaptation côtière",
    legal_name_normalized: 'reseau ouest africain pour l adaptation cotiere',
    acronym: 'ROAC',
    acronym_normalized: 'roac',
    slug: 'reseau-ouest-africain-adaptation-cotiere',
    organization_type_code: 'ngo_association',
    country_id: COUNTRY.sn,
    city: 'Dakar',
    description: {
      fr: "Réseau d'associations littorales de huit pays d'Afrique de l'Ouest, engagé sur l'érosion côtière, la submersion marine et la relocalisation des villages de pêcheurs.",
    },
    website: 'https://www.roac-afrique.org',
    contact_email: 'contact@roac-afrique.org',
    contact_phone: '+221 33 869 12 40',
    status: 'active',
    merged_into_id: null,
    merged_at: null,
    verified_at: '2026-03-04T14:20:00Z',
    verified_by: PERSON.bakayoko,
    trust_score: 88,
    created_by: PERSON.sowFall,
    created_at: '2026-02-19T11:05:00Z',
    updated_at: '2026-06-22T08:40:00Z',
  },
  {
    id: ORG.osed,
    legal_name: "Observatoire du Sahel pour l'énergie durable",
    legal_name_normalized: 'observatoire du sahel pour l energie durable',
    acronym: 'OSED',
    acronym_normalized: 'osed',
    slug: 'observatoire-sahel-energie-durable',
    organization_type_code: 'academic_research',
    country_id: COUNTRY.bf,
    city: 'Ouagadougou',
    description: {
      fr: "Centre de recherche appliquée sur l'accès à l'énergie en zone sahélienne : mini-réseaux solaires, cuisson propre, planification énergétique territoriale.",
    },
    website: 'https://www.osed-sahel.org',
    contact_email: 'direction@osed-sahel.org',
    contact_phone: '+226 25 36 08 11',
    status: 'active',
    merged_into_id: null,
    merged_at: null,
    verified_at: '2026-03-11T09:15:00Z',
    verified_by: PERSON.bakayoko,
    trust_score: 84,
    created_by: PERSON.ouedraogo,
    created_at: '2026-02-24T16:30:00Z',
    updated_at: '2026-05-30T10:10:00Z',
  },
  {
    // DOUBLON MANIFESTE de `ORG.osed`. Même entité, même domaine de messagerie,
    // même ville : la fiche a été créée par une collaboratrice qui a cherché
    // « OSED » sans que la recherche de la v1 rapproche le sigle du nom complet.
    id: ORG.osedSigle,
    legal_name: 'OSED',
    legal_name_normalized: 'osed',
    acronym: null,
    acronym_normalized: null,
    slug: 'osed',
    organization_type_code: 'ngo_association',
    country_id: COUNTRY.bf,
    city: 'Ouagadougou',
    description: null,
    website: null,
    contact_email: 'projets@osed-sahel.org',
    contact_phone: null,
    status: 'active',
    merged_into_id: null,
    merged_at: null,
    verified_at: null,
    verified_by: null,
    trust_score: 32,
    created_by: PERSON.compaore,
    created_at: '2026-06-11T13:45:00Z',
    updated_at: '2026-06-11T13:45:00Z',
  },
  {
    id: ORG.anteb,
    legal_name: "Agence nationale de la transition écologique du Bénin",
    legal_name_normalized: 'agence nationale de la transition ecologique du benin',
    acronym: 'ANTEB',
    acronym_normalized: 'anteb',
    slug: 'agence-nationale-transition-ecologique-benin',
    organization_type_code: 'public_national_institution',
    country_id: COUNTRY.bj,
    city: 'Cotonou',
    description: {
      fr: "Établissement public chargé de la mise en œuvre de la contribution déterminée au niveau national et de la coordination des projets d'adaptation.",
    },
    website: 'https://www.anteb.bj',
    contact_email: 'secretariat@anteb.bj',
    contact_phone: '+229 21 30 55 02',
    status: 'active',
    merged_into_id: null,
    merged_at: null,
    verified_at: '2026-04-02T08:00:00Z',
    verified_by: PERSON.tremblay,
    trust_score: 95,
    created_by: PERSON.zinsou,
    created_at: '2026-03-28T07:50:00Z',
    updated_at: '2026-07-01T15:20:00Z',
  },
  {
    id: ORG.cofemac,
    legal_name: 'Coalition des femmes pour le climat en Afrique centrale',
    legal_name_normalized: 'coalition des femmes pour le climat en afrique centrale',
    acronym: 'COFEMAC',
    acronym_normalized: 'cofemac',
    slug: 'coalition-femmes-climat-afrique-centrale',
    organization_type_code: 'ngo_association',
    country_id: COUNTRY.cm,
    city: 'Yaoundé',
    description: {
      fr: "Coalition de coopératives et d'associations féminines du bassin du Congo : accès au foncier, transition juste, participation aux négociations.",
    },
    website: 'https://www.cofemac.org',
    contact_email: 'coordination@cofemac.org',
    contact_phone: '+237 6 99 41 08 22',
    status: 'active',
    merged_into_id: null,
    merged_at: null,
    verified_at: '2026-04-18T11:30:00Z',
    verified_by: PERSON.bakayoko,
    trust_score: 79,
    created_by: PERSON.ngoBassong,
    created_at: '2026-04-10T09:25:00Z',
    updated_at: '2026-06-05T14:00:00Z',
  },
  {
    id: ORG.imre,
    legal_name: "Institut méditerranéen de recherche sur l'eau",
    legal_name_normalized: 'institut mediterraneen de recherche sur l eau',
    acronym: 'IMRE',
    acronym_normalized: 'imre',
    slug: 'institut-mediterraneen-recherche-eau',
    organization_type_code: 'academic_research',
    country_id: COUNTRY.ma,
    city: 'Rabat',
    description: {
      fr: "Laboratoire universitaire consacré à la gestion de la rareté de l'eau, au dessalement sobre en énergie et à la réutilisation des eaux usées traitées.",
    },
    website: 'https://www.imre.ma',
    contact_email: 'contact@imre.ma',
    contact_phone: '+212 5 37 77 21 09',
    status: 'active',
    merged_into_id: null,
    merged_at: null,
    verified_at: '2026-04-25T10:00:00Z',
    verified_by: PERSON.tremblay,
    trust_score: 82,
    created_by: PERSON.elFassi,
    created_at: '2026-04-20T13:15:00Z',
    updated_at: '2026-04-25T10:00:00Z',
  },
  {
    id: ORG.fhrc,
    legal_name: 'Fonds haïtien pour la résilience communautaire',
    legal_name_normalized: 'fonds haitien pour la resilience communautaire',
    acronym: 'FHRC',
    acronym_normalized: 'fhrc',
    slug: 'fonds-haitien-resilience-communautaire',
    organization_type_code: 'ngo_association',
    country_id: COUNTRY.ht,
    city: 'Port-au-Prince',
    description: {
      fr: "Fonds de proximité qui finance des micro-projets d'adaptation portés par des organisations de quartier et des coopératives agricoles.",
    },
    website: 'https://www.fhrc-haiti.org',
    contact_email: 'info@fhrc-haiti.org',
    contact_phone: null,
    status: 'active',
    merged_into_id: null,
    merged_at: null,
    verified_at: null,
    verified_by: null,
    trust_score: 61,
    created_by: PERSON.josephPierre,
    created_at: '2026-05-06T17:40:00Z',
    updated_at: '2026-05-06T17:40:00Z',
  },
  {
    id: ORG.cudcm,
    legal_name: 'Chaire universitaire de droit climatique de Montréal',
    legal_name_normalized: 'chaire universitaire de droit climatique de montreal',
    acronym: 'CUDCM',
    acronym_normalized: 'cudcm',
    slug: 'chaire-droit-climatique-montreal',
    organization_type_code: 'academic_research',
    country_id: COUNTRY.ca,
    city: 'Montréal',
    description: {
      fr: "Chaire de recherche consacrée au contentieux climatique, à la responsabilité des États et à la transposition des accords multilatéraux en droit interne.",
      en: 'Research chair on climate litigation, state responsibility and the domestic transposition of multilateral agreements.',
    },
    website: 'https://www.cudcm.ca',
    contact_email: 'chaire@cudcm.ca',
    contact_phone: '+1 514 343 60 11',
    status: 'active',
    merged_into_id: null,
    merged_at: null,
    verified_at: '2026-05-14T12:00:00Z',
    verified_by: PERSON.tremblay,
    trust_score: 90,
    created_by: PERSON.gagnon,
    created_at: '2026-05-11T08:05:00Z',
    updated_at: '2026-05-14T12:00:00Z',
  },
  {
    id: ORG.cvdmf,
    legal_name: 'Consortium des villes durables du Mékong francophone',
    legal_name_normalized: 'consortium des villes durables du mekong francophone',
    acronym: 'CVDMF',
    acronym_normalized: 'cvdmf',
    slug: 'consortium-villes-durables-mekong',
    organization_type_code: 'regional_organization',
    country_id: COUNTRY.vn,
    city: 'Hô Chi Minh-Ville',
    description: {
      fr: "Consortium de collectivités du delta du Mékong : plans de refroidissement urbain, gestion des inondations, transports sobres en carbone.",
    },
    website: 'https://www.cvdmf.org',
    contact_email: 'secretariat@cvdmf.org',
    contact_phone: '+84 28 39 10 44 27',
    status: 'active',
    merged_into_id: null,
    merged_at: null,
    verified_at: '2026-05-22T06:30:00Z',
    verified_by: PERSON.bakayoko,
    trust_score: 76,
    created_by: PERSON.tranVanMinh,
    created_at: '2026-05-18T04:10:00Z',
    updated_at: '2026-05-22T06:30:00Z',
  },
  {
    id: ORG.verdeo,
    legal_name: 'Verdéo Solutions',
    legal_name_normalized: 'verdeo solutions',
    acronym: null,
    acronym_normalized: null,
    slug: 'verdeo-solutions',
    organization_type_code: 'private_sector',
    country_id: COUNTRY.fr,
    city: 'Lyon',
    description: {
      fr: "Bureau d'études spécialisé dans la comptabilité carbone des collectivités et l'accompagnement des PME agroalimentaires vers la sobriété énergétique.",
    },
    website: 'https://www.verdeo-solutions.fr',
    contact_email: 'contact@verdeo-solutions.fr',
    contact_phone: '+33 4 72 11 08 90',
    status: 'active',
    merged_into_id: null,
    merged_at: null,
    verified_at: null,
    verified_by: null,
    trust_score: 54,
    created_by: PERSON.moreau,
    created_at: '2026-06-02T09:55:00Z',
    updated_at: '2026-06-02T09:55:00Z',
  },
  {
    id: ORG.ujfc,
    legal_name: 'Union des jeunes francophones pour le climat',
    legal_name_normalized: 'union des jeunes francophones pour le climat',
    acronym: 'UJFC',
    acronym_normalized: 'ujfc',
    slug: 'union-jeunes-francophones-climat',
    organization_type_code: 'ngo_association',
    country_id: COUNTRY.ci,
    city: 'Abidjan',
    description: {
      fr: "Association de jeunesse présente dans dix-sept pays : formation à la négociation, plaidoyer et représentation des jeunes délégations.",
    },
    website: 'https://www.ujfc.org',
    contact_email: 'bureau@ujfc.org',
    contact_phone: '+225 27 22 41 05 63',
    status: 'active',
    merged_into_id: null,
    merged_at: null,
    verified_at: '2026-06-15T15:45:00Z',
    verified_by: PERSON.perretAdmin,
    trust_score: 71,
    created_by: PERSON.koffi,
    created_at: '2026-06-09T10:20:00Z',
    updated_at: '2026-06-15T15:45:00Z',
  },
  {
    id: ORG.mvoi,
    legal_name: "Média Vert Océan Indien",
    legal_name_normalized: 'media vert ocean indien',
    acronym: 'MVOI',
    acronym_normalized: 'mvoi',
    slug: 'media-vert-ocean-indien',
    organization_type_code: 'media',
    country_id: COUNTRY.mg,
    city: 'Antananarivo',
    description: {
      fr: "Rédaction indépendante consacrée à l'environnement dans l'océan Indien, avec un réseau de correspondants dans cinq îles.",
    },
    website: 'https://www.mediavert-oi.mg',
    contact_email: 'redaction@mediavert-oi.mg',
    contact_phone: null,
    status: 'candidate',
    merged_into_id: null,
    merged_at: null,
    verified_at: null,
    verified_by: null,
    trust_score: 38,
    created_by: PERSON.rakotomalala,
    created_at: '2026-07-28T06:15:00Z',
    updated_at: '2026-07-28T06:15:00Z',
  },
] satisfies Organization[]

// ---------------------------------------------------------------------------
// Dénominations
//
// Le nom légal et le sigle sont recopiés ici par trigger : ces lignes existent
// donc en base sans qu'aucun écran ne les écrive. Les autres — traductions,
// anciens noms, fautes d'orthographe rencontrées à l'inscription — sont ce qui
// permet à une recherche « IEPF » ou « OSED » de ramener la bonne fiche.
// ---------------------------------------------------------------------------

/** Raccourci d'écriture : une dénomination avec ses valeurs par défaut. */
function name(
  n: number,
  organization_id: string,
  value: string,
  kind: OrganizationName['kind'],
  options: Partial<Pick<OrganizationName, 'locale' | 'is_confirmed' | 'created_by' | 'created_at'>> = {},
): OrganizationName {
  return {
    id: ORG_NAME(n),
    organization_id,
    name: value,
    name_normalized: null,
    kind,
    locale: options.locale ?? null,
    is_confirmed: options.is_confirmed ?? true,
    created_by: options.created_by ?? null,
    created_at: options.created_at ?? '2026-03-04T14:20:00Z',
  }
}

export const organizationNames = [
  // IFDD — reprise de `900_seed.sql`
  name(1, ORG.ifdd, 'Institut de la Francophonie pour le développement durable', 'legal'),
  name(2, ORG.ifdd, 'IFDD', 'acronym'),
  name(3, ORG.ifdd, 'Institut de la Francophonie pour le Developpement Durable', 'misspelling'),
  name(4, ORG.ifdd, 'Institute of the Francophonie for Sustainable Development', 'translation', { locale: 'en' }),
  name(5, ORG.ifdd, "Institut de l'énergie et de l'environnement de la Francophonie", 'former'),
  name(6, ORG.ifdd, 'IEPF', 'former'),

  name(10, ORG.roac, "Réseau ouest-africain pour l'adaptation côtière", 'legal'),
  name(11, ORG.roac, 'ROAC', 'acronym'),
  name(12, ORG.roac, 'West African Coastal Adaptation Network', 'translation', { locale: 'en', is_confirmed: false }),
  name(13, ORG.roac, 'Réseau ouest africain adaptation cotière', 'misspelling', { is_confirmed: false }),

  // Les deux fiches en doublon portent des dénominations qui se recouvrent :
  // c'est ce recouvrement que `find_similar_organizations()` fait remonter.
  name(20, ORG.osed, "Observatoire du Sahel pour l'énergie durable", 'legal'),
  name(21, ORG.osed, 'OSED', 'acronym'),
  name(22, ORG.osed, 'Observatoire sahélien de l’énergie durable', 'former', { is_confirmed: false }),
  name(23, ORG.osedSigle, 'OSED', 'legal'),
  name(24, ORG.osedSigle, "Observatoire du Sahel énergie durable", 'short', { is_confirmed: false, created_by: PERSON.compaore, created_at: '2026-06-11T13:45:00Z' }),

  name(30, ORG.anteb, "Agence nationale de la transition écologique du Bénin", 'legal'),
  name(31, ORG.anteb, 'ANTEB', 'acronym'),
  name(32, ORG.anteb, "Agence béninoise pour l'environnement", 'former'),

  name(40, ORG.cofemac, 'Coalition des femmes pour le climat en Afrique centrale', 'legal'),
  name(41, ORG.cofemac, 'COFEMAC', 'acronym'),

  name(50, ORG.imre, "Institut méditerranéen de recherche sur l'eau", 'legal'),
  name(51, ORG.imre, 'IMRE', 'acronym'),
  name(52, ORG.imre, 'Mediterranean Water Research Institute', 'translation', { locale: 'en' }),

  name(60, ORG.fhrc, 'Fonds haïtien pour la résilience communautaire', 'legal'),
  name(61, ORG.fhrc, 'FHRC', 'acronym'),

  name(70, ORG.cudcm, 'Chaire universitaire de droit climatique de Montréal', 'legal'),
  name(71, ORG.cudcm, 'CUDCM', 'acronym'),
  name(72, ORG.cudcm, 'Montreal Chair in Climate Law', 'translation', { locale: 'en' }),

  name(80, ORG.cvdmf, 'Consortium des villes durables du Mékong francophone', 'legal'),
  name(81, ORG.cvdmf, 'CVDMF', 'acronym'),

  name(90, ORG.verdeo, 'Verdéo Solutions', 'legal'),
  name(91, ORG.verdeo, 'Verdeo', 'short', { is_confirmed: false }),

  name(100, ORG.ujfc, 'Union des jeunes francophones pour le climat', 'legal'),
  name(101, ORG.ujfc, 'UJFC', 'acronym'),

  name(110, ORG.mvoi, 'Média Vert Océan Indien', 'legal'),
  name(111, ORG.mvoi, 'MVOI', 'acronym'),
] satisfies OrganizationName[]

// ---------------------------------------------------------------------------
// Domaines de messagerie
//
// Le signal de dédoublonnage le plus fiable : les deux fiches OSED partagent
// `osed-sahel.org`. Aucun domaine générique (gmail, outlook) n'apparaît ici —
// ils vivent dans `org.public_email_domains` et sont écartés de tout
// rapprochement.
// ---------------------------------------------------------------------------

export const organizationDomains = [
  {
    id: ORG_DOMAIN(1),
    organization_id: ORG.ifdd,
    domain: 'ifdd.francophonie.org',
    verified_at: '2026-01-12T09:00:00Z',
    verification_method: 'manual',
    auto_join: true,
    created_at: '2026-01-12T09:00:00Z',
  },
  {
    id: ORG_DOMAIN(2),
    organization_id: ORG.ifdd,
    domain: 'francophonie.org',
    verified_at: '2026-01-12T09:00:00Z',
    verification_method: 'manual',
    auto_join: false,
    created_at: '2026-01-12T09:00:00Z',
  },
  {
    id: ORG_DOMAIN(3),
    organization_id: ORG.roac,
    domain: 'roac-afrique.org',
    verified_at: '2026-03-04T14:20:00Z',
    verification_method: 'dns_txt',
    auto_join: true,
    created_at: '2026-02-19T11:05:00Z',
  },
  {
    id: ORG_DOMAIN(4),
    organization_id: ORG.osed,
    domain: 'osed-sahel.org',
    verified_at: '2026-03-11T09:15:00Z',
    verification_method: 'email_challenge',
    auto_join: true,
    created_at: '2026-02-24T16:30:00Z',
  },
  {
    // Même domaine que la fiche précédente : preuve matérielle du doublon.
    id: ORG_DOMAIN(5),
    organization_id: ORG.osedSigle,
    domain: 'osed-sahel.org',
    verified_at: null,
    verification_method: null,
    auto_join: false,
    created_at: '2026-06-11T13:45:00Z',
  },
  {
    id: ORG_DOMAIN(6),
    organization_id: ORG.anteb,
    domain: 'anteb.bj',
    verified_at: '2026-04-02T08:00:00Z',
    verification_method: 'dns_txt',
    auto_join: true,
    created_at: '2026-03-28T07:50:00Z',
  },
  {
    id: ORG_DOMAIN(7),
    organization_id: ORG.cofemac,
    domain: 'cofemac.org',
    verified_at: '2026-04-18T11:30:00Z',
    verification_method: 'email_challenge',
    auto_join: true,
    created_at: '2026-04-10T09:25:00Z',
  },
  {
    id: ORG_DOMAIN(8),
    organization_id: ORG.imre,
    domain: 'imre.ma',
    verified_at: '2026-04-25T10:00:00Z',
    verification_method: 'dns_txt',
    auto_join: true,
    created_at: '2026-04-20T13:15:00Z',
  },
  {
    id: ORG_DOMAIN(9),
    organization_id: ORG.fhrc,
    domain: 'fhrc-haiti.org',
    verified_at: null,
    verification_method: null,
    auto_join: false,
    created_at: '2026-05-06T17:40:00Z',
  },
  {
    id: ORG_DOMAIN(10),
    organization_id: ORG.cudcm,
    domain: 'cudcm.ca',
    verified_at: '2026-05-14T12:00:00Z',
    verification_method: 'dns_txt',
    auto_join: true,
    created_at: '2026-05-11T08:05:00Z',
  },
  {
    id: ORG_DOMAIN(11),
    organization_id: ORG.cvdmf,
    domain: 'cvdmf.org',
    verified_at: '2026-05-22T06:30:00Z',
    verification_method: 'email_challenge',
    auto_join: true,
    created_at: '2026-05-18T04:10:00Z',
  },
  {
    id: ORG_DOMAIN(12),
    organization_id: ORG.verdeo,
    domain: 'verdeo-solutions.fr',
    verified_at: null,
    verification_method: null,
    auto_join: false,
    created_at: '2026-06-02T09:55:00Z',
  },
  {
    id: ORG_DOMAIN(13),
    organization_id: ORG.ujfc,
    domain: 'ujfc.org',
    verified_at: '2026-06-15T15:45:00Z',
    verification_method: 'email_challenge',
    auto_join: true,
    created_at: '2026-06-09T10:20:00Z',
  },
  {
    id: ORG_DOMAIN(14),
    organization_id: ORG.mvoi,
    domain: 'mediavert-oi.mg',
    verified_at: null,
    verification_method: null,
    auto_join: false,
    created_at: '2026-07-28T06:15:00Z',
  },
] satisfies OrganizationDomain[]

// ---------------------------------------------------------------------------
// Doublons présumés
//
// Deux paires, volontairement de natures différentes : l'une à fusionner, qui
// attend une décision ; l'autre déjà tranchée comme « distinctes », parce que
// deux organisations de recherche ne sont pas la même pour avoir des noms
// voisins. Sans ce second cas, l'écran de fusion (A11) donnerait à croire que
// tout candidat est un doublon.
// ---------------------------------------------------------------------------

export const duplicateCandidates = [
  {
    id: DUPLICATE.osed,
    left_id: ORG.osed,
    right_id: ORG.osedSigle,
    score: 94.5,
    reasons: ['acronym_match', 'shared_domain', 'same_country', 'name_similarity'],
    detected_at: '2026-06-11T13:50:00Z',
    reviewed_at: null,
    reviewed_by: null,
    decision: null,
  },
  {
    // `left_id` est toujours inférieur à `right_id` : la contrainte
    // `ck_duplicate_candidates_ordered` interdit d'enregistrer deux fois la même
    // paire dans les deux sens.
    id: DUPLICATE.imreCudcm,
    left_id: ORG.imre,
    right_id: ORG.cudcm,
    score: 61.0,
    reasons: ['name_similarity'],
    detected_at: '2026-05-14T12:05:00Z',
    reviewed_at: '2026-05-15T09:30:00Z',
    reviewed_by: PERSON.bakayoko,
    decision: 'distinct',
  },
] satisfies DuplicateCandidate[]

// ---------------------------------------------------------------------------
// Registre des références — `org.organization_references`
//
// CE N'EST PAS UNE DONNÉE DE DÉMONSTRATION : c'est le registre que chaque module
// alimente lors de sa migration, recopié tel quel des huit fichiers SQL qui y
// insèrent (040 § 6, 050 § 8, 060 § 7, 070, 075, 080, 090, 125). La fusion le
// PARCOURT plutôt que d'énumérer des tables — d'où l'écran de fusion (A11), qui
// chiffre son transfert ligne à ligne à partir de lui et n'aura rien à changer
// le jour où un module de plus s'y déclare.
//
// `dedupe_on` porte les colonnes formant une unicité AVEC la colonne
// d'organisation : les lignes en conflit côté source sont supprimées avant la
// bascule, sans quoi l'unicité ferait échouer la fusion entière.
// ---------------------------------------------------------------------------

export const organizationReferences = [
  // 040 § 6 — le module lui-même
  { ref_schema: 'org', ref_table: 'organization_names', ref_column: 'organization_id', strategy: 'reassign', dedupe_on: [] },
  // `{domain}` : deux fiches en doublon déclarent presque toujours le même
  // domaine — c'est le signal qui les a fait remonter. Sans dédoublonnage, la
  // fiche absorbante héritait de deux lignes pour `osed-sahel.org`.
  { ref_schema: 'org', ref_table: 'organization_domains', ref_column: 'organization_id', strategy: 'reassign', dedupe_on: ['domain'] },
  { ref_schema: 'org', ref_table: 'memberships', ref_column: 'organization_id', strategy: 'reassign', dedupe_on: ['person_id'] },
  { ref_schema: 'identity', ref_table: 'people', ref_column: 'primary_organization_id', strategy: 'reassign', dedupe_on: [] },
  // 050 § 8 — médias. Le quota est un RÉGLAGE, pas un patrimoine : il se supprime.
  { ref_schema: 'media', ref_table: 'assets', ref_column: 'owner_organization_id', strategy: 'reassign', dedupe_on: [] },
  { ref_schema: 'media', ref_table: 'storage_quotas', ref_column: 'organization_id', strategy: 'delete', dedupe_on: [] },
  // 060 § 7 — séries d'événements
  { ref_schema: 'event', ref_table: 'event_series', ref_column: 'organizer_organization_id', strategy: 'reassign', dedupe_on: [] },
  // 070 — propositions
  { ref_schema: 'programme', ref_table: 'proposals', ref_column: 'organization_id', strategy: 'reassign', dedupe_on: [] },
  { ref_schema: 'programme', ref_table: 'proposal_speakers', ref_column: 'organization_id', strategy: 'reassign', dedupe_on: [] },
  { ref_schema: 'programme', ref_table: 'proposal_organizations', ref_column: 'organization_id', strategy: 'reassign', dedupe_on: ['proposal_id'] },
  // 075 — séances et inscriptions
  { ref_schema: 'programme', ref_table: 'sessions', ref_column: 'organization_id', strategy: 'reassign', dedupe_on: [] },
  { ref_schema: 'programme', ref_table: 'registrations', ref_column: 'organization_id', strategy: 'reassign', dedupe_on: [] },
  { ref_schema: 'programme', ref_table: 'session_organizations', ref_column: 'organization_id', strategy: 'reassign', dedupe_on: ['session_id'] },
  // 080 — messages d'incident
  { ref_schema: 'live', ref_table: 'incidents', ref_column: 'organization_id', strategy: 'reassign', dedupe_on: [] },
  // 090 — publications
  { ref_schema: 'publication', ref_table: 'articles', ref_column: 'organization_id', strategy: 'reassign', dedupe_on: [] },
  { ref_schema: 'publication', ref_table: 'publishing_policies', ref_column: 'organization_id', strategy: 'delete', dedupe_on: [] },
  // 125 — formations
  { ref_schema: 'training', ref_table: 'trainings', ref_column: 'organizer_organization_id', strategy: 'reassign', dedupe_on: [] },
  { ref_schema: 'training', ref_table: 'enrollments', ref_column: 'organization_id', strategy: 'reassign', dedupe_on: [] },
] satisfies OrganizationReference[]
