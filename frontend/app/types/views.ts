/**
 * Vues SQL consommées TELLES QUELLES par l'interface.
 *
 * Ces vues répondent à un écran entier en une requête : les utiliser plutôt que
 * de recomposer la jointure côté application, et surtout plutôt que de dériver
 * ces types des tables sous-jacentes — les colonnes ont d'autres noms
 * (`organization_name`, `room_name`) et portent des valeurs déjà calculées.
 *
 * Sources :
 *   - `programme.v_public_schedule`    — `docs/database/075_programme_sessions.sql` § 6
 *   - `programme.v_proposal_dashboard` — `docs/database/070_programme_proposals.sql` § 7
 */

import type {
  BroadcastChannelId,
  CallId,
  ColorHex,
  EventDayId,
  EventId,
  I18nText,
  IsoDateTime,
  Numeric,
  OrganizationId,
  PersonId,
  ProposalId,
  RoomId,
  SessionId,
  Slug,
  TaxonomyTermCode,
  TimeZoneName,
} from './shared'
import type { ParticipationMode, TrackKind } from './event/edition'
import type { AttachedImage } from './media'
import type { SessionStatus } from './programme/session'
import type { ProposalStatus } from './programme/proposal'

// ---------------------------------------------------------------------------
// programme.v_public_schedule — la programmation publique
// ---------------------------------------------------------------------------

/**
 * État temporel calculé EN BASE, à ne pas recalculer dans un composant : deux
 * implémentations divergeraient sur les cas limites (session en cours, annulée,
 * reportée).
 */
export type TemporalState = 'cancelled' | 'postponed' | 'upcoming' | 'ongoing' | 'past'

/**
 * Pastille de journée spéciale agrégée par la vue. Seuls les fils publiés
 * apparaissent.
 */
export interface ScheduleTrackBadge {
  slug: Slug
  title: I18nText
  color: ColorHex | null
  kind: TrackKind
}

/**
 * Pastille thématique agrégée par la vue — `reference.term_badges()`.
 *
 * Libellé et couleur viennent de `reference.taxonomy_terms`, où un
 * administrateur les modifie : ils ne sont NI dans un fichier i18n, NI dans une
 * feuille de style. C'est la règle qui a manqué à la v1.
 */
export interface ScheduleThemeBadge {
  code: TaxonomyTermCode
  label: I18nText
  color: ColorHex | null
  icon: string | null
}

/**
 * Ligne de `programme.v_public_schedule` — une ligne = un bloc du calendrier.
 * La vue ne retient que les sessions dont `published_at` est renseigné.
 */
export interface PublicScheduleRow {
  id: SessionId
  event_id: EventId
  event_day_id: EventDayId | null
  /** Dossier d'origine ; nul quand l'IFDD programme directement une activité. */
  proposal_id: ProposalId | null
  slug: Slug
  title: I18nText
  summary: I18nText | null
  starts_at: IsoDateTime
  ends_at: IsoDateTime
  /** Fuseau de la session : toute heure affichée le mentionne. */
  timezone: TimeZoneName
  format: ParticipationMode
  status: SessionStatus
  room_id: RoomId | null
  /** `event.rooms.name`, déjà joint. */
  room_name: I18nText | null
  organization_id: OrganizationId | null
  /** `org.organizations.legal_name`, déjà joint. */
  organization_name: string | null
  organization_acronym: string | null
  /** Code ISO 3166-1 alpha-2 du pays de l'organisation — stable, filtrable. */
  organization_country_code: string | null
  /** Nom du pays, multilingue : à résoudre comme tout `platform.i18n_text`. */
  organization_country: I18nText | null
  is_streamed: boolean
  broadcast_channel_id: BroadcastChannelId | null
  capacity: number | null
  /** Journées spéciales et fils thématiques, agrégés — pas de requête en plus. */
  tracks: ScheduleTrackBadge[]
  /**
   * Image de couverture, résolue EN BASE par `media.attached_image()` :
   * celle de la séance, à défaut celle de la proposition d'origine.
   *
   * Le repli est la règle, pas une commodité — une organisation joint son image
   * au DÉPÔT, et personne ne revient en téléverser une seconde après
   * l'acceptation. `null` quand il n'y en a réellement aucune : la carte doit
   * rester lisible sans image, et n'en invente jamais une de remplacement.
   */
  cover: AttachedImage | null
  temporal_state: TemporalState
  /** Inscrits et présents ; exclut les annulations. */
  registered_count: number
  /**
   * Codes de la taxonomie `activity_theme`, via `reference.terms_of()`.
   * POUR FILTRER : un tableau de codes se compare et s'indexe. L'AFFICHAGE passe
   * par `themes`, qui porte le libellé et la couleur.
   */
  theme_codes: TaxonomyTermCode[]
  /** Thématiques prêtes à afficher — `reference.term_badges()`. */
  themes: ScheduleThemeBadge[]
}

// ---------------------------------------------------------------------------
// programme.v_proposal_dashboard — la liste des propositions du back-office
// ---------------------------------------------------------------------------

/**
 * Ligne de `programme.v_proposal_dashboard` — avancement des revues, classement,
 * alertes. La vue exclut les dossiers supprimés (`deleted_at`).
 *
 * RAPPEL : cette liste se filtre TOUJOURS par le périmètre d'administration de
 * la personne connectée (`identity.administered_events()`), y compris quand
 * l'utilisateur forge une URL.
 */
export interface ProposalDashboardRow {
  id: ProposalId
  reference_code: string
  event_id: EventId
  call_id: CallId | null
  organization_id: OrganizationId
  /** `org.organizations.legal_name`, déjà joint. */
  organization_name: string
  /** Titre multilingue brut, du même type que `Proposal.title` et
   *  `PublicScheduleRow.title` : à résoudre avec `resolveI18nText()`, comme
   *  partout ailleurs. */
  title: I18nText
  /** Le même titre résolu en base par `platform.t()` (repli français). Réservé
   *  au tri, au filtrage et à l'export ; l'affichage passe par `title`, sans
   *  quoi la liste ne peut pas changer de langue sans requête. */
  title_text: string | null
  status: ProposalStatus
  submitted_at: IsoDateTime | null
  weighted_score: Numeric | null
  average_score: Numeric | null
  is_knocked_out: boolean
  review_count: number
  /** `event.calls_for_proposals.required_reviews` ; nul hors appel. */
  required_reviews: number | null
  /** `required_reviews - review_count`, plancher à zéro ; nul hors appel. */
  reviews_missing: number | null
  /** Révisionnistes affectés, déports exclus. */
  assigned_reviewers: number
  open_change_requests: number
  speaker_count: number
  /** Rang au sein de l'édition, par note pondérée décroissante. */
  event_rank: number

  // --- Ce que la liste du back-office montre et filtre (A7, 18/08) ----------
  // Ces colonnes ont été ajoutées à la vue le 18/08 : elle portait l'avancement
  // des revues et le classement, mais rien de ce qui identifie un dossier dans
  // un tableau de quarante lignes.

  format: ParticipationMode
  /** Code de la taxonomie `activity_category`. */
  activity_type_code: TaxonomyTermCode | null
  organization_acronym: string | null
  /** Pays de l'organisation PORTEUSE, code ISO — pour filtrer et pour le drapeau. */
  organization_country_code: string | null
  /** Le même pays, multilingue : à résoudre à l'affichage. */
  organization_country: I18nText | null
  /** Organisations associées hors porteur principal : la pastille « +2 ». */
  co_organizer_count: number
  /** Codes de thématiques, POUR FILTRER. L'affichage passe par `themes`. */
  theme_codes: TaxonomyTermCode[]
  /** Thématiques prêtes à afficher — `reference.term_badges()`. */
  themes: ScheduleThemeBadge[]
  /** Révisionnistes affectés, déports exclus. Pour filtrer « les dossiers de X ». */
  reviewer_ids: PersonId[]
  /** Les mêmes, nommés : un « 2/3 » ne dit pas de qui on attend la troisième. */
  reviewers: ProposalReviewer[]
  /** Revues attendues dont l'échéance est dépassée — le filtre « en retard ». */
  overdue_reviews: number
  /** Prochaine échéance de revue encore due, toutes affectations confondues. */
  next_review_due_at: IsoDateTime | null
  /** Membres du comité ayant ouvert le dossier. COLLECTIF : « non consulté par
   *  moi » dépend du lecteur et vient de `programme.unread_proposals_for()`. */
  read_count: number
}

/**
 * Une entrée de `v_proposal_dashboard.reviewers` — affectation non déportée,
 * avec l'échéance qui lui est propre et la date de remise de sa revue.
 * `submitted_at` nul : la revue est attendue, ou encore à l'état de brouillon.
 */
export interface ProposalReviewer {
  person_id: PersonId
  /** `identity.people.display_name`, colonne générée. */
  name: string
  due_at: IsoDateTime | null
  submitted_at: IsoDateTime | null
}
