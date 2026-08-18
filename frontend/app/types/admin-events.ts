/**
 * GESTION DES ÉVÉNEMENTS, BACK-OFFICE (A10) — contrats des écrans.
 *
 * Dérivé de `docs/database/060_events.sql` § 1 à 7. Aucun champ inventé : chaque
 * valeur affichée est une colonne de `event.events`, `event_days`,
 * `programme_tracks`, `venues`, `rooms`, `broadcast_channels`,
 * `calls_for_proposals`, `review_criteria` ou `call_reviewers` — ou un décompte
 * porté par un autre module et joint ici.
 *
 * ── CE QUI GOUVERNE CES ÉCRANS ──────────────────────────────────────────────
 *
 * UNE SÉRIE, DES ÉDITIONS. `event.event_series` porte l'identité durable (« les
 * COP climat »), `event.events` l'occurrence (« COP30 »). La liste affiche donc
 * la série ET l'édition : deux lignes « COP » sans leur série ne se distinguent
 * que par une année, et c'est ce qui rendait la v1 illisible.
 *
 * UN SEUL APPEL PAR ÉDITION, ZÉRO S'IL N'Y A PAS DE PAVILLON. La cardinalité est
 * 0..1, tenue par l'index `ux_calls_one_per_event` et non par l'application. Le
 * contrat le dit à sa façon : `EditionDetail.call` est un objet ou `null`, jamais
 * un tableau. Aucun écran ne peut donc offrir « ajouter un second appel ».
 *
 * LES JOURNÉES SPÉCIALES NE SONT PAS DES JOURS. `event.event_days` est le
 * CALENDRIER — une ligne par jour, dérivée des dates de l'édition.
 * `event.programme_tracks` porte les journées spéciales, dont la période n'est
 * qu'indicative et dont la COMPOSITION se fait au planificateur (A9), jamais
 * ici. Ces écrans créent le fil ; ils ne lui rattachent aucune séance.
 *
 * ── CE QUI N'EST PAS UNE COLONNE ────────────────────────────────────────────
 *
 * Les décomptes (`proposal_count`, `session_count`, `assigned_count`) sont des
 * jointures, pas des champs de `event.events` : ils vivent dans le contrat de
 * l'ÉCRAN et pas dans `types/event/`, où ils auraient laissé croire que la table
 * les porte. Même règle que `admin-planner.ts` et `admin-proposals.ts`.
 */

import type {
  AssetId,
  BroadcastChannelId,
  CallId,
  ColorHex,
  CountryId,
  CriterionId,
  EventDayId,
  EventId,
  I18nText,
  IsoDate,
  IsoDateTime,
  Numeric,
  PersonId,
  RoomId,
  Slug,
  TimeZoneName,
  TrackId,
  Url,
  Uuid,
  VenueId,
} from './shared'
import type { SeriesKind } from './event/series'
import type { EventStatus, ParticipationMode, TrackKind } from './event/edition'
import type { BroadcastProvider, VenueKind } from './event/venue'
import type { CallStatus } from './event/call'
import type { AttachedImage } from './media'
import type { ScheduleThemeBadge } from './views'

// ===========================================================================
// 1. LISTE DES ÉDITIONS
// ===========================================================================

/**
 * Une ligne de la liste des éditions.
 *
 * ELLE PORTE SA SÉRIE RÉSOLUE, et pas seulement `series_id` : la liste se lit
 * « COP31 — Conférences des Parties climat, 2027 », et une ligne qui n'afficherait
 * que l'année obligerait à connaître par cœur quelle COP tombe quand.
 */
export interface EditionListRow {
  id: EventId
  title: I18nText
  acronym: string | null
  slug: Slug
  /** Nul pour un rendez-vous hors série (`event.events.series_id`). */
  series_id: Uuid | null
  series_name: I18nText | null
  series_kind: SeriesKind | null
  /** Libellé de l'édition dans sa série : « COP30 », « Session 7 ». */
  edition_label: string | null
  edition_year: number
  status: EventStatus
  participation_mode: ParticipationMode
  timezone: TimeZoneName
  starts_at: IsoDateTime
  ends_at: IsoDateTime
  /** `event.events.country_id` — ce que le formulaire réédite. */
  country_id: CountryId | null
  /** `reference.countries.name`, résolu. Nul pour une édition en ligne. */
  country_name: I18nText | null
  city: string | null
  address: string | null
  /** Point relevé du lieu. `ck_events_coordinates` les veut tous deux ou aucun. */
  latitude: Numeric | null
  longitude: Numeric | null
  has_pavilion: boolean
  /** Non nul dès que la programmation publique est visible, et depuis quand. */
  programme_published_at: IsoDateTime | null

  // -- Décomptes joints, jamais des colonnes de `event.events` --------------

  /**
   * Dossiers DÉPOSÉS sur l'appel de cette édition, brouillons exclus.
   *
   * Les brouillons sont écartés ici, à l'inverse de la liste des propositions
   * (A7) qui les montre : la colonne répond à « combien de dossiers cette
   * édition a-t-elle reçus ? », et un brouillon n'a rien été reçu.
   */
  proposal_count: number
  /** Séances de l'édition, placées ou non — ce que le planificateur manipule. */
  session_count: number
  /** Séances effectivement placées en salle : le reste attend un créneau. */
  scheduled_session_count: number
  /** Statut de l'appel, ou `null` quand l'édition n'en porte aucun. */
  call_status: CallStatus | null
  /** Échéance effective de l'appel, prolongation comprise. */
  call_deadline: IsoDateTime | null
  /** Nombre de jours du calendrier déjà créés (`event.event_days`). */
  day_count: number
}

/** Une série proposée au filtre et au formulaire. */
export interface EditionSeriesOption {
  id: Uuid
  name: I18nText
  kind: SeriesKind
  is_active: boolean
  /** Éditions déjà rattachées : ce qui distingue une série vive d'une coquille. */
  edition_count: number
}

/**
 * TOUT L'ÉCRAN DE LA LISTE EN UNE RÉPONSE.
 *
 * Les séries et les années accompagnent les lignes plutôt que d'être demandées à
 * part : les facettes se comptent sur le même jeu de lignes que la liste, sans
 * quoi le « 2027 (4) » du filtre finirait par ne plus correspondre à ce qui
 * s'affiche. Même raison qu'en A7.
 */
export interface EditionListScreen {
  rows: EditionListRow[]
  series: EditionSeriesOption[]
  /** Années présentes dans les lignes, décroissantes. */
  years: number[]
  /** Vrai quand la personne administre la plateforme entière. */
  is_global_scope: boolean
}

/** Colonnes triables de la liste. */
export type EditionSortKey =
  | 'title'
  | 'series'
  | 'edition_year'
  | 'starts_at'
  | 'location'
  | 'status'
  | 'proposal_count'
  | 'programme'

/** Filtres de la liste, portés par l'URL — comme en A7. */
export interface EditionListFilters {
  search: string
  series: Uuid[]
  years: number[]
  statuses: EventStatus[]
  /** `null` : sans importance ; `true` : pavillon tenu ; `false` : sans pavillon. */
  has_pavilion: boolean | null
  /** `null` : sans importance ; `true` : programmation publiée ; `false` : non. */
  published: boolean | null
}

// ===========================================================================
// 2. CRÉATION ET ÉDITION D'UNE ÉDITION
// ===========================================================================

/**
 * Un fuseau proposé au formulaire.
 *
 * `platform.timezone_name` valide un identifiant IANA, sans en tenir la liste :
 * le domaine vérifie l'existence du fuseau dans la base de données de fuseaux de
 * PostgreSQL. La liste offerte à l'écran est donc une COMMODITÉ DE SAISIE, pas un
 * vocabulaire fermé — d'où `Intl.supportedValuesOf('timeZone')` en secours et la
 * possibilité de saisir un identifiant absent de la liste.
 */
export interface TimeZoneOption {
  /** Identifiant IANA : `America/Belem`. */
  value: TimeZoneName
  /** Ville telle qu'elle s'écrit, accents compris : « Belém ». */
  city: string
  /** Décalage courant, pour lever l'ambiguïté entre deux fuseaux voisins. */
  offset_label: string
}

/** Ce qu'il faut savoir avant d'ouvrir le formulaire d'une édition. */
export interface EditionFormOptions {
  series: EditionSeriesOption[]
  countries: { id: CountryId; name: I18nText; iso2: string }[]
  timezones: TimeZoneOption[]
  /** Statuts atteignables, dans l'ordre du cycle de vie de l'édition. */
  statuses: EventStatus[]
}

/**
 * Ce que le formulaire envoie.
 *
 * `starts_at` et `ends_at` sont des instants COMPLETS (`timestamptz`), composés à
 * partir de l'heure locale saisie et du fuseau choisi : la base ne stocke rien
 * d'autre, et laisser passer une date nue reviendrait à décider du fuseau à sa
 * place. C'est la même règle qu'au formulaire de dépôt (A4).
 */
export interface EditionFormPayload {
  /** Nul à la création. */
  id: EventId | null
  series_id: Uuid | null
  edition_label: string | null
  edition_year: number
  title: I18nText
  acronym: string | null
  slug: Slug
  description: I18nText
  status: EventStatus
  participation_mode: ParticipationMode
  timezone: TimeZoneName
  starts_at: IsoDateTime
  ends_at: IsoDateTime
  country_id: CountryId | null
  city: string | null
  address: string | null
  /**
   * COORDONNÉES DU LIEU, facultatives, et indépendantes de l'adresse.
   *
   * Une adresse de parc des expositions ne se géocode pas toujours : « Parc du
   * Hangar, avenida Doutor Freitas » place un marqueur à deux kilomètres du
   * pavillon. L'équipe relève le point sur place. `ck_events_coordinates` exige
   * les DEUX ou AUCUN — une latitude seule ne désigne rien.
   */
  latitude: Numeric | null
  longitude: Numeric | null
  has_pavilion: boolean
  /** Message d'accueil, consignes d'accès — `event.events.highlights`. */
  highlights: I18nText | null
  /**
   * Bannière de l'édition. `event.events` NE PORTE PAS son image : le
   * rattachement média est polymorphe (`media.attachments`, rôle `banner`).
   * L'objet est téléversé d'abord, son identifiant envoyé ensuite.
   */
  banner_asset_id: AssetId | null
}

/**
 * Un refus de sauvegarde, tel que la base le formule.
 *
 * CE NE SONT PAS DES ERREURS DE RÉSEAU MAIS DES RÉPONSES : chaque code
 * correspond à une contrainte nommée de `060_events.sql`, et l'écran les rend au
 * champ concerné. Le code du modèle ne réimplémente pas l'invariant, il traduit
 * l'erreur PostgreSQL en message français exploitable — même consigne côté API.
 */
export type EditionErrorCode =
  /** `ck_events_period` — la fin doit suivre le début. */
  | 'period'
  /** `ck_events_physical_location` — hors ligne, pays et ville sont exigés. */
  | 'physical_location'
  /** `ux_events_slug` — le slug est unique sur toute la plateforme. */
  | 'slug_taken'
  /** `ux_events_series_edition` — cette série a déjà cette année et ce libellé. */
  | 'edition_taken'
  /** `CHECK (edition_year BETWEEN 2000 AND 2100)`. */
  | 'year_range'
  /** `ck_events_coordinates` — latitude et longitude vont ensemble, ou pas du tout. */
  | 'coordinates'
  /** Champ obligatoire non renseigné (`NOT NULL`). */
  | 'required'

export interface EditionFormError {
  code: EditionErrorCode
  /** Champ du formulaire à marquer. `null` quand l'erreur porte sur l'ensemble. */
  field: keyof EditionFormPayload | null
}

/**
 * La réponse d'une sauvegarde d'édition.
 *
 * ELLE DIT CE QUI EST ARRIVÉ AU CALENDRIER. Changer les dates d'une édition
 * ajoute ou retire des jours ; l'écran doit l'annoncer, parce qu'un jour retiré
 * détache les séances qui s'y tenaient (`xmod_fk_sessions_event_day`,
 * `ON DELETE SET NULL`). Une sauvegarde silencieuse laisserait découvrir la
 * conséquence au planificateur, deux écrans plus loin.
 */
export interface EditionSaveResult {
  ok: boolean
  edition: EditionListRow | null
  errors: EditionFormError[]
  /** Jours du calendrier créés par la nouvelle période. */
  days_created: number
  /**
   * Jours retirés parce qu'ils sortent de la période, et séances qu'ils
   * portaient — ces séances ne sont pas supprimées, elles perdent leur jour.
   */
  days_removed: number
  sessions_detached: number
}

// ===========================================================================
// 3. LES ONGLETS DE L'ÉDITION
// ===========================================================================

/** Les six onglets, dans l'ordre où ils s'affichent. */
export type EditionTabKey = 'days' | 'tracks' | 'venues' | 'channels' | 'call' | 'committee'

// --- 3.1 Journées du calendrier -------------------------------------------

/**
 * Un jour du calendrier — `event.event_days`.
 *
 * GÉNÉRÉ DEPUIS LES DATES DE L'ÉDITION, un jour par date de la période. Le titre,
 * le slug, la description, la mise en avant et la couleur sont ÉDITORIAUX : la
 * régénération ne les écrase pas.
 */
export interface EditionDay {
  id: EventDayId
  day_date: IsoDate
  title: I18nText | null
  slug: Slug | null
  description: I18nText | null
  is_featured: boolean
  color_hex: ColorHex | null
  sort_order: number
  /** Séances programmées ce jour-là (`programme.sessions.event_day_id`). */
  session_count: number
  /**
   * Vrai quand la date sort de la période de l'édition — un jour laissé par une
   * période plus large, qu'on peut vouloir garder (une soirée d'ouverture la
   * veille) ou retirer. On le signale ; on ne le supprime pas d'office.
   */
  is_outside_period: boolean
}

/** Ce que la régénération du calendrier va faire, AVANT de le faire. */
export interface DayGenerationPlan {
  /** Dates de la période qui n'ont pas encore de jour. */
  to_create: IsoDate[]
  /** Jours hors période, avec le nombre de séances qu'ils portent. */
  to_review: { id: EventDayId; day_date: IsoDate; session_count: number }[]
  /** Jours déjà en place et dans la période : rien à faire. */
  unchanged: number
}

export interface EditionDayPayload {
  id: EventDayId
  title: I18nText | null
  slug: Slug | null
  description: I18nText | null
  is_featured: boolean
  color_hex: ColorHex | null
}

// --- 3.2 Journées spéciales ------------------------------------------------

/**
 * Un fil de programmation — `event.programme_tracks`.
 *
 * SA COMPOSITION NE SE FAIT PAS ICI. `session_count` est en lecture seule : le
 * rattachement d'une séance à un fil est une décision éditoriale prise au
 * planificateur (A9), dans `programme.session_tracks`. Cet écran crée le fil,
 * l'habille et ouvre sa page publique.
 */
export interface EditionTrack {
  id: TrackId
  code: string
  slug: Slug
  kind: TrackKind
  title: I18nText
  subtitle: I18nText | null
  description: I18nText | null
  /** Portée annoncée au public, PUREMENT INDICATIVE. */
  starts_on: IsoDate | null
  ends_on: IsoDate | null
  color_hex: ColorHex | null
  curated_by: PersonId | null
  /** Nom du responsable, résolu — l'écran n'affiche pas un identifiant. */
  curator_name: string | null
  /** Page publique du fil ; `null` tant qu'elle n'est pas ouverte. */
  published_at: IsoDateTime | null
  sort_order: number
  /** Séances rattachées, composées au planificateur. Lecture seule ici. */
  session_count: number
  /** Thématiques du fil — `reference.entity_terms`, avec libellé et couleur. */
  themes: ScheduleThemeBadge[]
}

export interface EditionTrackPayload {
  /** Nul à la création. */
  id: TrackId | null
  event_id: EventId
  code: string
  slug: Slug
  kind: TrackKind
  title: I18nText
  subtitle: I18nText | null
  description: I18nText | null
  starts_on: IsoDate | null
  ends_on: IsoDate | null
  color_hex: ColorHex | null
  curated_by: PersonId | null
  /** Ouvrir ou refermer la page publique du fil. */
  is_published: boolean
  sort_order: number
}

// --- 3.3 Lieux et salles ---------------------------------------------------

/**
 * Une salle — `event.rooms`.
 *
 * `is_virtual` N'EST PAS UN DÉTAIL : une salle virtuelle accepte des séances
 * simultanées, et `programme.detect_conflicts()` n'y signale aucune double
 * réservation. Cocher cette case sur le stand physique ferait taire le seul
 * conflit que l'équipe doit absolument voir.
 */
export interface EditionRoom {
  id: RoomId
  venue_id: VenueId
  name: I18nText
  code: string
  capacity: number | null
  is_virtual: boolean
  has_streaming: boolean
  equipment: string[]
  sort_order: number
  /** Séances placées dans cette salle : ce qu'un retrait déplacerait. */
  session_count: number
}

/** Un lieu et ses salles — `event.venues`. */
export interface EditionVenue {
  id: VenueId
  name: I18nText
  kind: VenueKind
  address: string | null
  map_url: Url | null
  rooms: EditionRoom[]
}

export interface EditionVenuePayload {
  id: VenueId | null
  event_id: EventId
  name: I18nText
  kind: VenueKind
  address: string | null
  map_url: Url | null
}

export interface EditionRoomPayload {
  id: RoomId | null
  venue_id: VenueId
  name: I18nText
  code: string
  capacity: number | null
  is_virtual: boolean
  has_streaming: boolean
  equipment: string[]
  sort_order: number
}

// --- 3.4 Canaux de diffusion ----------------------------------------------

/**
 * Un canal de diffusion — `event.broadcast_channels`.
 *
 * RESSOURCE RÉSERVABLE, au même titre qu'une salle : la règle « un seul direct à
 * la fois » se joue là. Un canal PAR DÉFAUT est donc indispensable — sans lui,
 * une séance marquée « diffusée » n'occupe aucun canal et échappe à la
 * détection. L'index `ux_broadcast_channels_default` n'en autorise qu'un par
 * édition ; l'écran le pose, il ne le laisse pas deviner.
 */
export interface EditionChannel {
  id: BroadcastChannelId
  /** Nul pour un canal GÉNÉRAL de la plateforme, hors édition : non modifiable ici. */
  event_id: EventId | null
  code: string
  name: I18nText
  provider: BroadcastProvider
  /** Compte diffuseur, ex. `@ifddoif`. */
  channel_ref: string | null
  locale: string | null
  is_default: boolean
  is_active: boolean
  /** Séances diffusées sur ce canal. */
  session_count: number
}

export interface EditionChannelPayload {
  id: BroadcastChannelId | null
  event_id: EventId
  code: string
  name: I18nText
  provider: BroadcastProvider
  channel_ref: string | null
  locale: string | null
  is_default: boolean
  is_active: boolean
}

// --- 3.5 Appel à propositions ---------------------------------------------

/**
 * Un critère de la grille — `event.review_criteria`.
 *
 * `is_knockout` DISQUALIFIE : une note nulle sur un critère éliminatoire écarte
 * la proposition quelle que soit la moyenne. C'est la seule case de cette grille
 * dont l'oubli change une décision.
 */
export interface EditionCriterion {
  /** Nul pour une ligne ajoutée et pas encore enregistrée. */
  id: CriterionId | null
  code: string
  label: I18nText
  description: I18nText | null
  max_score: Numeric
  weight: Numeric
  is_knockout: boolean
  sort_order: number
  /** Notes déjà posées sur ce critère : ce qu'une modification de barème rendrait faux. */
  score_count: number
}

/**
 * L'appel à propositions de l'édition — `event.calls_for_proposals`.
 *
 * UN SEUL PAR ÉDITION. Ce n'est pas un tableau, et l'écran n'offre aucun bouton
 * « ajouter un appel » quand il en existe déjà un non annulé.
 */
export interface EditionCall {
  id: CallId
  event_id: EventId
  code: string
  title: I18nText
  description: I18nText | null
  status: CallStatus
  opens_at: IsoDateTime
  closes_at: IsoDateTime
  extended_until: IsoDateTime | null
  results_expected_at: IsoDate | null
  max_proposals_per_organization: number | null
  requires_verified_organization: boolean
  min_speakers: number
  max_speakers: number
  default_duration_minutes: number
  min_duration_minutes: number
  max_duration_minutes: number
  /** Plage d'accueil du pavillon, `HH:MM:SS`, en heure LOCALE de l'édition. */
  daily_start_time: string
  daily_end_time: string
  allowed_formats: ParticipationMode[]
  required_reviews: number
  blind_review: boolean
  guidelines_url: Url | null

  // -- Dérivés, jamais des colonnes ---------------------------------------

  /** `event.effective_deadline()` — `extended_until ?? closes_at`. */
  effective_deadline: IsoDateTime
  /** `event.is_call_open()` — statut ET fenêtre, pas seulement le statut. */
  is_open: boolean
  /** `event.max_weighted_score()` — la note maximale atteignable. */
  max_weighted_score: number
  /** Dossiers déposés sur cet appel, brouillons exclus. */
  proposal_count: number
  criteria: EditionCriterion[]
}

/**
 * Ce que le formulaire de l'appel envoie — la grille COMPRISE.
 *
 * La grille part avec l'appel et non par un appel séparé : un appel sans critère
 * ne peut recevoir aucune évaluation, et deux enregistrements distincts
 * laisseraient exister cet état le temps d'un oubli.
 */
export interface EditionCallPayload {
  id: CallId | null
  event_id: EventId
  code: string
  title: I18nText
  description: I18nText | null
  status: CallStatus
  opens_at: IsoDateTime
  closes_at: IsoDateTime
  extended_until: IsoDateTime | null
  results_expected_at: IsoDate | null
  max_proposals_per_organization: number | null
  requires_verified_organization: boolean
  min_speakers: number
  max_speakers: number
  default_duration_minutes: number
  min_duration_minutes: number
  max_duration_minutes: number
  daily_start_time: string
  daily_end_time: string
  allowed_formats: ParticipationMode[]
  required_reviews: number
  blind_review: boolean
  guidelines_url: Url | null
  criteria: EditionCriterion[]
}

/** Contraintes nommées de `event.calls_for_proposals`, telles qu'elles refusent. */
export type CallErrorCode =
  /** `ck_calls_window` — la clôture doit suivre l'ouverture. */
  | 'window'
  /** `ck_calls_extension` — une prolongation dépasse la clôture initiale. */
  | 'extension'
  /** `ck_calls_speakers` — le maximum doit atteindre le minimum. */
  | 'speakers'
  /** `ck_calls_duration_bounds` — bornes de durée incohérentes, ou défaut hors bornes. */
  | 'duration_bounds'
  /** `ck_calls_daily_window` — la fermeture du pavillon doit suivre son ouverture. */
  | 'daily_window'
  /** `ux_calls_one_per_event` — cette édition porte déjà un appel non annulé. */
  | 'already_exists'
  /** `ux_calls_code` — ce code est déjà pris sur cette édition. */
  | 'code_taken'
  /** Une grille vide ne peut évaluer aucun dossier. */
  | 'criteria_empty'
  /** Deux critères portent le même code (`ux_review_criteria`). */
  | 'criterion_code_duplicate'
  /** Champ obligatoire non renseigné. */
  | 'required'

export interface CallFormError {
  code: CallErrorCode
  field: string | null
  /** Rang du critère fautif, quand l'erreur porte sur une ligne de la grille. */
  criterion_index: number | null
}

export interface CallSaveResult {
  ok: boolean
  call: EditionCall | null
  errors: CallFormError[]
  /**
   * Vrai quand la grille a changé alors que des notes existaient déjà. Les notes
   * ne sont pas perdues — `programme.review_scores` référence le critère — mais
   * les moyennes se recalculent : l'écran doit le dire, pas le taire.
   */
  scores_affected: boolean
}

// --- 3.6 Comité de sélection ----------------------------------------------

/**
 * Un membre du comité — `event.call_reviewers`.
 *
 * CETTE TABLE DIT LA COMPOSITION, PAS LE DROIT D'ACCÈS. L'autorisation reste
 * portée par `identity.role_assignments` sur la portée `event` : siéger au comité
 * n'accorde rien, et l'écran ne doit pas laisser croire qu'ajouter quelqu'un ici
 * lui ouvre les dossiers.
 */
export interface EditionCommitteeMember {
  person_id: PersonId
  full_name: string
  email: string
  organization_name: string | null
  is_lead: boolean
  /** Plafond INDICATIF de dossiers confiés à ce membre. */
  workload_cap: number | null
  added_at: IsoDateTime
  /** Dossiers effectivement confiés, déports exclus. */
  assigned_count: number
  /** Revues déjà rendues sur ces dossiers. */
  submitted_count: number
  /** Vrai quand la personne détient bien `programme.proposal.review` sur l'édition. */
  has_review_permission: boolean
}

/** Une personne que l'on peut ajouter au comité. */
export interface CommitteeCandidate {
  person_id: PersonId
  full_name: string
  email: string
  organization_name: string | null
  /** Détient-elle déjà la permission d'évaluer sur cette édition ? */
  has_review_permission: boolean
}

export interface CommitteePayload {
  call_id: CallId
  members: { person_id: PersonId; is_lead: boolean; workload_cap: number | null }[]
}

export interface CommitteeSaveResult {
  ok: boolean
  members: EditionCommitteeMember[]
  /**
   * Membres retirés qui portaient encore des affectations. Le retrait n'annule
   * pas les revues déjà rendues : elles restent au dossier, comme le veut
   * l'historique opposable du modèle.
   */
  removed_with_assignments: { full_name: string; assigned_count: number }[]
}

// --- 3.7 La composition de l'écran de détail -------------------------------

/**
 * TOUT L'ÉCRAN DE DÉTAIL EN UNE RÉPONSE — l'édition, ses six onglets et les
 * listes de référence dont ses formulaires ont besoin.
 *
 * UNE COMPOSITION, PAS DOUZE LECTURES. Ouvrir l'onglet « Appel » ne doit pas
 * attendre un aller-retour : les six onglets d'une même édition tiennent
 * largement en une réponse, et l'équipe passe de l'un à l'autre sans arrêt en
 * préparant une COP. La contrepartie est assumée — un enregistrement dans un
 * onglet rafraîchit la composition entière, ce qui garantit que les décomptes
 * des cinq autres restent justes.
 */
export interface EditionDetail {
  edition: EditionListRow
  /**
   * Les deux textes longs de l'édition, portés ICI et non sur `EditionListRow`.
   *
   * Une ligne de tableau à huit colonnes n'a pas à charger deux paragraphes par
   * édition ; le formulaire de modification, lui, en a besoin. Les demander au
   * détail plutôt qu'à la liste évite l'un et l'autre défaut — une liste alourdie,
   * ou un appel de plus au moment d'ouvrir le formulaire.
   */
  description: I18nText
  highlights: I18nText | null
  /** Période de l'édition en dates civiles, dans son fuseau : ce que les onglets bornent. */
  period: { first_day: IsoDate; last_day: IsoDate }
  /** `media.attached_image('event','events',id,'banner')`. */
  banner: AttachedImage | null
  days: EditionDay[]
  tracks: EditionTrack[]
  venues: EditionVenue[]
  channels: EditionChannel[]
  /** `null` quand l'édition ne porte aucun appel — sans pavillon, il n'y en a pas. */
  call: EditionCall | null
  committee: EditionCommitteeMember[]
  /** Personnel de l'IFDD assignable comme responsable d'un fil. */
  curators: CommitteeCandidate[]
  /** Personnes que l'on peut appeler au comité, membres actuels exclus. */
  committee_candidates: CommitteeCandidate[]
  /** Thématiques disponibles pour habiller un fil (`reference.taxonomy_terms`). */
  available_themes: ScheduleThemeBadge[]
}

/** Réponse commune des écritures d'onglet : la composition, recalculée. */
export interface EditionTabResult {
  ok: boolean
  detail: EditionDetail | null
  /**
   * Séances détachées par l'écriture — salle retirée, jour supprimé, canal
   * désactivé. Toutes ces clés étrangères sont `ON DELETE SET NULL` : la séance
   * survit et perd son rattachement. L'écran l'annonce.
   */
  sessions_detached: number
  /** Contrainte refusée, le cas échéant. */
  error_code: string | null
}
