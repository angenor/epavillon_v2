/**
 * Contrats du RATTACHEMENT À UNE ORGANISATION — ce qui circule entre l'écran A2
 * et l'API.
 *
 * Comme `types/auth.ts`, ces types ne décrivent aucune table : ils nomment les
 * requêtes et les réponses d'un écran. Les entités elles-mêmes sont dans
 * `types/org.ts`, dérivées de `040_organizations.sql`.
 *
 * L'ENJEU DE CET ÉCRAN EST LA QUALITÉ DU RÉFÉRENTIEL, pas le confort de saisie.
 * Le défaut n°1 de la v1 — deux fiches pour une même organisation, l'une créée
 * par qui cherchait le nom complet, l'autre par qui cherchait le sigle — se joue
 * ici et nulle part ailleurs. Trois choix de contrat en découlent :
 *
 *  · LA CRÉATION PORTE CE QU'ON A VU. `CreateOrganizationPayload` déclare
 *    `acknowledged_match_ids` : les fiches proches montrées à l'utilisateur au
 *    moment où il a insisté. L'API sait alors distinguer « personne n'a rien
 *    vu » de « on a montré, la personne a maintenu » — le second cas mérite une
 *    revue humaine, pas un refus.
 *  · REJOINDRE N'EST PAS ENTRER. Le résultat distingue `joined` de `pending` :
 *    une adhésion attend l'accord d'un référent, SAUF domaine vérifié en
 *    rattachement automatique (`organization_domains.auto_join`).
 *  · RIEN N'EST BLOQUÉ. Aucune issue ne dit « refusé parce qu'un doublon
 *    existe » : l'écran rend l'erreur visible, il ne l'empêche pas.
 */

import type {
  MembershipRole,
  Organization,
  OrganizationDomain,
  SimilarOrganization,
} from './org'
import type { CountryId, I18nText, OrganizationId, TaxonomyTermCode, Url, Uuid } from './shared'

// ---------------------------------------------------------------------------
// Recherche
// ---------------------------------------------------------------------------

/**
 * Score à partir duquel une correspondance est FORTE — `040_organizations.sql`
 * § 5 : « un score >= 85 déclenche un blocage doux côté interface plutôt qu'une
 * simple suggestion ». Le seuil est celui du modèle ; il est nommé ici pour que
 * l'écran ne le réinvente pas, et pour qu'il ne soit écrit qu'une fois.
 *
 * Ce qu'il vaut concrètement, mesuré sur la base : un sigle exact donne 125, un
 * ancien nom exact 100, un fragment de nom 51 à 60. Seule la première catégorie
 * franchit le seuil, et c'est l'intention.
 */
export const STRONG_MATCH_SCORE = 85

/** Paramètres de `org.find_similar_organizations(...)`. */
export interface OrganizationSearchQuery {
  /** Ce que la personne a tapé : nom complet, sigle, fragment. */
  name: string
  /** Pays déclaré au profil : bonus de 10 quand il coïncide. */
  country_id?: CountryId | null
  /** Adresse de la personne : son domaine vaut bonus de 40, sauf messagerie grand public. */
  email?: string | null
  /** Site saisi dans le formulaire de création — même usage que l'adresse. */
  website?: string | null
  limit?: number
}

/**
 * Ce que le domaine d'une adresse révèle. `null` quand le domaine est générique
 * (`org.public_email_domains`) ou qu'aucune fiche ne le porte : dans les deux
 * cas l'écran ne propose rien, il ne devine pas.
 */
export interface EmailDomainMatch {
  domain: string
  organization: Organization
  /** La ligne qui a produit la correspondance : `verified_at` et `auto_join`. */
  domain_record: OrganizationDomain
  /**
   * Le rattachement peut-il être immédiat ? Vrai seulement si le domaine est
   * VÉRIFIÉ **et** marqué `auto_join` — `ck_domain_autojoin_requires_verification`
   * garantit déjà que le second implique le premier ; on le teste quand même,
   * une donnée importée pouvant précéder la contrainte.
   */
  can_auto_join: boolean
  /** Adhésions actives : le « déjà 4 membres » qui rassure avant de cliquer. */
  member_count: number
}

// ---------------------------------------------------------------------------
// Rejoindre une organisation existante
// ---------------------------------------------------------------------------

export interface JoinOrganizationPayload {
  organization_id: OrganizationId
  /** Fonction occupée, telle que la personne la déclare. `memberships.job_title`. */
  job_title: string | null
}

/**
 * Issue d'une demande de rattachement.
 *
 * `pending` n'est PAS un échec : c'est le fonctionnement normal. Un référent
 * d'organisation accepte ou refuse (`memberships.status`), et l'écran doit le
 * dire clairement plutôt que de laisser croire que tout est réglé.
 */
export type JoinOrganizationResult =
  | { status: 'joined'; membership_id: Uuid; organization: Organization }
  | { status: 'pending'; membership_id: Uuid; organization: Organization }
  /** La personne est déjà membre, ou sa demande est déjà déposée. */
  | { status: 'already_member'; organization: Organization; membership_status: 'active' | 'pending' }

// ---------------------------------------------------------------------------
// Créer une organisation
// ---------------------------------------------------------------------------

/**
 * Les sept champs du prompt, et pas un de plus. `slug` est composé par l'API
 * (`platform.slugify`), `status` vaut `candidate` — une fiche créée depuis un
 * formulaire public n'est pas une fiche de référence tant que l'IFDD ne l'a pas
 * regardée.
 */
export interface CreateOrganizationPayload {
  legal_name: string
  acronym: string | null
  organization_type_code: TaxonomyTermCode
  country_id: CountryId
  city: string | null
  website: Url | null
  description: I18nText | null
  /** Fonction de la personne dans l'organisation qu'elle crée. */
  job_title: string | null
  /**
   * Fiches proches AFFICHÉES avant la création, et maintenues malgré tout.
   * Vide quand la recherche n'a rien ramené — la distinction compte pour la
   * revue : créer sans rien voir n'est pas la même faute que créer en sachant.
   */
  acknowledged_match_ids: OrganizationId[]
}

/**
 * Issue d'une création. Une seule forme de succès : la fiche est créée en
 * `candidate`, et la personne en devient le référent (`memberships.role =
 * 'manager'`, active — elle n'a personne pour l'approuver).
 *
 * `duplicate_blocked` n'existe pas, et c'est délibéré : la base refuse
 * seulement le doublon EXACT (`ux_organizations_name_country`), auquel cas
 * l'API renvoie `name_taken` avec la fiche en cause — de quoi la rejoindre.
 */
export type CreateOrganizationResult =
  | { status: 'created'; organization: Organization; membership_id: Uuid; role: MembershipRole }
  | { status: 'name_taken'; existing: SimilarOrganization }
