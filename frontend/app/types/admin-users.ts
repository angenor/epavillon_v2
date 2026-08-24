/**
 * UTILISATEURS ET RÔLES, BACK-OFFICE (A12) — contrats des écrans.
 *
 * Dérivé de `docs/database/030_identity.sql` : `people`, `accounts`,
 * `role_assignments`, `roles`, `permissions`, `role_permissions`,
 * `privacy_requests`, `current_consents`, et les deux fonctions qui décident de
 * tout — `effective_permissions()` et `administered_events()`. Aucun champ
 * inventé ; les deux seules colonnes qui manquaient ont été AJOUTÉES AU MODÈLE
 * avant d'écrire ce fichier (voir plus bas).
 *
 * ── LA PORTÉE EST LE SUJET DE CET ÉCRAN, PAS UN DÉTAIL DU FORMULAIRE ────────
 *
 * La v1 posait un ENUM de huit rôles GLOBAUX. Être « révisionniste » valait pour
 * toutes les COP, passées et à venir, et il n'existait aucun moyen de confier un
 * webinaire à son responsable sans lui ouvrir la plateforme entière. Le coût est
 * consigné dans l'en-tête de `030_identity.sql` : une page d'administration
 * séparée, développée dans l'urgence et en partie codée en dur, pour le seul
 * cycle PACO.
 *
 * En v2, c'est une attribution de rôle, et la portée en fait partie intégrante :
 *
 *   Administrateur                  role=admin        scope=global
 *   Administrateur de la COP31      role=admin        scope=event:<uuid>
 *   Révisionniste de la COP31       role=reviewer     scope=event:<uuid>
 *   Référent de l'organisation X    role=org_manager  scope=organization:<uuid>
 *
 * Les quatre lignes ci-dessus portent DEUX rôles seulement. La différence entre
 * les deux premières ne se lit nulle part dans le nom du rôle — elle est dans la
 * portée. C'est pourquoi rien, dans ces contrats, ne transporte un `role_code`
 * sans le `ScopeRef` qui l'accompagne : un composant qui recevrait l'un sans
 * l'autre finirait par afficher « Administrateur » à quelqu'un qui n'administre
 * qu'une édition, et l'écran mentirait sur le seul point qui compte.
 *
 * ── DEUX COLONNES AJOUTÉES AU MODÈLE POUR CET ÉCRAN ─────────────────────────
 *
 * `role_assignments.revoked_by` et `revoked_reason`. Le prompt demande « motif,
 * historique des attributions ET DES RÉVOCATIONS » : la table portait `note` —
 * le motif de l'octroi — et `revoked_at` — une date nue. Qui avait retiré le
 * rôle ne se retrouvait que dans `platform.audit_log`, et pourquoi, nulle part.
 * Or c'est la question qu'on pose six mois plus tard. Une contrainte
 * (`ck_role_assignment_revocation`) interdit de renseigner l'un ou l'autre sur
 * une attribution vivante, sans quoi une attribution en cours portant « fin de
 * mission » se lirait comme révoquée.
 *
 * ── ET UNE PERMISSION QUI N'ÉTAIT DÉTENUE PAR PERSONNE ──────────────────────
 *
 * `identity.role.assign` existait dans le catalogue depuis le premier jour et
 * aucune ligne de `role_permissions` ne l'accordait : seul `super_admin` la
 * détenait, par l'effet du trigger qui lui donne tout. Le back-office des rôles
 * n'aurait donc été utilisable que par le compte technique pivot. Elle est
 * désormais accordée à `admin` — sans élévation possible, la portée de
 * l'attribution suivant celle de l'administrateur, et `super_admin` n'admettant
 * que la portée globale.
 *
 * ── CE QUE LE PÉRIMÈTRE D'ADMINISTRATION VEUT DIRE ICI ──────────────────────
 *
 * Une personne n'appartient à aucune édition — pas plus qu'une organisation
 * (A11). La règle métier n° 8 se lit donc, là encore, par l'autre bout :
 *   · `identity.person.read`, quelle que soit sa portée, ouvre la liste ; celle-ci
 *     est FILTRÉE sur les personnes qui interviennent dans les éditions
 *     administrées, et l'écran le dit quand il a restreint ;
 *   · `identity.role.assign` s'exige SUR LA PORTÉE VISÉE : attribuer un rôle
 *     global demande la permission globale. Un compte détaché sur la COP31 ne
 *     peut attribuer que sur la COP31 — c'est exactement ce que refuse
 *     `has_permission` en base, et ce que `grantableScopes()` calcule à l'écran
 *     pour ne pas offrir un bouton qui sera refusé.
 */

import type {
  AccountId,
  Email,
  I18nText,
  IsoDateTime,
  PermissionCode,
  PersonId,
  RoleAssignmentId,
  RoleCode,
  Uuid,
} from './shared'
import type {
  AdministeredEvents,
  Civility,
  PersonStatus,
  PrivacyRequestStatus,
  PrivacyRequestType,
  RoleAssignment,
  ScopeType,
} from './identity'

// ===========================================================================
// 1. LA PORTÉE, RÉSOLUE
// ===========================================================================

/**
 * Une portée, avec le NOM de ce qu'elle désigne.
 *
 * `role_assignments.scope_id` n'a AUCUNE clé étrangère, et c'est délibéré : la
 * cible vit dans un autre module, qui peut devenir un service distant. Le nom se
 * résout donc par une jointure applicative — titre de l'édition, dénomination de
 * l'organisation — et il peut manquer : une édition supprimée laisse une
 * attribution orpheline, que l'écran doit montrer comme telle plutôt que de
 * taire.
 */
export interface ScopeRef {
  scope_type: ScopeType
  /** `null` si et seulement si `scope_type` vaut `global`. */
  scope_id: Uuid | null
  /**
   * Nom de la cible, résolu hors du module identité. `null` pour la portée
   * globale — qui n'en a pas besoin — comme pour une cible introuvable ; les
   * deux se distinguent par `scope_type`, jamais par ce champ.
   */
  scope_label: I18nText | null
  /** Précision de second rang : les dates de l'édition, le pays de l'organisation. */
  scope_hint: string | null
  /** La cible a disparu : attribution orpheline, à retirer. */
  is_dangling: boolean
}

/**
 * État calculé d'une attribution. Il n'existe pas en base : il se déduit de
 * `revoked_at`, `valid_from` et `valid_until` — et les quatre cas se traitent
 * différemment à l'écran.
 *
 * Les couleurs suivent la charte, et pas l'intuition : `active` est CONFIRMÉ
 * (vert), `scheduled` est une INFORMATION à venir (cyan), `expired` est CLOS
 * (gris), `revoked` est un RETRAIT (rouge). Le jaune, qui dit « demande
 * attention », n'a rien à faire ici.
 */
export type AssignmentState = 'active' | 'scheduled' | 'expired' | 'revoked'

/**
 * Une attribution telle que l'écran la lit — la ligne de la table, sa portée
 * résolue, son rôle nommé, et les personnes qui l'ont accordée puis retirée.
 */
export interface RoleAssignmentView extends RoleAssignment, ScopeRef {
  role_label: I18nText
  role_description: I18nText | null
  /** Le rôle est-il un rôle système (`roles.is_system`) ? Non supprimable. */
  role_is_system: boolean
  /** Ce que ce rôle apporte, pour l'expliquer sans quitter le panneau. */
  role_permissions: PermissionCode[]
  granted_by_name: string | null
  revoked_by_name: string | null
  state: AssignmentState
}

// ===========================================================================
// 2. LA LISTE
// ===========================================================================

/**
 * Une ligne de la liste des utilisateurs.
 *
 * La PERSONNE et le COMPTE restent deux choses distinctes, jusque dans les noms
 * de champ : `has_account` vaut faux pour une personne créée par une invitation
 * ou saisie comme intervenante, qui existe sans jamais s'être connectée. Une
 * liste qui les confondrait afficherait « jamais connecté » là où il faut lire
 * « aucun compte » — et l'administrateur relancerait quelqu'un qui n'a jamais
 * reçu de quoi se connecter.
 */
export interface UserListRow {
  person_id: PersonId
  display_name: string
  primary_email: Email
  email_verified_at: IsoDateTime | null
  job_title: string | null
  /** Nom du pays, résolu depuis `reference.countries`. */
  country_name: I18nText | null
  country_id: Uuid | null
  organization_id: Uuid | null
  /** Dénomination du rattachement principal — `org.organizations.legal_name`. */
  organization_name: string | null
  organization_acronym: string | null
  status: PersonStatus
  status_reason: string | null
  suspended_until: IsoDateTime | null
  /** Attributions EN COURS seulement. L'historique se lit sur la fiche. */
  roles: RoleAssignmentView[]
  /** `max(accounts.last_login_at)` : une personne peut cumuler des fournisseurs. */
  last_login_at: IsoDateTime | null
  has_account: boolean
  /** Un second facteur est actif sur au moins un compte. */
  mfa_enabled: boolean
  /** Verrou après échecs répétés — porte sur le COMPTE, pas sur la personne. */
  locked_until: IsoDateTime | null
  /** Une demande RGPD est ouverte pour cette personne. */
  open_privacy_request: PrivacyRequestType | null
  created_at: IsoDateTime
}

/** Facette de filtrage, avec son décompte — même motif que la liste A11. */
export interface UserFacet {
  value: string
  label: I18nText | string
  count: number
}

/** L'écran de la liste, en une réponse. */
export interface UserListScreen {
  rows: UserListRow[]
  /** Rôles du catalogue, pour la facette et le panneau d'attribution. */
  roles: AssignableRole[]
  countries: UserFacet[]
  organizations: UserFacet[]
  /** La liste a-t-elle été restreinte aux éditions administrées ? */
  scoped_to_events: boolean
  /** Demandes RGPD ouvertes, tous comptes confondus — le pont vers l'écran annexe. */
  open_privacy_requests: number
  /** Comptes suspendus ou bloqués, pour ouvrir l'écran sur ce qui demande un regard. */
  restricted_accounts: number
}

/** Filtres de la liste. Tous portés par l'URL, en français. */
export interface UserListFilters {
  search: string
  /** Codes de rôle, sans distinction de portée : « qui est révisionniste ? ». */
  roles: RoleCode[]
  /** Portées, pour la question inverse : « qui a un rôle SUR CETTE ÉDITION ? ». */
  scope_type: ScopeType | null
  scope_id: Uuid | null
  statuses: PersonStatus[]
  countries: Uuid[]
  organizations: Uuid[]
  /** Personnes sans aucune attribution en cours. */
  without_role: boolean
  /** Personnes sans compte : elles ne peuvent pas se connecter. */
  without_account: boolean
}

export type UserSortKey =
  | 'display_name'
  | 'primary_email'
  | 'organization'
  | 'country'
  | 'roles'
  | 'last_login_at'
  | 'status'

// ===========================================================================
// 3. LE PANNEAU D'ATTRIBUTION — le point central de l'écran
// ===========================================================================

/**
 * Un rôle du catalogue, vu par celui qui attribue.
 *
 * `allowed_scopes` vient de la BASE (`roles.allowed_scopes`) et dit ce que le
 * rôle admet ; `grantable_scopes` est le croisement avec ce que l'ACTEUR détient
 * — deux notions qu'il serait facile de confondre, et dont la confusion produit
 * soit un formulaire qui propose l'impossible, soit un formulaire qui cache le
 * permis.
 */
export interface AssignableRole {
  code: RoleCode
  label: I18nText
  description: I18nText | null
  allowed_scopes: ScopeType[]
  is_system: boolean
  /** Ce que le rôle apporte, résolu : c'est la réponse à « ça donne quoi ? ». */
  permissions: { code: PermissionCode; label: I18nText; module_code: string }[]
  /** Nombre d'attributions en cours de ce rôle, toutes portées confondues. */
  active_count: number
}

/** Une cible de portée offerte au choix — une édition, une organisation. */
export interface ScopeChoice {
  scope_type: Exclude<ScopeType, 'global'>
  scope_id: Uuid
  label: string
  hint: string | null
  /**
   * Hors du périmètre d'administration de l'acteur : offerte en lecture pour
   * qu'il comprenne qu'elle existe, jamais sélectionnable. La taire ferait
   * croire à un bogue à qui cherche une édition qu'il sait présente.
   */
  disabled: boolean
}

/**
 * Ce que le panneau d'attribution a besoin de savoir, en une réponse.
 *
 * `negotiation_spaces` est VIDE et le restera tant que le module Négociations
 * n'a pas d'écran : le rôle `negotiator` admet cette portée en base, le panneau
 * l'affiche donc, désactivée et expliquée. Offrir un choix sans cible, ou
 * masquer une portée que le modèle autorise, seraient deux façons différentes de
 * mentir.
 */
export interface RoleAssignmentOptions {
  roles: AssignableRole[]
  events: ScopeChoice[]
  organizations: ScopeChoice[]
  negotiation_spaces: ScopeChoice[]
  /** L'acteur peut-il attribuer sur la portée GLOBALE ? */
  can_assign_global: boolean
  /** Éditions sur lesquelles il peut attribuer — vide si `can_assign_global`. */
  grantable_event_ids: Uuid[]
}

/**
 * Attribution d'un rôle. `valid_until` facultative, motif conseillé.
 *
 * LA PERSONNE VISÉE VIENT DU CHEMIN, JAMAIS DU CORPS. L'API ne lit pas de
 * `person_id` ici : le porter attribuerait un rôle à quelqu'un d'autre que la
 * fiche ouverte le jour où les deux divergent.
 */
export interface GrantRolePayload {
  role_code: RoleCode
  scope_type: ScopeType
  scope_id: Uuid | null
  /** Prise d'effet ; l'API pose `now()` si elle est absente. */
  valid_from: IsoDateTime | null
  /** Date de fin FACULTATIVE — `ck_role_assignment_window` exige qu'elle suive. */
  valid_until: IsoDateTime | null
  /** Motif de l'octroi : `role_assignments.note`. */
  note: string | null
}

/**
 * Retrait d'un rôle. Le motif alimente `revoked_reason`, ajoutée pour A12.
 * L'attribution visée vient du chemin.
 */
export interface RevokeRolePayload {
  reason: string
}

/**
 * Ce que rend une écriture de rôle.
 *
 * `duplicate` traduit `ux_role_assignments_active` — la même personne, le même
 * rôle, la même portée, deux fois. `scope_not_allowed` traduit
 * `tg_role_assignments_check_scope`, et `forbidden_scope` le refus
 * d'autorisation : l'acteur n'a pas `identity.role.assign` sur la portée visée.
 * Trois refus distincts, trois messages distincts — « impossible » ne dit pas
 * quoi corriger.
 */
export interface RoleWriteResult {
  status: 'granted' | 'revoked' | 'duplicate' | 'scope_not_allowed' | 'forbidden_scope' | 'not_found'
  assignment: RoleAssignmentView | null
  /** Attributions en cours après l'écriture — l'écran s'y recale sans recharger. */
  assignments: RoleAssignmentView[]
  /** L'attribution déjà en place, quand `status` vaut `duplicate`. */
  conflict_with: RoleAssignmentView | null
  /**
   * Le message du trigger, MOT POUR MOT, quand `status` vaut `scope_not_allowed`
   * — « Le rôle « admin » ne peut pas être attribué sur la portée
   * « organization » (portées autorisées : global, event). ». Nul partout
   * ailleurs.
   *
   * La base sait déjà dire quelles portées un rôle admet ; le reformuler dans
   * l'écran produirait un second libellé qui se périmerait à la première
   * évolution du modèle.
   */
  message: string | null
}

// ===========================================================================
// 4. LES PERMISSIONS EFFECTIVES — « voici ce que cette personne peut faire »
// ===========================================================================

/** D'où vient une permission : quel rôle l'apporte, et sur quelle portée. */
export interface PermissionGrant extends ScopeRef {
  role_code: RoleCode
  role_label: I18nText
  assignment_id: RoleAssignmentId
  /** Fin de validité, quand l'attribution en porte une. */
  valid_until: IsoDateTime | null
}

/**
 * Une permission effective, et TOUT ce qui l'accorde.
 *
 * `effective_permissions()` rend des lignes (permission, portée) sans dire d'où
 * elles viennent. C'est suffisant pour autoriser, insuffisant pour expliquer —
 * or l'écran demandé est un écran d'EXPLICATION : « pourquoi cette personne
 * peut-elle décider d'un dossier ? ». La réponse est un rôle et une portée, et
 * il peut y en avoir plusieurs pour la même permission.
 */
export interface EffectivePermissionRow {
  permission_code: PermissionCode
  label: I18nText
  module_code: string
  /** Vraie si au moins un octroi est global : la permission vaut alors partout. */
  is_global: boolean
  grants: PermissionGrant[]
}

/** Les permissions d'une personne, groupées par module. */
export interface PermissionModuleGroup {
  module_code: string
  module_label: I18nText
  rows: EffectivePermissionRow[]
}

/**
 * L'écran « ce que cette personne peut faire, et où ».
 *
 * `administered` reprend `identity.administered_events()` telle quelle : c'est
 * la phrase que tout le back-office lit avant d'afficher quoi que ce soit, et la
 * montrer ici évite d'avoir à la déduire des vingt-quatre lignes du dessous.
 */
export interface EffectivePermissionsView {
  person_id: PersonId
  groups: PermissionModuleGroup[]
  administered: AdministeredEvents
  /** Nombre de permissions distinctes, tous modules et portées confondus. */
  total: number
  /** Permissions du catalogue que cette personne n'a PAS — l'autre moitié de la réponse. */
  missing: { permission_code: PermissionCode; label: I18nText; module_code: string }[]
}

// ===========================================================================
// 5. LA FICHE D'UNE PERSONNE
// ===========================================================================

/** Un compte de connexion, secrets exclus — `identity.accounts`. */
export interface AccountView {
  id: AccountId
  provider: string
  last_login_at: IsoDateTime | null
  password_changed_at: IsoDateTime | null
  mfa_enabled_at: IsoDateTime | null
  failed_attempts: number
  locked_until: IsoDateTime | null
  created_at: IsoDateTime
}

/** Une entrée de l'historique des attributions, prête à afficher. */
export interface AssignmentHistoryEntry {
  assignment_id: RoleAssignmentId
  /** `granted` ou `revoked` : deux événements pour une même ligne de table. */
  kind: 'granted' | 'revoked'
  occurred_at: IsoDateTime
  role_code: RoleCode
  role_label: I18nText
  scope: ScopeRef
  /** Auteur de l'octroi ou du retrait. `null` : action du système ou compte effacé. */
  actor_name: string | null
  /** `note` pour un octroi, `revoked_reason` pour un retrait. */
  reason: string | null
  /** Terme prévu, pour un octroi à durée déterminée. */
  valid_until: IsoDateTime | null
}

/** L'état courant d'un consentement — vue `identity.current_consents`. */
export interface ConsentView {
  purpose: string
  is_granted: boolean
  policy_version: string
  recorded_at: IsoDateTime
}

/** La fiche complète, en une réponse. */
export interface UserDetail {
  person_id: PersonId
  display_name: string
  first_name: string
  last_name: string
  civility: Civility | null
  primary_email: Email
  email_verified_at: IsoDateTime | null
  /** Adresses secondaires — `identity.person_emails`. */
  other_emails: { email: Email; label: string; verified_at: IsoDateTime | null }[]
  phone: string | null
  job_title: string | null
  biography: I18nText | null
  country_id: Uuid | null
  country_name: I18nText | null
  city: string | null
  preferred_locale: string
  timezone: string
  organization_id: Uuid | null
  organization_name: string | null
  is_directory_visible: boolean
  status: PersonStatus
  status_reason: string | null
  status_changed_at: IsoDateTime | null
  status_changed_by_name: string | null
  suspended_until: IsoDateTime | null
  created_at: IsoDateTime
  accounts: AccountView[]
  /** Attributions EN COURS, portées résolues. */
  assignments: RoleAssignmentView[]
  /** Tout ce qui a été accordé puis retiré, du plus récent au plus ancien. */
  history: AssignmentHistoryEntry[]
  permissions: EffectivePermissionsView
  consents: ConsentView[]
  privacy_requests: PrivacyRequestView[]
  /** La personne relève-t-elle du périmètre de l'acteur ? Faux : lecture seule. */
  in_scope: boolean
}

// ===========================================================================
// 6. SUSPENSION, BLOCAGE
// ===========================================================================

/**
 * Changement de statut d'une personne.
 *
 * Les trois statuts accessibles ici sont `active`, `suspended` et `blocked`.
 * `anonymized` n'en est PAS un quatrième : il ne se pose que par
 * `identity.anonymize_person()`, depuis une demande d'effacement, et jamais
 * depuis un panneau de modération — l'offrir ici reviendrait à proposer la
 * destruction irréversible d'une identité à côté d'une suspension de quinze
 * jours.
 *
 * La personne visée vient du chemin.
 */
export interface SetPersonStatusPayload {
  status: Exclude<PersonStatus, 'anonymized'>
  /** Motif OBLIGATOIRE dès qu'on restreint : c'est ce que la personne lira. */
  reason: string
  /** Terme de la suspension — exigé par `ck_people_suspension_window`. */
  suspended_until: IsoDateTime | null
  /** Révoquer les sessions ouvertes dans la foulée. */
  revoke_sessions: boolean
}

/**
 * Aucun refus de portée ici : une personne n'appartient à aucune édition. Le
 * droit se vérifie GLOBALEMENT, avant l'appel, et l'API répond 403 — jamais un
 * statut de plus dans cette union.
 */
export interface PersonWriteResult {
  status: 'saved' | 'missing_deadline' | 'not_found'
  detail: UserDetail | null
}

// ===========================================================================
// 7. DEMANDES RGPD
// ===========================================================================

/** Une demande, avec le nom de la personne et l'état de son échéance. */
export interface PrivacyRequestView {
  id: Uuid
  person_id: PersonId
  person_name: string
  person_email: Email
  request_type: PrivacyRequestType
  status: PrivacyRequestStatus
  due_at: IsoDateTime
  /** Jours restants avant l'échéance réglementaire ; négatif si dépassée. */
  days_left: number
  is_overdue: boolean
  handled_by_name: string | null
  resolution: string | null
  result_asset_id: Uuid | null
  created_at: IsoDateTime
  completed_at: IsoDateTime | null
}

export interface PrivacyQueueScreen {
  requests: PrivacyRequestView[]
  open_count: number
  overdue_count: number
  /** Échéance réglementaire, en jours — 30, portée par le `DEFAULT` de la table. */
  deadline_days: number
}

/**
 * Traitement d'une demande.
 *
 * `anonymize` n'est pas un statut : c'est l'ACTE que réclame une demande
 * d'effacement, et il est irréversible — `anonymize_person()` purge l'identité,
 * supprime les comptes et révoque les sessions, en conservant les agrégats de
 * participation. Le distinguer de la clôture administrative évite qu'on
 * l'exécute en croyant seulement classer un dossier.
 *
 * La demande visée vient du chemin.
 */
export interface HandlePrivacyRequestPayload {
  action: 'start' | 'complete' | 'reject' | 'anonymize'
  resolution: string
}

export interface PrivacyWriteResult {
  status: 'saved' | 'anonymized' | 'wrong_type' | 'not_found'
  request: PrivacyRequestView | null
  requests: PrivacyRequestView[]
}
