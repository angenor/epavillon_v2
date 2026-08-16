/**
 * Schéma `programme`, partie 1 — propositions d'activité, cycle de vie,
 * co-organisation, intervenants, pièces jointes, échanges.
 * Dérivé de `docs/database/070_programme_proposals.sql`.
 *
 * DÉCISION DU MODÈLE : la PROPOSITION est un DOSSIER, la SESSION est une
 * OCCURRENCE PROGRAMMÉE (`programme/session.ts`). Une proposition acceptée
 * engendre une ou plusieurs sessions ; une session peut exister sans proposition
 * quand l'IFDD programme directement une activité. La v1 confondait les deux, et
 * son unique colonne `validation_status` mélangeait états de dossier et états de
 * diffusion.
 */

import type {
  AssetId,
  CallId,
  CountryId,
  EventId,
  I18nText,
  IsoDateTime,
  Numeric,
  OrganizationId,
  PermissionCode,
  PersonId,
  ProposalId,
  Slug,
  TaxonomyTermCode,
  Uuid,
} from '../shared'
import type { ParticipationMode } from '../event/edition'

// ---------------------------------------------------------------------------
// Cycle de vie
// ---------------------------------------------------------------------------

/** ENUM `programme.proposal_status`. */
export type ProposalStatus =
  | 'draft'
  | 'submitted'
  | 'under_review'
  | 'changes_requested'
  | 'accepted'
  | 'rejected'
  | 'withdrawn'
  | 'cancelled'

/**
 * Table `programme.proposal_transitions_allowed` — `070` § 1.
 *
 * La machine à états est une DONNÉE, pas du code : l'interface peut lire cette
 * table pour n'afficher que les actions réellement possibles, au lieu de
 * réimplémenter le graphe et de diverger de la base.
 */
export interface ProposalTransitionRule {
  from_status: ProposalStatus
  to_status: ProposalStatus
  required_permission: PermissionCode | null
  /** Le soumissionnaire lui-même peut-il déclencher cette transition ? */
  allowed_for_owner: boolean
  /** Motif obligatoire : le trigger refuse la transition sans `decision_reason`. */
  requires_reason: boolean
}

/** Table `programme.proposal_transitions` — `070` § 3. Journal des changements d'état. */
export interface ProposalTransition {
  id: Uuid
  proposal_id: ProposalId
  /** Nul pour la ligne d'ouverture du dossier. */
  from_status: ProposalStatus | null
  to_status: ProposalStatus
  actor_id: PersonId | null
  reason: string | null
  occurred_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// Propositions
// ---------------------------------------------------------------------------

/**
 * Table `programme.proposals` — `070` § 2.
 * La colonne `search_vector` (`tsvector`) est omise : sans usage côté client.
 */
export interface Proposal {
  id: ProposalId
  /** Numéro de dossier communiqué à l'organisation (« COP30-00147 »),
   *  attribué à la création, jamais réutilisé. */
  reference_code: string
  /** Nul pour une proposition hors appel, créée par l'IFDD. */
  call_id: CallId | null
  /** Dénormalisation assumée : l'édition reste connue même sans appel. */
  event_id: EventId
  /** PORTEUR PRINCIPAL : celui qui soumet, répond aux demandes de correction et
   *  est notifié de la décision. Tenu en cohérence avec la ligne de rôle `lead`
   *  de `proposal_organizations` par trigger. */
  organization_id: OrganizationId
  submitted_by: PersonId
  contact_person_id: PersonId | null
  title: I18nText
  slug: Slug
  summary: I18nText | null
  objectives: I18nText
  detailed_presentation: I18nText
  expected_outcomes: I18nText | null
  target_audience: I18nText | null
  format: ParticipationMode
  /** Code de la taxonomie `activity_category`. Les thématiques passent par
   *  `reference.entity_terms` (`'programme'`, `'proposals'`, id). */
  activity_type_code: TaxonomyTermCode | null
  /** Codes de `reference.locales`. */
  language_codes: string[]
  country_id: CountryId | null
  /** Créneau SOUHAITÉ par l'organisation ; le créneau retenu vit sur la session. */
  preferred_start_at: IsoDateTime | null
  preferred_end_at: IsoDateTime | null
  duration_minutes: number | null
  /** Occurrences demandées : un cycle de webinaires en annonce plusieurs dès le dépôt. */
  requested_sessions: number
  scheduling_constraints: string | null
  status: ProposalStatus
  submitted_at: IsoDateTime | null
  decided_at: IsoDateTime | null
  decision_reason: string | null
  decided_by: PersonId | null
  /** Agrégats recalculés par `refresh_proposal_score()` : moyenne sur 20 et
   *  moyenne pondérée. Dénormalisés pour trier et paginer sans recalcul. */
  average_score: Numeric | null
  weighted_score: Numeric | null
  review_count: number
  /** Vrai dès qu'un critère éliminatoire a reçu la note zéro. */
  is_knocked_out: boolean
  view_count: number
  deleted_at: IsoDateTime | null
  deleted_by: PersonId | null
  deleted_reason: string | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// Co-organisation
// ---------------------------------------------------------------------------

/**
 * ENUM `programme.organization_role`.
 * Partagé avec `programme.session_organizations` : la liste des co-organisateurs
 * est reprise de la proposition à la programmation, puis modifiable.
 */
export type OrganizationRole = 'lead' | 'co_organizer' | 'partner' | 'sponsor'

/**
 * Table `programme.proposal_organizations` — `070` § 3 bis.
 * Plusieurs organisations peuvent co-organiser une activité. Sans cette table,
 * les co-organisateurs restaient dans le texte de présentation — invisibles des
 * filtres, des statistiques et du décompte par organisation.
 */
export interface ProposalOrganization {
  proposal_id: ProposalId
  organization_id: OrganizationId
  role: OrganizationRole
  /** Une co-organisation annoncée engage un tiers : tant qu'elle n'est pas
   *  confirmée, elle s'affiche « en attente » au back-office. */
  confirmed_at: IsoDateTime | null
  sort_order: number
  added_by: PersonId | null
  added_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// Intervenants et pièces jointes
// ---------------------------------------------------------------------------

/** ENUM `programme.speaker_role`, partagé avec `session_speakers`. */
export type SpeakerRole =
  | 'speaker'
  | 'moderator'
  | 'panelist'
  | 'keynote'
  | 'facilitator'
  | 'interpreter'

/**
 * Table `programme.proposal_speakers` — `070` § 4.
 * L'intervenant EST une personne (`identity.people`), créée à la volée si elle
 * est inconnue : la v1 recopiait nom, prénom, email et photo à chaque activité.
 */
export interface ProposalSpeaker {
  id: Uuid
  proposal_id: ProposalId
  person_id: PersonId
  role: SpeakerRole
  /** Fonction et organisation AU MOMENT de cette activité : une personne change
   *  d'employeur, l'archive d'une COP passée ne doit pas être réécrite. */
  job_title_snapshot: string | null
  organization_snapshot: string | null
  organization_id: OrganizationId | null
  bio: I18nText | null
  /** Confirmation par l'intervenant lui-même, via jeton envoyé par courriel. */
  confirmed_at: IsoDateTime | null
  confirmation_sent_at: IsoDateTime | null
  is_available_for_questions: boolean
  sort_order: number
  created_at: IsoDateTime
}

/** Table `programme.proposal_documents` — `070` § 4. */
export interface ProposalDocument {
  id: Uuid
  proposal_id: ProposalId
  asset_id: AssetId
  title: I18nText
  /** Code de la taxonomie `document_type`. */
  document_type_code: TaxonomyTermCode | null
  /** Visible du public une fois l'activité publiée, ou pièce interne au dossier. */
  is_public: boolean
  uploaded_by: PersonId | null
  uploaded_at: IsoDateTime
  sort_order: number
}

// ---------------------------------------------------------------------------
// Échanges autour d'un dossier
// ---------------------------------------------------------------------------

/** ENUM `programme.comment_visibility`. */
export type CommentVisibility = 'committee' | 'submitter' | 'private'

/**
 * Table `programme.proposal_comments` — `070` § 6.
 * La visibilité est un ÉTAT EXPLICITE : la v1 stockait les destinataires dans un
 * tableau d'identifiants sans contrainte référentielle.
 */
export interface ProposalComment {
  id: Uuid
  proposal_id: ProposalId
  parent_id: Uuid | null
  author_id: PersonId
  visibility: CommentVisibility
  body: string
  /** Demande de correction adressée au soumissionnaire : c'est ce fil qui pilote
   *  l'état `changes_requested`. */
  is_change_request: boolean
  resolved_at: IsoDateTime | null
  resolved_by: PersonId | null
  edited_at: IsoDateTime | null
  deleted_at: IsoDateTime | null
  created_at: IsoDateTime
}

/**
 * Table `programme.proposal_reads` — `070` § 6.
 * Accusés de lecture : « lu par 3 membres du comité sur 5 ».
 */
export interface ProposalRead {
  proposal_id: ProposalId
  person_id: PersonId
  first_read_at: IsoDateTime
  last_read_at: IsoDateTime
  read_count: number
}

// ---------------------------------------------------------------------------
// Historique
// ---------------------------------------------------------------------------

/**
 * Ligne de `programme.proposal_history(proposal_id)` — `070` § 7 bis, dérivée de
 * `platform.entity_history()`. Alimente l'onglet « Historique » du back-office.
 * Les champs recalculés (scores, compteurs) en sont écartés côté base.
 */
export interface ProposalHistoryEntry {
  occurred_at: IsoDateTime
  actor_id: PersonId | null
  actor_label: string | null
  /** `INSERT`, `UPDATE` ou `DELETE`. */
  action: string
  /** Nom de la colonne modifiée. */
  field: string | null
  old_value: unknown
  new_value: unknown
}
