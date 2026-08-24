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
 *   - `content.v_showcase`             — `docs/database/115_content.sql` § 4
 *   - `event.v_public_editions`        — `docs/database/060_events.sql` § 9
 *   - `programme.v_edition_stats`      — `docs/database/075_programme_sessions.sql` § 9
 *
 * Les trois dernières sont ajoutées au prompt A15 : la page d'accueil publique
 * les consomme ensemble — la vitrine, l'historique des éditions et le volume de
 * leur programme.
 */

import type {
  BroadcastChannelId,
  CallId,
  ColorHex,
  CountryId,
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
  Url,
  Uuid,
} from './shared'
import type { EventStatus, ParticipationMode, TrackKind } from './event/edition'
import type { SeriesKind } from './event/series'
import type { CallStatus } from './event/call'
import type { AttachedImage } from './media'
import type { HighlightId, HighlightPlacement } from './content'
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

// ---------------------------------------------------------------------------
// content.v_showcase — la vitrine publique prête à afficher
//
// `docs/database/115_content.sql` § 4. Une requête pour tout le bandeau
// d'accueil : libellé et couleur de la nature, nom de l'auteur, organisation,
// pays, fond, vignette, vidéo — tout est résolu EN BASE. Une colonne qui
// manquerait ici coûterait une requête par diapositive, ou un renoncement
// d'affichage.
//
// LE FILTRE TEMPOREL EST DANS LA VUE, et c'est le point le plus important :
// `status = 'published'` et la fenêtre de diffusion en cours sont déjà
// appliqués. Ne JAMAIS les rejouer côté écran — la v1 laissait chaque composant
// comparer les dates à sa façon, et une annonce périmée survivait là où
// quelqu'un avait oublié la comparaison.
// ---------------------------------------------------------------------------

/**
 * Ligne de `content.v_showcase` — une ligne = une diapositive.
 * Trier par `placement`, puis `sort_order`.
 */
export interface ShowcaseRow {
  id: HighlightId
  placement: HighlightPlacement
  sort_order: number

  /** Code du terme de `highlight_nature` — POUR FILTRER et pour choisir une
   *  icône. L'AFFICHAGE passe par `nature_label` et `nature_color`. */
  nature_code: TaxonomyTermCode
  /** `reference.taxonomy_terms.label`, joint. Nul si le terme a été désactivé —
   *  la jointure est un `LEFT JOIN … AND n.is_active`. DONNÉE multilingue. */
  nature_label: I18nText | null
  /** `taxonomy_terms.color_hex` — la couleur de l'éyclette vient de la BASE. */
  nature_color: ColorHex | null
  nature_icon: string | null

  title: I18nText
  /** Le texte porté en grand sur le fond, déjà coupé par l'éditeur. */
  quote: I18nText | null
  body: I18nText | null

  /** `COALESCE(people.display_name, highlights.author_name)` : le nom du profil
   *  prime, pour qu'une correction de patronyme se répercute sur la vitrine. */
  author_name: string | null
  author_title: I18nText | null
  person_id: PersonId | null

  organization_id: OrganizationId | null
  /** `COALESCE(organizations.legal_name, highlights.organization_label)`. */
  organization_name: string | null
  organization_acronym: string | null

  /** Code ISO 3166-1 alpha-2 : il filtre et porte le drapeau. Résolu depuis
   *  `COALESCE(highlights.country_id, organizations.country_id)` — un
   *  témoignage sans pays explicite hérite de celui de son organisation. */
  country_code: string | null
  /** Le même pays, multilingue : à résoudre à l'affichage. */
  country_name: I18nText | null

  event_id: EventId | null
  event_slug: Slug | null
  event_title: I18nText | null

  session_id: SessionId | null
  session_slug: Slug | null
  session_title: I18nText | null
  session_starts_at: IsoDateTime | null
  session_ends_at: IsoDateTime | null
  /** Fuseau de la séance mise en avant : toute heure affichée le mentionne. */
  session_timezone: TimeZoneName | null

  link_url: Url | null
  link_label: I18nText | null

  /** Fond photographique — rôle `banner`, `media.attached_image()`. */
  background_image: AttachedImage | null
  /**
   * Fond vidéo — rôle `video`, SORTI DU MÊME MÉCANISME bien qu'il ne s'agisse
   * pas d'une image : `media.attached_image()` ne fait aucune hypothèse sur le
   * type MIME. Le front décide de rendre `<video>` ou `<img>` D'APRÈS LE MÉDIA
   * REÇU, jamais d'après une colonne qui prétendrait le savoir.
   *
   * `null` est le cas courant, et pas seulement quand aucune vidéo n'est
   * jointe : un objet encore en traitement n'est pas servi (seul `ready` l'est).
   * Le bandeau se rabat alors sur `background_image`, puis sur
   * `background_color_hex`.
   */
  background_video: AttachedImage | null
  /** Vignette du rail — rôle `cover`. Sert aussi d'affiche à la vidéo. */
  thumbnail: AttachedImage | null
  background_color_hex: ColorHex | null

  /** Codes de la taxonomie `activity_theme`, POUR FILTRER. */
  theme_codes: TaxonomyTermCode[]
  /** Thématiques prêtes à afficher — `reference.term_badges()`. Trois au plus
   *  sur une carte, puis « +N » : au-delà, elles cessent d'informer. */
  themes: ScheduleThemeBadge[]

  /** Bornes de la fenêtre de diffusion. Elles sont DÉJÀ APPLIQUÉES par la vue :
   *  elles servent à l'afficher (« jusqu'au 30 septembre »), pas à re-filtrer. */
  starts_at: IsoDateTime | null
  ends_at: IsoDateTime | null
  published_at: IsoDateTime | null
}

// ---------------------------------------------------------------------------
// event.v_public_editions — l'historique des éditions
//
// `docs/database/060_events.sql` § 9. Filtre déjà appliqué :
// `status NOT IN ('draft','cancelled')`. Trier par `starts_at`.
// ---------------------------------------------------------------------------

/**
 * État temporel d'une ÉDITION — trois valeurs, et pas cinq.
 *
 * `event.v_public_editions` ne compare que `now()` aux bornes de l'édition : les
 * éditions annulées sont écartées par le `WHERE` de la vue, et une édition ne se
 * reporte pas comme une séance. Déclaré à part de `TemporalState` (qui en
 * compte cinq) plutôt que réutilisé : promettre `'cancelled' | 'postponed'` sur
 * une colonne qui ne les produit jamais obligerait chaque écran à écrire deux
 * branches mortes, et laisserait croire qu'une édition peut arriver dans cet
 * état.
 */
export type EditionTemporalState = 'upcoming' | 'ongoing' | 'past'

/** Ligne de `event.v_public_editions` — une ligne = une édition publique. */
export interface PublicEditionRow {
  id: EventId
  slug: Slug
  title: I18nText
  description: I18nText
  acronym: string | null
  /** Libellé de l'édition dans sa série : « COP30 », « PACO 2026 ». */
  edition_label: string | null
  edition_year: number
  status: EventStatus
  participation_mode: ParticipationMode
  /** Fuseau de l'édition : toute heure affichée le mentionne. */
  timezone: TimeZoneName
  starts_at: IsoDateTime
  ends_at: IsoDateTime
  /** L'OIF tient-elle un stand ? Sans pavillon, pas d'appel (règle métier n° 5). */
  has_pavilion: boolean
  /** Non nul dès que la programmation publique est visible, et depuis quand. */
  programme_published_at: IsoDateTime | null
  /** Contenus éditoriaux légers de l'édition (message d'accueil, accès). */
  highlights: I18nText | null

  /** La série situe l'édition dans son cycle : c'est elle, et non une liste de
   *  slugs recopiée dans un composant, qui distingue une COP d'un cycle de
   *  webinaires. Nulle pour un rendez-vous hors série. */
  series_id: Uuid | null
  series_kind: SeriesKind | null
  series_name: I18nText | null
  series_slug: Slug | null

  country_id: CountryId | null
  country_code: string | null
  country_name: I18nText | null
  city: string | null

  /**
   * LES TROIS DÉCLINAISONS, ET NON UNE SEULE À RECADRER (19/08).
   *
   * `banner` en 32:9 pour un bandeau pleine largeur, `cover` en 16:9 pour une
   * carte ou un partage, `thumbnail` en 1:1 pour une liste dense. La vue les
   * rend toutes les trois plutôt que de choisir : l'écran seul sait de quelle
   * largeur il dispose, et un repli décidé en base serait le même pour une
   * fiche pleine largeur et pour une vignette de 48 px.
   *
   * TOUTES SOUVENT NULLES : chaque écran doit rester présentable sans.
   */
  banner: AttachedImage | null
  cover: AttachedImage | null
  thumbnail: AttachedImage | null

  temporal_state: EditionTemporalState

  /** L'APPEL À PROPOSITIONS, EN QUATRE COLONNES ET PAS UNE DE PLUS. Une édition
   *  n'en porte qu'un (`ux_calls_one_per_event`), mais son état ne se déduit ni
   *  de `has_pavilion` ni de l'existence de la ligne. */
  call_id: CallId | null
  call_status: CallStatus | null
  /** `event.is_call_open()` — l'ouverture EFFECTIVE, statut et dates comprises. */
  call_is_open: boolean
  /** `event.effective_deadline()` — l'échéance EFFECTIVE, PROLONGATION COMPRISE.
   *  Ne jamais afficher `submission_deadline` à sa place : la prolongation est
   *  décidée dans le module, et c'est cette date que les organisations tiennent. */
  call_deadline: IsoDateTime | null

  theme_codes: TaxonomyTermCode[]
  themes: ScheduleThemeBadge[]

  /**
   * LE VOLUME DU PROGRAMME PUBLIÉ, joint par l'API — les mêmes colonnes que
   * `EditionStatsRow` ci-dessous.
   *
   * La vue `event.v_public_editions` ne peut pas les porter : le module `event`
   * se charge AVANT `programme`, et la dépendance irait dans le mauvais sens.
   * L'API les joint donc à la lecture, PAR LA GAUCHE — sans quoi une édition
   * annoncée, dont aucune séance n'est encore publiée, disparaîtrait de
   * l'historique. Elles valent alors zéro, ce qui est vrai.
   *
   * Les avoir ici évite un second appel : l'accueil compose ses chiffres à
   * partir de cette liste, sans rien redemander.
   */
  published_session_count: number
  streamed_session_count: number
  organization_count: number
  programme_starts_at: IsoDateTime | null
  programme_ends_at: IsoDateTime | null
}

// ---------------------------------------------------------------------------
// programme.v_edition_stats — le volume du programme publié
// ---------------------------------------------------------------------------

/**
 * Ligne de `programme.v_edition_stats` — `075_programme_sessions.sql`.
 *
 * Elle complète `v_public_editions`, qui ne peut pas dépendre de `programme` :
 * le module `event` se charge AVANT, et la dépendance va dans l'autre sens.
 *
 * PIÈGE : **une édition sans programme publié n'a AUCUNE LIGNE ici.** Le compte
 * ne vaut pas zéro, il est absent. Tout écran doit traiter l'absence comme zéro
 * — d'où l'indexation par `event_id` plutôt qu'une liste à parcourir.
 */
export interface EditionStatsRow {
  event_id: EventId
  /** Séances publiées, annulations exclues. */
  published_session_count: number
  streamed_session_count: number
  /** Organisations DISTINCTES qui animent une séance publiée. */
  organization_count: number
  /** Premier début et dernière fin du programme publié. */
  programme_starts_at: IsoDateTime | null
  programme_ends_at: IsoDateTime | null
}
