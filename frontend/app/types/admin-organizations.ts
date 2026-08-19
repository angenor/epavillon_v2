/**
 * ORGANISATIONS ET FUSION DES DOUBLONS, BACK-OFFICE (A11) — contrats des écrans.
 *
 * Dérivé de `docs/database/040_organizations.sql` et de
 * `analytics.mv_organization_scorecard` (`130_analytics.sql` § 5). Aucun champ
 * inventé : chaque valeur affichée est une colonne d'`org.organizations`,
 * `organization_names`, `organization_domains`, `memberships`,
 * `duplicate_candidates`, `merge_log`, `organization_references` — ou un
 * décompte porté par un autre module et joint ici.
 *
 * ── LE DÉFAUT N° 1 DE LA V1, PRIS PAR LES QUATRE BOUTS ──────────────────────
 *
 * « 2 personnes ont créé deux fois la même organisation et on voit les 2 sur la
 * plateforme sans possibilité de fusionner. » Le SQL pose quatre verrous : les
 * dénominations multiples, la recherche multi-signaux, l'unicité structurelle,
 * la fusion réversible. Ces écrans sont la surface des deux derniers — la file
 * des doublons présumés et l'outil de fusion —, plus la fiche qui rend enfin
 * visible ce qu'une organisation EST : ses noms, ses domaines, ses membres, ses
 * activités.
 *
 * ── CE QUI SERA TRANSFÉRÉ NE SE DEVINE PAS : IL SE LIT ──────────────────────
 *
 * `org.organization_references` est un REGISTRE — chaque module y déclare les
 * colonnes qui pointent vers une organisation, et `merge_organizations()` le
 * parcourt dynamiquement. Le décompte de l'écran de fusion se construit donc à
 * partir de ce registre, table par table, et jamais d'une liste écrite à la
 * main dans un composant Vue : le jour où le module Formations déclare ses deux
 * colonnes, l'écran les compte sans qu'on y touche. C'est la raison d'être de
 * `MergeTransferLine`.
 *
 * TROIS SORTS, PAS UN. Le registre porte une `strategy` et un `dedupe_on`, et
 * les confondre à l'écran ferait mentir le décompte :
 *   · `reassign` — la ligne bascule vers la fiche absorbante ;
 *   · `delete`   — la ligne disparaît (quotas de stockage, politiques de
 *                  publication : ce sont des réglages, pas un patrimoine) ;
 *   · dédoublonnage — une ligne `reassign` dont la valeur existe DÉJÀ côté cible
 *                  (la même personne adhérente des deux fiches, la même
 *                  proposition co-organisée par les deux) est SUPPRIMÉE avant la
 *                  bascule, sans quoi l'unicité ferait échouer la fusion.
 * L'écran annonce donc « transférées » et « supprimées, déjà présentes », deux
 * chiffres distincts.
 *
 * ── CE QUE LE MODÈLE NE PORTE PAS, ET QUI EST DEMANDÉ ICI ───────────────────
 *
 * `org.merge_organizations(source, target, reason)` ne prend AUCUN choix de
 * champ : elle réaffecte les rattachements et laisse la fiche cible telle
 * quelle. Or le prompt demande de choisir, champ par champ, la valeur à
 * conserver — le site web renseigné d'un côté, le téléphone de l'autre. Ces
 * choix sont donc un UPDATE de la fiche cible, à faire AVANT l'appel et DANS LA
 * MÊME TRANSACTION. Obligation d'API inscrite dans `docs/progression/ecrans/a11-organisations-fusion.md` : deux
 * transactions laisseraient une fiche absorbante à moitié complétée si la fusion
 * échouait ensuite.
 *
 * ── LE PÉRIMÈTRE D'ADMINISTRATION, ICI, N'EST PAS UNE ÉDITION ───────────────
 *
 * Une organisation n'appartient à aucune édition : la règle métier n° 8 ne peut
 * pas se lire comme ailleurs. Elle se lit par l'autre bout, celui des
 * permissions et de leur PORTÉE (`identity.effective_permissions`) :
 *   · `org.organization.read`  — ouvre la liste. Accordée sur une seule édition,
 *     elle ne montre que les organisations qui ont déposé ou tenu une activité
 *     dans CETTE édition : c'est le filtrage par périmètre, appliqué à une
 *     entité transverse ;
 *   · `org.organization.merge` — ouvre la fusion, et EXIGE la portée globale :
 *     fusionner deux fiches déplace des rattachements dans toutes les éditions,
 *     y compris celles qu'on n'administre pas.
 */

import type {
  CountryId,
  Email,
  I18nText,
  IsoDateTime,
  Numeric,
  OrganizationId,
  PersonId,
  Slug,
  TaxonomyTermCode,
  Url,
  Uuid,
} from './shared'
import type {
  DuplicateDecision,
  DomainVerificationMethod,
  MembershipRole,
  MembershipStatus,
  NameKind,
  OrganizationReferenceStrategy,
  OrganizationStatus,
} from './org'
import type { OrganizationScorecard } from './analytics'
import type { OrganizationRole, ProposalStatus } from './programme/proposal'
import type { SessionStatus } from './programme/session'

// ===========================================================================
// 1. LISTE DES ORGANISATIONS
// ===========================================================================

/**
 * Une ligne de la liste — la fiche de performance, plus ce qu'il faut pour la
 * LIRE.
 *
 * La vue rend `organization_type_code` et `pays_nom` ; le code d'un type ne
 * s'affiche pas (`ngo_association` n'est pas un libellé), d'où le terme résolu
 * qui l'accompagne. Il vient de `reference.taxonomy_terms`, jamais d'un fichier
 * i18n : c'est une donnée que l'IFDD modifie depuis le back-office.
 */
export interface OrganizationListRow extends OrganizationScorecard {
  /** Libellé du type, résolu depuis `reference.taxonomy_terms`. */
  organization_type_label: I18nText | null
  /** Couleur du terme, quand elle est posée — repère de teinte, pas de fond de texte. */
  organization_type_color: string | null
  /**
   * Cette fiche figure-t-elle dans une paire de doublons présumés NON ARBITRÉE ?
   * C'est le signal qui relie les deux écrans : une fiche douteuse se repère
   * dans la liste, et la file dit avec qui elle se confond.
   */
  pending_duplicate_count: number
  /** Fiches absorbées par celle-ci. Zéro pour la quasi-totalité. */
  absorbed_count: number
}

/** Colonnes triables de la liste. */
export type OrganizationSortKey =
  | 'legal_name'
  | 'country'
  | 'type'
  | 'membres_actifs'
  | 'propositions_deposees'
  | 'propositions_acceptees'
  | 'ratio_acceptation'
  | 'score_confiance'
  | 'derniere_activite'

/**
 * Filtres de la liste — tous portés par l'URL.
 *
 * `verified` et `has_duplicate` sont à trois états (`null` = sans importance) :
 * une case à cocher ne saurait pas dire « fiches NON vérifiées », qui est
 * précisément ce qu'on vient chercher ici.
 */
export interface OrganizationListFilters {
  search: string
  countries: CountryId[]
  types: TaxonomyTermCode[]
  statuses: OrganizationStatus[]
  verified: boolean | null
  has_duplicate: boolean | null
  /** Score de confiance maximal retenu — « les fiches sous 50 ». */
  max_trust_score: number | null
}

/** Une facette comptée sur le même jeu de lignes que la liste. */
export interface OrganizationFacet {
  value: string
  label: I18nText | string
  count: number
}

/**
 * TOUT L'ÉCRAN DE LA LISTE EN UNE RÉPONSE.
 *
 * Les facettes accompagnent les lignes plutôt que d'être demandées à part : leur
 * décompte porte sur le même jeu, sans quoi « Sénégal (3) » finirait par ne plus
 * correspondre à ce qui s'affiche. Même raison qu'en A7 et A10.
 */
export interface OrganizationListScreen {
  rows: OrganizationListRow[]
  countries: OrganizationFacet[]
  types: OrganizationFacet[]
  /** Paires de doublons non arbitrées, toutes fiches confondues : la pastille de la file. */
  pending_duplicates: number
  /**
   * Le périmètre a-t-il restreint la liste ? Vrai pour une personne dont la
   * permission de lecture ne vaut que sur une ou deux éditions : l'écran le dit,
   * plutôt que de laisser croire que la plateforme ne compte que ces fiches.
   */
  scoped_to_events: boolean
}

// ===========================================================================
// 2. FILE DES DOUBLONS PRÉSUMÉS
// ===========================================================================

/**
 * Motif de la suspicion — les quatre valeurs que `find_similar_organizations()`
 * pose dans `match_reasons` et que le worker recopie dans
 * `duplicate_candidates.reasons`.
 *
 * ILS NE SE VALENT PAS, et l'écran doit le montrer : un domaine de messagerie
 * partagé est une preuve matérielle (« c'est la même maison »), une similarité
 * de nom une simple présomption, un même pays presque rien tout seul.
 */
export type DuplicateReason = 'name_similarity' | 'shared_domain' | 'same_country' | 'acronym_match'

/**
 * Une des deux fiches d'une paire, réduite à ce qui permet de TRANCHER sans
 * ouvrir les deux fiches : qui l'a créée, quand, ce qu'elle porte déjà.
 */
export interface DuplicateSide {
  organization_id: OrganizationId
  legal_name: string
  acronym: string | null
  slug: Slug
  status: OrganizationStatus
  organization_type_code: TaxonomyTermCode
  organization_type_label: I18nText | null
  country_id: CountryId | null
  country_name: I18nText | null
  city: string | null
  website: Url | null
  contact_email: Email | null
  verified_at: IsoDateTime | null
  trust_score: number
  member_count: number
  proposal_count: number
  session_count: number
  /** Domaines déclarés, vérifiés ou non : c'est là que se lit le motif partagé. */
  domains: string[]
  created_at: IsoDateTime
  created_by_name: string | null
}

/**
 * Une paire de `org.duplicate_candidates`.
 *
 * `left_id` est TOUJOURS inférieur à `right_id` (`ck_duplicate_candidates_ordered`) :
 * la contrainte interdit d'enregistrer deux fois la même paire dans les deux
 * sens. Cet ordre est technique et ne dit RIEN de qui doit absorber qui — c'est
 * la désignation de l'écran de fusion, et elle n'a rien à voir avec la place
 * d'une fiche dans la paire.
 */
export interface DuplicatePair {
  id: Uuid
  score: Numeric
  reasons: DuplicateReason[]
  detected_at: IsoDateTime
  reviewed_at: IsoDateTime | null
  reviewed_by: PersonId | null
  reviewed_by_name: string | null
  decision: DuplicateDecision | null
  left: DuplicateSide
  right: DuplicateSide
}

/**
 * LA FILE, ET CE QUI EN EST SORTI.
 *
 * Les paires arbitrées ne disparaissent pas : « ce ne sont pas des doublons » se
 * reprend, et une paire écartée par erreur serait autrement introuvable. Elles
 * sont simplement rangées à part, sous la file active.
 */
export interface DuplicateQueueScreen {
  /** Non arbitrées, triées par similarité DÉCROISSANTE — le prompt le demande. */
  pending: DuplicatePair[]
  /** Déjà tranchées : fusionnées, écartées, reportées. */
  settled: DuplicatePair[]
}

/** Décision portée sur une paire depuis la file. La fusion, elle, a son écran. */
export interface DuplicateDecisionPayload {
  pair_id: Uuid
  /** `distinct` retire la paire de la file ; `deferred` la remet à plus tard. */
  decision: Exclude<DuplicateDecision, 'merged'>
  /** Motif, facultatif ici — la fusion, elle, l'exige. */
  note: string | null
}

export interface DuplicateDecisionResult {
  status: 'recorded' | 'not_found'
  pair: DuplicatePair | null
}

// ===========================================================================
// 3. ÉCRAN DE FUSION
// ===========================================================================

/**
 * Champ comparé dans la vue côte à côte. Ce sont les colonnes d'`org.organizations`
 * qu'un humain a saisies ; les colonnes générées (`*_normalized`), les états de
 * fusion et les compteurs n'y figurent pas — ils ne se choisissent pas.
 */
export type MergeField =
  | 'legal_name'
  | 'acronym'
  | 'slug'
  | 'organization_type_code'
  | 'country_id'
  | 'city'
  | 'description'
  | 'website'
  | 'contact_email'
  | 'contact_phone'

/** Quel côté de la paire fournit la valeur retenue. */
export type MergeSideKey = 'source' | 'target'

/**
 * Un champ, ses deux valeurs et leur écart.
 *
 * `differs` NE SUFFIT PAS À DÉCIDER, et c'est pourquoi `filled` existe. Deux
 * situations que l'écran ne doit pas rendre pareil : les deux fiches portent une
 * valeur DIFFÉRENTE — quelqu'un doit trancher —, ou une seule est renseignée —
 * il n'y a rien à trancher, on prend celle qui existe. Confondre les deux
 * imposerait douze arbitrages là où il n'y en a que trois.
 */
export interface MergeFieldComparison {
  field: MergeField
  source_value: unknown
  target_value: unknown
  /** Les deux valeurs diffèrent-elles, vides comprises ? */
  differs: boolean
  /** Quels côtés portent une valeur : ni l'un ni l'autre, l'un, ou les deux. */
  filled: 'none' | 'source' | 'target' | 'both'
  /**
   * Valeur affichée à la place du code brut — libellé de type d'organisation,
   * nom de pays. Nulle pour les champs qui s'affichent tels quels.
   */
  source_label: I18nText | string | null
  target_label: I18nText | string | null
}

/**
 * Une ligne du décompte de transfert — une entrée du registre
 * `org.organization_references`, chiffrée sur la paire courante.
 *
 * `label_key` nomme la table pour l'interface ; c'est une clé i18n et non un
 * libellé, parce que le registre est une donnée technique — « programme.proposals »
 * ne se montre pas à un chargé de programme.
 */
export interface MergeTransferLine {
  ref_schema: string
  ref_table: string
  ref_column: string
  strategy: OrganizationReferenceStrategy
  /** Colonnes formant une unicité avec la colonne d'organisation. */
  dedupe_on: string[]
  /** Lignes qui basculeront vers la fiche absorbante. */
  reassigned: number
  /**
   * Lignes SUPPRIMÉES avant la bascule parce que la cible porte déjà la même
   * valeur — la même personne adhérente des deux fiches. Ce ne sont pas des
   * pertes de données : le rattachement existe déjà de l'autre côté.
   */
  deduped: number
  /** Lignes supprimées par la stratégie `delete` — réglages, jamais patrimoine. */
  deleted: number
}

/**
 * TOUT L'ÉCRAN DE FUSION, POUR UNE PAIRE DONNÉE.
 *
 * Il est calculé pour un SENS de fusion (source absorbée par cible) et recalculé
 * quand on inverse : le décompte n'est pas symétrique — trois adhésions
 * transférées dans un sens peuvent en faire cinq dans l'autre, selon ce que
 * chaque fiche porte déjà.
 */
export interface MergePreview {
  /** La fiche ABSORBÉE : elle passera en statut `merged`. */
  source: DuplicateSide
  /** La fiche ABSORBANTE : elle survit, et c'est elle qu'on complète. */
  target: DuplicateSide
  /** Paire de la file dont vient cette fusion, ou `null` pour une fusion à la main. */
  pair_id: Uuid | null
  comparisons: MergeFieldComparison[]
  transfers: MergeTransferLine[]
  /**
   * Dénominations que la source apportera à la cible — c'est ce qui fait qu'une
   * recherche sur l'ancien nom continue de trouver la bonne fiche. Elles sont
   * traitées à part du registre par `merge_organizations()` (§ 6, étape 1).
   */
  transferred_names: { name: string; kind: NameKind; is_confirmed: boolean; already_present: boolean }[]
  /** Domaines apportés, avec leur état de vérification. */
  transferred_domains: { domain: string; verified_at: IsoDateTime | null; already_present: boolean }[]
  /**
   * Ce qui doit ARRÊTER la main avant le geste. Non bloquant : la fusion reste
   * possible, l'écran ne décide pas à la place de l'équipe.
   */
  warnings: MergeWarning[]
}

/**
 * Un avertissement de l'écran de fusion.
 *
 * `source_is_verified` est le plus important : absorber une fiche PORTANT LE
 * SCEAU DE L'IFDD dans une fiche qui ne l'a pas fait perdre la vérification, et
 * personne ne s'en aperçoit avant qu'un public la cherche.
 */
export interface MergeWarning {
  code:
    | 'source_is_verified'
    | 'source_has_more_activity'
    | 'source_has_verified_domain'
    | 'target_not_verified'
    | 'different_countries'
    | 'different_types'
  /** Valeurs à interpoler dans le message traduit. */
  values?: Record<string, string | number>
}

/**
 * La demande de fusion.
 *
 * `confirmation_name` est le NOM DE LA FICHE ABSORBÉE, saisi à la main. C'est le
 * dernier verrou avant une opération que rien n'annule d'un clic — la fiche
 * survit, mais les rattachements sont déplacés et il faudrait les reprendre un à
 * un. L'API le revérifie : masquer un bouton n'a jamais empêché une requête.
 */
export interface MergePayload {
  source_id: OrganizationId
  target_id: OrganizationId
  pair_id: Uuid | null
  /** Obligatoire — `org.merge_log.reason`, et c'est ce qu'on relira dans six mois. */
  reason: string
  /**
   * Pour chaque champ divergent, le côté dont la valeur est conservée. Absent du
   * dictionnaire, le champ garde la valeur de la CIBLE : c'est elle qui survit,
   * et l'absence de choix ne doit rien écraser.
   */
  field_choices: Partial<Record<MergeField, MergeSideKey>>
  confirmation_name: string
}

/**
 * Ce que rend la fusion.
 *
 * Les deux refus sont ceux que la base ou l'écran opposent réellement :
 * `confirmation_mismatch` — le nom saisi ne correspond pas ;
 * `already_merged` — `tg_forbid_merge_chains` interdit de cibler une fiche
 * elle-même fusionnée (« Cibler la fiche finale »), et une fusion concurrente a
 * pu passer entre l'ouverture de l'écran et la validation.
 */
export interface MergeResult {
  status: 'merged' | 'confirmation_mismatch' | 'already_merged' | 'not_found'
  /** Fiche absorbante après fusion — celle vers laquelle l'écran renvoie. */
  target: OrganizationId | null
  /** Décompte réel des lignes déplacées, par `schéma.table.colonne` — `merge_log.rows_reassigned`. */
  rows_reassigned: Record<string, number>
  /** Champs de la cible modifiés par les choix de l'opérateur. */
  fields_applied: MergeField[]
}

// ===========================================================================
// 4. FICHE D'UNE ORGANISATION
// ===========================================================================

/** Une dénomination, avec l'auteur qui l'a ajoutée. */
export interface OrganizationNameRow {
  id: Uuid
  name: string
  kind: NameKind
  locale: string | null
  is_confirmed: boolean
  created_by_name: string | null
  created_at: IsoDateTime
  /**
   * Dénomination POSÉE PAR LA BASE — le nom légal et le sigle, recopiés par
   * `tg_sync_organization_names`. Elles ne se retirent pas à la main : elles
   * suivent la fiche.
   */
  is_derived: boolean
}

/** Un domaine de messagerie et son état de vérification. */
export interface OrganizationDomainRow {
  id: Uuid
  domain: string
  verified_at: IsoDateTime | null
  verification_method: DomainVerificationMethod | null
  auto_join: boolean
  created_at: IsoDateTime
  /**
   * Ce domaine est-il porté par une AUTRE fiche ? C'est le signal de doublon le
   * plus fiable du modèle, et il se voit d'abord ici.
   */
  shared_with: { organization_id: OrganizationId; legal_name: string }[]
}

/** Un membre, avec la direction de son attente quand il y en a une. */
export interface OrganizationMemberRow {
  id: Uuid
  person_id: PersonId
  display_name: string
  primary_email: Email
  role: MembershipRole
  status: MembershipStatus
  is_primary: boolean
  job_title: string | null
  /**
   * Renseigné : l'organisation a INVITÉ et attend la personne. Nul sur une
   * adhésion `pending` : la personne a DEMANDÉ et attend un référent. Le même
   * statut recouvre deux attentes inverses — les confondre, c'est approuver une
   * adhésion que l'intéressé n'a jamais acceptée.
   */
  invited_at: IsoDateTime | null
  approved_at: IsoDateTime | null
  revoked_at: IsoDateTime | null
  created_at: IsoDateTime
}

/** Une activité portée par l'organisation, dossier ou séance. */
export interface OrganizationActivityRow {
  kind: 'proposal' | 'session'
  id: Uuid
  reference_code: string | null
  title: I18nText
  event_id: Uuid
  event_name: I18nText
  edition_year: number
  /** Rôle tenu — `programme.organization_role` : porteur, co-organisateur, partenaire, soutien. */
  role: OrganizationRole
  status: ProposalStatus | SessionStatus
  /** Date qui situe l'activité : dépôt pour un dossier, créneau pour une séance. */
  occurred_at: IsoDateTime | null
}

/**
 * Une ligne d'historique — `platform.entity_history('org', 'organizations', id)`.
 *
 * Ce n'est PAS une table : l'historique est un sous-produit du journal d'audit.
 * `actor_label` y est dénormalisé volontairement — il reste lisible après
 * anonymisation RGPD, quand `actor_id` ne pointe plus vers personne.
 */
export interface OrganizationHistoryEntry {
  occurred_at: IsoDateTime
  actor_id: PersonId | null
  actor_label: string | null
  action: 'insert' | 'update' | 'delete'
  field: string | null
  old_value: unknown
  new_value: unknown
}

/** Une fusion inscrite au journal — `org.merge_log`. */
export interface OrganizationMergeEntry {
  id: Uuid
  source_id: OrganizationId
  source_name: string
  target_id: OrganizationId
  target_name: string
  performed_by_name: string | null
  performed_at: IsoDateTime
  rows_reassigned: Record<string, number>
  reason: string | null
}

/**
 * TOUTE LA FICHE EN UNE RÉPONSE.
 *
 * `merged_into` n'est pas un détail d'affichage : une fiche absorbée reste
 * consultable — c'est la promesse de `org.resolve_organization()`, et les URL
 * déjà diffusées continuent de résoudre. L'écran d'une fiche `merged` s'ouvre
 * donc normalement, coiffé du renvoi vers la fiche vivante.
 */
export interface OrganizationDetail {
  organization_id: OrganizationId
  legal_name: string
  acronym: string | null
  slug: Slug
  status: OrganizationStatus
  organization_type_code: TaxonomyTermCode
  organization_type_label: I18nText | null
  country_id: CountryId | null
  country_name: I18nText | null
  city: string | null
  description: I18nText | null
  website: Url | null
  contact_email: Email | null
  contact_phone: string | null
  verified_at: IsoDateTime | null
  verified_by_name: string | null
  trust_score: number
  created_at: IsoDateTime
  created_by_name: string | null

  /** Fiche absorbante, quand celle-ci a été fusionnée. */
  merged_into: { organization_id: OrganizationId; legal_name: string; merged_at: IsoDateTime } | null
  /** Fiches que celle-ci a absorbées. */
  absorbed: { organization_id: OrganizationId; legal_name: string; merged_at: IsoDateTime }[]

  scorecard: OrganizationScorecard
  names: OrganizationNameRow[]
  domains: OrganizationDomainRow[]
  members: OrganizationMemberRow[]
  activities: OrganizationActivityRow[]
  history: OrganizationHistoryEntry[]
  merges: OrganizationMergeEntry[]
  /** Paires non arbitrées où cette fiche apparaît : le lien vers la fusion. */
  duplicates: DuplicatePair[]
}

/**
 * Les écritures de la fiche.
 *
 * LE SCEAU N'EST PAS LE STATUT. `verified_at` dit que l'IFDD a reconnu
 * l'organisation ; `status` dit où elle en est de son cycle de vie. Une fiche
 * peut être active sans sceau — elle soumet, mais rien ne s'affiche à côté de
 * son nom. Les mélanger ferait disparaître d'un écran une organisation qu'on
 * voulait seulement ne pas mettre en avant.
 */
export interface OrganizationVerificationPayload {
  organization_id: OrganizationId
  /** Vrai pose le sceau, faux le retire. */
  verified: boolean
}

/** Vérification manuelle d'un domaine, ou bascule de son rattachement automatique. */
export interface DomainVerificationPayload {
  organization_id: OrganizationId
  domain_id: Uuid
  /** `manual` : la vérification par jeton DNS et par courriel appartient au worker. */
  verified: boolean
  auto_join: boolean
}

/** Confirmation d'une dénomination saisie à l'import ou par un utilisateur. */
export interface NameConfirmationPayload {
  organization_id: OrganizationId
  name_id: Uuid
  is_confirmed: boolean
}

/**
 * Ce que rend une écriture de la fiche : la fiche entière, recomposée.
 *
 * Même parti qu'en A10 : vérifier un domaine change le score de confiance, qui
 * change le rang de la fiche dans la liste, et poser le sceau change ce que la
 * file des doublons affiche. Rendre le seul objet modifié laisserait trois
 * panneaux afficher des valeurs fausses jusqu'au prochain rechargement.
 */
export interface OrganizationWriteResult {
  status: 'saved' | 'not_found' | 'domain_taken'
  detail: OrganizationDetail | null
  /** Fiche qui détient déjà ce domaine vérifié — `ux_organization_domains_verified`. */
  conflict_with: { organization_id: OrganizationId; legal_name: string } | null
}
