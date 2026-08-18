/**
 * LA LISTE DES PROPOSITIONS DU BACK-OFFICE (A7) — ce que l'écran reçoit, ce
 * qu'il filtre, ce qu'il renvoie quand l'équipe agit sur plusieurs dossiers.
 *
 * LES LIGNES NE SONT PAS ICI. Elles sont `ProposalDashboardRow`
 * (`types/views.ts`), c'est-à-dire la vue `programme.v_proposal_dashboard`
 * telle quelle : format, pays, thématiques, révisionnistes et retards y ont été
 * ajoutés le 18/08 pour que cet écran tienne en une requête. Redéclarer ici une
 * ligne « d'écran » à côté d'elle serait la garantie d'une divergence au premier
 * champ ajouté.
 *
 * CE QUI EST ICI, C'EST CE QUE LA BASE NE PORTE PAS : le vocabulaire des
 * filtres, les facettes avec leur décompte, la liste des dossiers que la
 * personne connectée n'a pas encore ouverts, et le résultat des actions
 * groupées. Ces compositions appartiendront à l'API (prompt B7), pas à une vue
 * SQL de plus — comme celles de l'espace organisation et du tableau de bord.
 */

import type { ProposalStatus } from './programme/proposal'
import type { ParticipationMode } from './event/edition'
import type { ProposalDashboardRow } from './views'
import type {
  ColorHex,
  EventId,
  I18nText,
  IsoDateTime,
  OrganizationId,
  PersonId,
  ProposalId,
  TaxonomyTermCode,
  TimeZoneName,
} from './shared'

// ---------------------------------------------------------------------------
// Filtres
// ---------------------------------------------------------------------------

/**
 * DEUX SIGNAUX QUI NE SONT PAS DES STATUTS, et qu'on ne peut pas exprimer
 * autrement : un dossier « non évalué » peut être déposé ou en évaluation, un
 * dossier « en retard » peut porter deux revues sur trois. Ils croisent la
 * colonne `review_count` et la colonne `overdue_reviews` de la vue, pas le
 * statut — les confondre ferait manquer exactement les dossiers qu'on cherche.
 *
 * `unread` est le troisième, et il est PERSONNEL : « que je n'ai pas encore
 * ouverts », d'où `unread_ids` plus bas.
 */
export type ProposalFlag = 'unreviewed' | 'late' | 'unread'

/**
 * Les colonnes triables. Le tri par défaut est la NOTE DÉCROISSANTE : cet écran
 * sert à décider, et la première question du comité est « lesquels tiennent le
 * haut du classement ». `title` et `organization` trient sur la forme résolue en
 * français (`title_text`), comme le fait la vue.
 */
export type ProposalSortKey =
  | 'reference_code'
  | 'title'
  | 'organization'
  | 'country'
  | 'format'
  | 'status'
  | 'reviews'
  | 'average_score'
  | 'event_rank'
  | 'submitted_at'

/**
 * TEXTES DÉJÀ RÉSOLUS d'une ligne, fournis par l'écran aux fonctions pures de
 * tri et d'export (`utils/proposal-list.ts`).
 *
 * Trier la colonne « Format » sur `in_person` / `online` / `hybrid` donnerait un
 * ordre qui n'a de sens dans aucune des deux langues, et le nom d'un pays est
 * une donnée multilingue de la base. Les fonctions pures ne connaissent ni i18n
 * ni locale : elles reçoivent le texte que l'utilisateur LIT.
 */
export interface ProposalFilterText {
  status: (row: ProposalDashboardRow) => string
  format: (row: ProposalDashboardRow) => string
  country: (row: ProposalDashboardRow) => string
}

/** État complet des filtres, tel que l'URL le porte et le restitue. */
export interface ProposalListFilters {
  /** Recherche plein texte — numéro de dossier, titre, organisation. */
  search: string
  statuses: ProposalStatus[]
  themes: TaxonomyTermCode[]
  formats: ParticipationMode[]
  /** Codes ISO 3166-1 alpha-2, tels que `organization_country_code` les porte. */
  countries: string[]
  organizations: OrganizationId[]
  /** Révisionniste affecté, déports exclus. Un seul à la fois : croiser deux
   *  membres du comité répond à une question qui ne se pose pas. */
  reviewer: PersonId | null
  flags: ProposalFlag[]
}

/**
 * Une valeur de filtre et son décompte SUR LE PÉRIMÈTRE, filtres non appliqués.
 *
 * `label` vient de la BASE quand la valeur y est nommée — thématique, pays,
 * organisation, personne ; il est alors multilingue ou déjà résolu. Il est nul
 * pour un code d'ENUM (statut, format), que l'écran traduit lui-même : un statut
 * est un libellé d'interface, pas une donnée modifiable au back-office.
 */
export interface ProposalFacet {
  value: string
  label: I18nText | string | null
  count: number
  /** Couleur de `reference.taxonomy_terms` — donnée, jamais jeton de design. */
  color?: ColorHex | null
}

/** Les facettes de chaque filtre, dans l'ordre où l'écran les propose. */
export interface ProposalFacets {
  statuses: ProposalFacet[]
  themes: ProposalFacet[]
  formats: ProposalFacet[]
  countries: ProposalFacet[]
  organizations: ProposalFacet[]
  reviewers: ProposalFacet[]
  /** Décompte des trois signaux transverses, par code de `ProposalFlag`. */
  flags: ProposalFacet[]
}

// ---------------------------------------------------------------------------
// La réponse de l'écran
// ---------------------------------------------------------------------------

/**
 * TOUT L'ÉCRAN EN UNE RÉPONSE. Les lignes, les facettes, ce que la personne n'a
 * pas lu, et le fuseau dans lequel s'affichent les dates — celui de l'édition,
 * jamais celui du navigateur.
 */
export interface ProposalListScreen {
  event_id: EventId
  /** `event.events.timezone` : toute date de cet écran s'y affiche. */
  timezone: TimeZoneName
  /** Nom de ville de l'édition, qui NOMME le fuseau (« heure de Belém »). */
  city: string | null
  /** Échéance effective de l'appel — `event.effective_deadline()`, prolongation
   *  comprise. Nulle si l'édition n'a pas d'appel. */
  deadline: IsoDateTime | null
  /** `calls_for_proposals.required_reviews` : le dénominateur du « 2/3 ». */
  required_reviews: number | null
  rows: ProposalDashboardRow[]
  facets: ProposalFacets
  /**
   * Dossiers que la personne connectée n'a JAMAIS ouverts —
   * `programme.unread_proposals_for()`. Une liste d'identifiants et non un
   * champ de ligne : « non lu » est une relation entre un dossier et un lecteur,
   * la même ligne étant lue par l'un et pas par l'autre.
   */
  unread_ids: ProposalId[]
}

// ---------------------------------------------------------------------------
// Actions groupées
// ---------------------------------------------------------------------------

export interface AssignReviewerPayload {
  proposal_ids: ProposalId[]
  reviewer_id: PersonId
  /** Échéance de revue commune, dans le fuseau de l'édition. */
  due_at?: IsoDateTime | null
}

export interface ChangeStatusPayload {
  proposal_ids: ProposalId[]
  to_status: ProposalStatus
  /** Obligatoire pour les transitions dont `requires_reason` est vrai. */
  reason?: string | null
}

/**
 * POURQUOI UN DOSSIER N'A PAS SUIVI. Une action groupée porte sur une sélection
 * hétérogène : sur douze dossiers retenus, trois sont déjà confiés à cette
 * personne, un a été déporté, deux ne sont pas dans le bon état. Répondre « 6
 * dossiers traités » sans dire ce qu'il est advenu des six autres, c'est laisser
 * croire à un échec silencieux — le défaut classique des actions de masse.
 */
export type BulkSkipReason =
  | 'already_assigned'
  | 'recused'
  | 'transition_not_allowed'
  | 'reason_required'
  | 'not_found'

export interface BulkSkip {
  proposal_id: ProposalId
  reference_code: string
  reason: BulkSkipReason
}

export interface BulkResult {
  /** Dossiers réellement modifiés. */
  applied: ProposalId[]
  skipped: BulkSkip[]
}

/**
 * Transitions proposées par l'action groupée « changer de statut », dérivées de
 * `programme.proposal_transitions_allowed` — la machine à états est une DONNÉE,
 * l'écran ne la réimplémente pas. `requires_reason` commande l'affichage du
 * champ de motif : le trigger refuse la transition sans lui.
 */
export interface BulkStatusOption {
  to_status: ProposalStatus
  requires_reason: boolean
  /** Nombre de dossiers de la sélection à qui cette transition s'applique. */
  eligible: number
}
