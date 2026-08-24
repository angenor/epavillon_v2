/**
 * ACCUEIL PUBLIC (A15) — contrat de l'écran `/`.
 *
 * Ce fichier ne décrit AUCUNE table : il décrit ce que l'API rend à la page
 * d'accueil, en une fois. Les tables sont dans `types/content.ts`, les lignes de
 * vue dans `types/views.ts`, et le back-office de la vitrine dans
 * `types/admin-showcase.ts`. Même règle qu'en A3, A6 et A10.
 *
 * ── CE QUI GOUVERNE CET ÉCRAN ───────────────────────────────────────────────
 *
 * UNE REQUÊTE, PAS SIX. L'accueil montre le bandeau, la colonne « À venir »,
 * l'appel en cours et l'historique des éditions. Chacun a sa vue en base ; les
 * assembler côté client, c'est six états de chargement à composer et six
 * occasions de désynchronisation. `HomeScreen` est donc la réponse ENTIÈRE.
 *
 * L'ÉDITION EN COURS SE CHOISIT PAR LES DONNÉES, jamais par une constante :
 * la première édition à pavillon non terminée par `starts_at`, à défaut la plus
 * récente. C'était déjà la logique de l'ancien `pages/index.vue`, qui
 * redirigeait ; elle survit à la révocation de cette redirection.
 *
 * RIEN N'EST RE-FILTRÉ ICI. `content.v_showcase` applique déjà le statut et la
 * fenêtre de diffusion ; `event.v_public_editions` écarte déjà les brouillons et
 * les annulations. Un écran qui rejouerait ces filtres finirait par en oublier
 * un — c'est exactement ce qui laissait survivre les annonces périmées en v1.
 */

import type { EventId, IsoDateTime } from './shared'
import type {
  EditionStatsRow,
  EditionTemporalState,
  PublicEditionRow,
  PublicScheduleRow,
  ShowcaseRow,
} from './views'

// ===========================================================================
// 1. LA PAGE D'ACCUEIL, EN UNE RÉPONSE
// ===========================================================================

/**
 * Tout ce que `/` affiche. Les tableaux sont triés côté API : le bandeau par
 * `sort_order`, les séances et les éditions par date. Un écran ne retrie pas.
 */
export interface HomeScreen {
  /** `content.v_showcase`, `placement = 'home_hero'`, par `sort_order`. Le
   *  bandeau qui défile. Tableau VIDE possible : la page reste entière, elle
   *  s'ouvre alors sur l'appel à propositions. */
  hero: ShowcaseRow[]
  /** `programme.v_public_schedule` — `temporal_state` valant `'upcoming'` ou
   *  `'ongoing'`, TOUTES ÉDITIONS CONFONDUES, annulations exclues, les six
   *  premières par `starts_at`. Chaque heure porte son fuseau. */
  upcomingSessions: PublicScheduleRow[]
  /** `event.v_public_editions`, l'historique COMPLET, par `starts_at`
   *  décroissant. Le filtre de période s'applique à l'affichage — voir
   *  `editionHistory()` — et non à cette liste, pour que les onglets puissent
   *  annoncer leurs décomptes sans requête. */
  editions: PublicEditionRow[]
  /**
   * `programme.v_edition_stats`, INDEXÉE par `event_id`.
   *
   * Une édition sans programme publié n'a AUCUNE ligne dans la vue : la clé est
   * absente, et l'absence vaut zéro. C'est la forme qui rend cette règle
   * difficile à oublier — une liste obligerait à chercher, et « pas trouvé »
   * se serait vite transformé en tiret à l'écran.
   */
  stats: Record<EventId, EditionStatsRow>
  /**
   * L'édition dont l'accueil présente l'appel, section `#appel-a-propositions`.
   *
   * `null` quand aucune édition ne tient de pavillon. Non nulle mais sans
   * `call_id` quand l'appel n'est pas encore ouvert : la section s'efface alors,
   * mais L'ANCRE DOIT RESTER — le pied de page y renvoie depuis
   * `app/layouts/public.vue`.
   */
  currentEdition: PublicEditionRow | null
  /** Instant de composition de la réponse. Sert à dater « à venir » et « en
   *  cours » sans supposer que l'horloge du navigateur est juste. */
  generated_at: IsoDateTime
}

// ===========================================================================
// 2. L'HISTORIQUE DES ÉVÉNEMENTS
// ===========================================================================

/**
 * Le filtre segmenté de l'historique, porté par l'URL (`?periode=`) comme
 * partout dans ce projet.
 *
 * `'all'` n'est pas une valeur de `EditionTemporalState` : c'est l'absence de
 * filtre. Les trois autres reprennent la colonne telle quelle, sans traduction
 * intermédiaire — un `switch` qui remapperait ces codes serait le premier
 * endroit à diverger de la base.
 */
export type EditionPeriod = 'all' | EditionTemporalState

/** Les éditions d'un même millésime. Le millésime sert de repère de colonne. */
export interface EditionHistoryGroup {
  year: number
  editions: PublicEditionRow[]
}

/** Ce que chaque onglet annonce, calculé sur l'ensemble NON FILTRÉ. */
export interface EditionHistoryCounts {
  all: number
  upcoming: number
  ongoing: number
  past: number
}

/**
 * L'historique tel que la section le rend : groupé par année décroissante,
 * avec les décomptes des onglets et le volume de programme de chaque édition.
 */
export interface EditionHistory {
  period: EditionPeriod
  /** Années décroissantes ; à l'intérieur, éditions par `starts_at` décroissant. */
  groups: EditionHistoryGroup[]
  counts: EditionHistoryCounts
  /** Même indexation, même règle d'absence que `HomeScreen.stats`. */
  stats: Record<EventId, EditionStatsRow>
}
