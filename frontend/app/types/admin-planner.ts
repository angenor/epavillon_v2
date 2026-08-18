/**
 * LE PLANIFICATEUR DE CRÉNEAUX (A9) — contrats de l'écran.
 *
 * Dérivé de `docs/database/075_programme_sessions.sql` § 1, 2, 2 bis, 7 et de
 * `060_events.sql` § 3, 3 bis, 4, 4 bis. Aucun champ inventé : ce que l'écran
 * affiche existe en base, et ce qu'il écrit est une colonne de
 * `programme.sessions` ou une ligne de `programme.session_tracks`.
 *
 * ── CE QUI GOUVERNE TOUT L'ÉCRAN ────────────────────────────────────────────
 *
 * LES CHEVAUCHEMENTS NE SONT JAMAIS BLOQUÉS. Aucune contrainte d'exclusion
 * n'existe sur les créneaux, aucune de ces écritures ne peut échouer pour cause
 * de conflit, et aucun type d'ici ne permet d'exprimer un refus de placement.
 * Les organisations ont proposé leurs créneaux sans se coordonner ; c'est
 * l'équipe qui réorganise, par déplacements successifs, en passant par des états
 * incohérents. `detect_conflicts()` recense, l'écran montre.
 *
 * LE SEUL GARDE-FOU DUR EST LA PUBLICATION : `publication_readiness()` liste ce
 * qui doit être réglé avant de rendre la programmation publique, et c'est le
 * seul endroit de cet écran où un bouton peut refuser d'agir.
 *
 * ── L'UNITÉ MANIPULÉE EST LA SÉANCE, PAS LE DOSSIER ─────────────────────────
 *
 * Une séance existe dès que le dossier est retenu, avec le créneau souhaité par
 * l'organisation et SANS SALLE. « Placer » une activité, c'est lui attribuer une
 * salle et arrêter son créneau ; « la retirer », c'est lui reprendre sa salle,
 * pas la supprimer. Le panneau latéral liste donc les séances dont `room_id` est
 * nul, et le calendrier les autres.
 */

import type {
  BroadcastChannelId,
  EventDayId,
  EventId,
  I18nText,
  IsoDate,
  IsoDateTime,
  OrganizationId,
  ProposalId,
  RoomId,
  SessionId,
  Slug,
  TimeZoneName,
  TrackId,
} from './shared'
import type { ParticipationMode } from './event/edition'
import type { SessionStatus, ScheduleConflict, PublicationReadinessIssue } from './programme/session'
import type { ScheduleThemeBadge } from './views'

// ---------------------------------------------------------------------------
// Ce que l'écran lit
// ---------------------------------------------------------------------------

/**
 * Une séance telle que le planificateur la manipule — placée ou non.
 *
 * ELLE PORTE TOUT CE QU'UN BLOC ET UNE CARTE AFFICHENT, déjà joint : sans quoi
 * chaque bloc du calendrier coûterait une requête pour son organisation, une
 * pour sa note et une pour ses thématiques. C'est la même règle que
 * `v_public_schedule`, qui n'aide pas ici — elle ne montre que le publié, et cet
 * écran travaille d'abord sur ce qui ne l'est pas.
 */
export interface PlannerSession {
  id: SessionId
  event_id: EventId
  /** Nul quand l'IFDD programme directement, sans passer par l'appel. */
  proposal_id: ProposalId | null
  event_day_id: EventDayId | null
  title: I18nText
  slug: Slug
  summary: I18nText | null
  status: SessionStatus
  format: ParticipationMode
  starts_at: IsoDateTime
  ends_at: IsoDateTime
  timezone: TimeZoneName

  /** Nul tant que la séance n'est pas placée : c'est ce qui la range au panneau. */
  room_id: RoomId | null
  room_name: I18nText | null
  /** Dérivée de `event.rooms.is_virtual` : une salle virtuelle n'occupe pas le stand. */
  enforce_room_exclusivity: boolean
  location_note: I18nText | null

  organization_id: OrganizationId | null
  organization_name: string | null
  organization_acronym: string | null
  /** Code ISO 3166-1 alpha-2 : il situe l'organisation aussi sûrement que son nom. */
  organization_country_code: string | null

  /** Numéro lisible du dossier d'origine (« COP31-00147 »), nul sans dossier. */
  reference_code: string | null
  /** Note consolidée du dossier — c'est par elle que le panneau se trie. */
  average_score: number | null
  /** Durée souhaitée au dépôt, en minutes. Sert de longueur par défaut au bloc. */
  requested_duration_minutes: number | null
  /** Créneau souhaité par l'organisation, à distinguer du créneau retenu. */
  preferred_start_at: IsoDateTime | null
  /** Contraintes déclarées au dépôt (« pas le matin », « après la plénière »). */
  scheduling_constraints: string | null

  is_streamed: boolean
  broadcast_channel_id: BroadcastChannelId | null

  /** Journées spéciales auxquelles l'équipe l'a RATTACHÉE — jamais déduit des dates. */
  track_ids: TrackId[]
  /** Thématiques du dossier, libellé et couleur venus de la base. */
  themes: ScheduleThemeBadge[]
  /** Nombre d'intervenants déclarés : `publication_readiness()` s'en inquiète. */
  speaker_count: number
  published_at: IsoDateTime | null
}

/** Salle offerte au placement, avec ce qui décide du choix. */
export interface PlannerRoom {
  id: RoomId
  name: I18nText
  code: string
  capacity: number | null
  /** Une salle virtuelle accepte les créneaux simultanés, sans conflit. */
  is_virtual: boolean
  has_streaming: boolean
  sort_order: number
}

/** Journée du calendrier de l'édition — les colonnes de jours de l'écran. */
export interface PlannerDay {
  id: EventDayId
  day_date: IsoDate
  title: I18nText | null
  is_featured: boolean
  color_hex: string | null
}

/** Journée spéciale offerte au rattachement (`event.programme_tracks`). */
export interface PlannerTrack {
  id: TrackId
  title: I18nText
  kind: 'special_day' | 'thematic_track' | 'side_programme'
  color_hex: string | null
  starts_on: IsoDate | null
  ends_on: IsoDate | null
}

/** Canal de diffusion — ressource réservable : un seul direct à la fois. */
export interface PlannerChannel {
  id: BroadcastChannelId
  name: I18nText
  provider: string
  is_default: boolean
}

/**
 * TOUT L'ÉCRAN EN UNE RÉPONSE.
 *
 * Les conflits en font partie et ne sont pas un appel séparé : un planificateur
 * qui afficherait sa grille avant de savoir ce qui s'y chevauche montrerait,
 * pendant une seconde, une programmation qui a l'air saine.
 */
export interface PlannerScreen {
  event_id: EventId
  event_title: I18nText
  /** Fuseau de l'ÉDITION : c'est lui qui place les blocs, jamais celui du poste. */
  timezone: TimeZoneName
  /** Nom de la ville hôte — « heure de Belém » plutôt que « heure de Belem ». */
  zone_label: string | null
  /** La programmation est-elle déjà publique ? Change le libellé du bouton. */
  programme_published_at: IsoDateTime | null

  days: PlannerDay[]
  rooms: PlannerRoom[]
  tracks: PlannerTrack[]
  channels: PlannerChannel[]

  /** Séances installées dans une salle : les blocs du calendrier. */
  placed: PlannerSession[]
  /** Séances retenues sans salle : le panneau latéral, et son compteur. */
  unplaced: PlannerSession[]

  conflicts: ScheduleConflict[]
}

// ---------------------------------------------------------------------------
// Filtres et tri du panneau latéral
// ---------------------------------------------------------------------------

/** Clés de tri du panneau. La note d'abord : c'est l'ordre du comité. */
export type UnplacedSortKey = 'score' | 'duration' | 'preferred' | 'title'

export interface UnplacedFilters {
  search: string
  themes: string[]
  formats: ParticipationMode[]
  organizations: OrganizationId[]
}

/** Valeur d'un filtre, avec son décompte : une facette vide ne s'offre pas. */
export interface PlannerFacet {
  value: string
  label: string
  count: number
  /** Couleur venue de la base (thématique), rendue en point. */
  color?: string | null
}

export interface UnplacedFacets {
  themes: PlannerFacet[]
  formats: PlannerFacet[]
  organizations: PlannerFacet[]
}

// ---------------------------------------------------------------------------
// Ce que l'écran écrit
// ---------------------------------------------------------------------------

/**
 * PLACER, DÉPLACER, REDIMENSIONNER — une seule écriture pour les trois.
 *
 * La base n'en distingue pas : ce sont les colonnes `room_id`, `starts_at` et
 * `ends_at` de `programme.sessions`. Trois appels différents auraient donné
 * trois occasions de diverger sur la détection des conflits, qui est justement
 * ce que l'écran doit rendre identique dans les trois gestes.
 *
 * `room_id` nul RETIRE la séance du calendrier et la renvoie au panneau. Ce
 * n'est pas une suppression : la séance existe, son créneau souhaité reste.
 */
export interface ScheduleSessionPayload {
  session_id: SessionId
  room_id: RoomId | null
  starts_at: IsoDateTime
  ends_at: IsoDateTime
}

/**
 * RATTACHEMENT AUX JOURNÉES SPÉCIALES — `programme.session_tracks`.
 *
 * MANUEL ET INDÉPENDANT DE LA DATE : toutes les activités du 12 novembre ne
 * relèvent pas de la « Journée finance durable ». La liste envoyée remplace la
 * précédente ; la base trace qui a rattaché quoi (`added_by`), parce qu'il
 * arrive d'avoir à l'expliquer à une organisation qui s'étonne de ne pas y
 * figurer.
 */
export interface SessionTracksPayload {
  session_id: SessionId
  track_ids: TrackId[]
}

/**
 * DIFFUSION — `sessions.is_streamed` et `broadcast_channel_id`.
 *
 * Le canal n'est pas facultatif dans les faits : la base pose le canal par
 * défaut de l'édition dès que la diffusion est activée, faute de quoi une séance
 * marquée « diffusée » sans canal échapperait à la règle « un seul direct ».
 * L'écran laisse le choix quand l'édition en a plusieurs, et n'invente rien
 * quand elle n'en a qu'un.
 */
export interface SessionBroadcastPayload {
  session_id: SessionId
  is_streamed: boolean
  broadcast_channel_id: BroadcastChannelId | null
}

/**
 * Réponse commune aux écritures de l'écran : la séance telle qu'elle est
 * devenue, ET les conflits recalculés pour toute l'édition.
 *
 * LES DEUX ENSEMBLE, TOUJOURS. Déplacer un bloc peut résoudre le conflit d'un
 * autre bloc à l'autre bout de la semaine ; ne rendre que la séance modifiée
 * laisserait le bandeau afficher un conflit qui n'existe plus.
 */
export interface PlannerMutationResult {
  session: PlannerSession
  conflicts: ScheduleConflict[]
}

/**
 * PUBLIER LA PROGRAMMATION — le seul contrôle bloquant de l'écran.
 *
 * `blocked` vaut vrai dès qu'il reste un point de gravité `blocking` : rien
 * n'est publié, et la liste dit quoi régler. Les avertissements, eux, ne
 * retiennent pas la publication — ils l'accompagnent.
 */
export interface PublishProgrammeResult {
  blocked: boolean
  /** Nombre de séances rendues publiques. Zéro quand `blocked` vaut vrai. */
  published_count: number
  published_at: IsoDateTime | null
  issues: PublicationReadinessIssue[]
}
