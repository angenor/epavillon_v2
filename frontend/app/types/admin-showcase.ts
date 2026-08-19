/**
 * BACK-OFFICE DE LA VITRINE (A15) — contrats des trois écrans
 * `/admin/vitrine`, `/admin/vitrine/nouveau`, `/admin/vitrine/[id]`.
 *
 * Dérivé de `docs/database/115_content.sql`. Aucun champ inventé : chaque valeur
 * est une colonne de `content.highlights`, une colonne de `content.v_showcase`,
 * ou un état recomposé qui porte ici son nom et sa justification.
 *
 * Fichier séparé de `types/home.ts` : l'unité de découpage du projet est
 * l'ÉCRAN, et la vitrine publique et sa gestion n'en sont pas le même.
 *
 * ── LES TROIS RÈGLES QUI GOUVERNENT CES ÉCRANS ──────────────────────────────
 *
 * L'ORDRE EST LA FONCTION PRINCIPALE. Son absence était le défaut n° 6 de la
 * v1 : le carrousel suivait `created_at DESC` et l'IFDD ne décidait pas ce qui
 * passe en premier. Le réordonnancement se fait par des boutons monter /
 * descendre (44 px) et non par un glisser-déposer seul — il doit rester
 * utilisable au clavier.
 *
 * LE PÉRIMÈTRE D'ADMINISTRATION PASSE PAR `event_id` (ADR-14, règle n° 8). Une
 * diapositive rattachée à une édition n'appartient qu'à qui administre cette
 * édition. Une diapositive SANS édition parle de la plateforme entière : elle
 * demande la PORTÉE GLOBALE. Le filtre s'applique aussi quand l'URL est forgée
 * à la main — d'où `ShowcaseListScreen.is_global_scope`, qui dit à l'écran s'il
 * peut seulement OFFRIR de créer un contenu de plateforme.
 *
 * LE STATUT NE SUFFIT PAS À DIRE CE QUI EST À L'ÉCRAN. `published` plus une
 * fenêtre passée, c'est un contenu invisible que le back-office doit montrer
 * comme tel : voir `ShowcaseBroadcastState`.
 */

import type {
  ColorHex,
  CountryId,
  EventId,
  I18nText,
  IsoDateTime,
  OrganizationId,
  PersonId,
  SessionId,
  Slug,
  TaxonomyTermCode,
  TimeZoneName,
  Url,
} from './shared'
import type {
  HighlightId,
  HighlightMediaRule,
  HighlightPlacement,
  HighlightStatus,
} from './content'
import type { AttachedImage } from './media'
import type { ScheduleThemeBadge, ShowcaseRow } from './views'

// ===========================================================================
// 1. L'ÉTAT DE DIFFUSION
// ===========================================================================

/**
 * Ce que la vitrine fait RÉELLEMENT de la diapositive, statut et fenêtre
 * combinés — l'équivalent de `live.event_incidents()` pour les incidents (A13).
 *
 *   `live`       publiée, fenêtre en cours (ou sans limite) → elle est à l'écran
 *   `scheduled`  publiée, `starts_at` encore à venir        → elle s'allumera
 *   `expired`    publiée, `ends_at` dépassé                 → elle s'est éteinte
 *   `draft`      brouillon                                  → jamais sortie
 *   `archived`   retirée, conservée, réutilisable
 *
 * Seul `live` correspond à une ligne de `content.v_showcase`. Les quatre autres
 * existent en base et n'apparaissent nulle part au public : c'est précisément ce
 * que le back-office doit rendre visible, sans quoi l'éditeur ne comprend pas
 * pourquoi son témoignage « publié » ne s'affiche pas.
 */
export type ShowcaseBroadcastState = 'live' | 'scheduled' | 'expired' | 'draft' | 'archived'

// ===========================================================================
// 2. LA LISTE
// ===========================================================================

/**
 * Une ligne du tableau dense de `/admin/vitrine`.
 *
 * Elle porte l'attribution et le rattachement DÉJÀ RÉSOLUS, comme
 * `content.v_showcase` : une colonne « édition » qui n'afficherait que
 * `event_id` obligerait à connaître les identifiants par cœur.
 */
export interface ShowcaseListRow {
  id: HighlightId
  placement: HighlightPlacement
  status: HighlightStatus
  /** État réel de diffusion — voir `ShowcaseBroadcastState`. */
  broadcast_state: ShowcaseBroadcastState
  sort_order: number

  nature_code: TaxonomyTermCode
  /** DONNÉE multilingue de `reference.taxonomy_terms`, jamais une clé i18n. */
  nature_label: I18nText | null
  /** Couleur de l'éyclette, venue de la base. */
  nature_color: ColorHex | null
  nature_icon: string | null

  title: I18nText

  /** `COALESCE(people.display_name, author_name)`, comme la vue publique. */
  author_name: string | null
  author_title: I18nText | null
  /** `COALESCE(organizations.legal_name, organization_label)`. */
  organization_name: string | null
  organization_acronym: string | null
  country_name: I18nText | null

  /** `null` = contenu de PLATEFORME : il n'apparaît qu'en portée globale. */
  event_id: EventId | null
  event_title: I18nText | null
  event_slug: Slug | null
  session_id: SessionId | null
  session_title: I18nText | null

  /** Vignette du rail (rôle `cover`). Repli d'affichage dans la colonne :
   *  `thumbnail`, à défaut `background_image`, à défaut l'aplat. */
  thumbnail: AttachedImage | null
  background_image: AttachedImage | null
  /** Vrai quand un objet vidéo PRÊT est rattaché. Un objet encore en traitement
   *  ne compte pas : le bandeau se rabattrait sur l'image, et la liste doit dire
   *  ce que le public voit. */
  has_video: boolean
  background_color_hex: ColorHex | null

  starts_at: IsoDateTime | null
  ends_at: IsoDateTime | null
  published_at: IsoDateTime | null
  updated_at: IsoDateTime

  /** Première / dernière de son EMPLACEMENT : les boutons monter et descendre
   *  se désactivent d'après ces deux drapeaux, calculés là où l'ordre est connu. */
  is_first: boolean
  is_last: boolean
}

/** Une nature offerte au filtre et au formulaire — un terme de `highlight_nature`. */
export interface ShowcaseNatureOption {
  code: TaxonomyTermCode
  label: I18nText
  color: ColorHex | null
  icon: string | null
}

/** Une édition offerte au filtre et au formulaire, DANS LE PÉRIMÈTRE seulement. */
export interface ShowcaseEventOption {
  id: EventId
  title: I18nText
  acronym: string | null
  edition_year: number
  slug: Slug
}

/** L'écran `/admin/vitrine` en une réponse. */
export interface ShowcaseListScreen {
  /** Toutes les diapositives du périmètre, triées par `placement` puis
   *  `sort_order`. Les onglets d'emplacement filtrent côté écran. */
  rows: ShowcaseListRow[]
  /** Nombre de lignes par emplacement — les deux onglets l'annoncent. */
  counts: Record<HighlightPlacement, number>
  /** Nombre de lignes par état de diffusion, tous emplacements confondus. */
  broadcast_counts: Record<ShowcaseBroadcastState, number>
  natures: ShowcaseNatureOption[]
  events: ShowcaseEventOption[]
  /** Vrai quand la personne administre la plateforme entière. Faux : elle ne
   *  voit ni ne peut créer de contenu à `event_id` nul. */
  is_global_scope: boolean
}

// ===========================================================================
// 3. LE FORMULAIRE
// ===========================================================================

/**
 * Les valeurs saisissables, une par colonne éditable de `content.highlights` —
 * plus les thématiques, qui vivent dans `reference.entity_terms`.
 *
 * `id` nul : création. Les trois médias n'y sont PAS : ils passent par
 * `media.attachments`, et leur téléversement arrive en phase B — le formulaire
 * en montre les emplacements et les contraintes, lus de `media_rules`.
 */
export interface ShowcaseFormValues {
  id: HighlightId | null
  placement: HighlightPlacement
  status: HighlightStatus
  nature_code: TaxonomyTermCode
  sort_order: number

  /** Le français est OBLIGATOIRE — c'est la langue pivot de la base. */
  title: I18nText
  quote: I18nText | null
  body: I18nText | null

  /** Personne du répertoire. Renseignée, elle prime sur `author_name` à
   *  l'affichage : le formulaire doit le dire plutôt que de laisser croire que
   *  le nom libre sera retenu. */
  person_id: PersonId | null
  author_name: string | null
  author_title: I18nText | null

  /** EXCLUSIF de `organization_label` — règle métier n° 1. */
  organization_id: OrganizationId | null
  organization_label: string | null

  country_id: CountryId | null

  /** Vide (`null`) = contenu de plateforme : réservé à la portée globale. */
  event_id: EventId | null
  /** Renseignée, elle DÉRIVE `event_id` par trigger et refuse la contradiction. */
  session_id: SessionId | null

  link_url: Url | null
  /** Nul si `link_url` est nul (`ck_highlights_link_shape`). */
  link_label: I18nText | null

  background_color_hex: ColorHex | null

  starts_at: IsoDateTime | null
  ends_at: IsoDateTime | null

  /** Codes de `activity_theme`. Trois au plus s'affichent sur une carte. */
  theme_codes: TaxonomyTermCode[]
}

/** Une séance offerte au rattachement — celles de l'édition choisie. */
export interface ShowcaseSessionOption {
  id: SessionId
  event_id: EventId
  title: I18nText
  starts_at: IsoDateTime
  /** Le fuseau de la séance : l'option l'affiche, comme toute date du projet. */
  timezone: TimeZoneName
}

/** Une organisation du répertoire, offerte plutôt qu'un nom à retaper. */
export interface ShowcaseOrganizationOption {
  id: OrganizationId
  legal_name: string
  acronym: string | null
}

/** Une personne du répertoire, offerte plutôt qu'un nom à retaper. */
export interface ShowcasePersonOption {
  id: PersonId
  /** `identity.people.display_name`, colonne générée. */
  display_name: string
}

/** Un pays du référentiel. Le nom est multilingue, le code porte le drapeau. */
export interface ShowcaseCountryOption {
  id: CountryId
  iso2: string
  name: I18nText
}

/**
 * Un emplacement de média du formulaire : la contrainte lue de
 * `media.attachable_roles`, et ce qui y est déjà rattaché.
 */
export interface ShowcaseMediaSlot extends HighlightMediaRule {
  /** L'objet actuellement rattaché, ou `null`. */
  current: AttachedImage | null
  /**
   * Vrai quand un objet EXISTE pour ce rôle mais n'est pas encore servi —
   * en traitement, en quarantaine, en échec. `current` est alors nul, et
   * l'écran doit le dire au lieu d'afficher un emplacement vide : c'est la
   * différence entre « aucune vidéo » et « la vidéo arrive ».
   */
  is_pending: boolean
}

/** L'écran `/admin/vitrine/nouveau` et `/admin/vitrine/[id]`, en une réponse. */
export interface ShowcaseFormScreen {
  values: ShowcaseFormValues
  /**
   * L'APERÇU, dans le contrat EXACT du bandeau public.
   *
   * Le même composant rend l'aperçu et la vitrine : une seconde mise en page
   * divergerait au premier ajustement, et l'éditeur cesserait de croire ce
   * qu'il voit. C'est ce qui rend cet écran utilisable.
   */
  preview: ShowcaseRow
  natures: ShowcaseNatureOption[]
  events: ShowcaseEventOption[]
  /** Séances de l'édition choisie ; vide tant qu'aucune n'est choisie. */
  sessions: ShowcaseSessionOption[]
  organizations: ShowcaseOrganizationOption[]
  people: ShowcasePersonOption[]
  countries: ShowcaseCountryOption[]
  /** Les thématiques `activity_theme` offertes, avec libellé et couleur. */
  available_themes: ScheduleThemeBadge[]
  media: ShowcaseMediaSlot[]
  is_global_scope: boolean
}

// ===========================================================================
// 4. LES ÉCRITURES
// ===========================================================================

/** Le corps de `saveShowcase` — création si `id` est nul, mise à jour sinon. */
export type ShowcaseSavePayload = ShowcaseFormValues

/** Publier, retirer (retour en brouillon) ou archiver, depuis la liste. */
export interface ShowcaseStatusPayload {
  id: HighlightId
  status: HighlightStatus
}

/** Un cran vers le haut ou vers le bas, DANS SON EMPLACEMENT. */
export interface ShowcaseReorderPayload {
  id: HighlightId
  direction: 'up' | 'down'
}

/**
 * Le champ mis en cause par une erreur de validation. Le nom est celui de la
 * COLONNE : l'écran s'en sert pour poser le focus, et une clé inventée
 * obligerait à tenir une table de correspondance de plus.
 */
export type ShowcaseFormField =
  | 'nature_code'
  | 'title'
  | 'quote'
  | 'body'
  | 'author_name'
  | 'author_title'
  | 'organization_id'
  | 'organization_label'
  | 'event_id'
  | 'session_id'
  | 'link_url'
  | 'link_label'
  | 'background_color_hex'
  | 'starts_at'
  | 'ends_at'
  | 'sort_order'

/**
 * Le motif du refus. CODE et non message : la traduction appartient aux
 * fichiers i18n de l'écran, jamais à l'API ni aux données simulées.
 *
 * Les cinq premiers reprennent une contrainte de `115_content.sql` — leur
 * message doit donc être exploitable en français, et pas la traduction littérale
 * d'une erreur PostgreSQL.
 */
export type ShowcaseValidationCode =
  /** `ck_highlights_window` — la fin précède le début. */
  | 'window_inverted'
  /** `ck_highlights_organization_shape` — désignée ET nommée. */
  | 'organization_both'
  /** `ck_highlights_link_shape` — un libellé de lien sans lien. */
  | 'link_label_without_url'
  /** `tg_highlight_normalize` — la séance appartient à une autre édition. */
  | 'session_event_mismatch'
  /** `background_color_hex ~ '^#[0-9a-fA-F]{6}$'`. */
  | 'color_format'
  /** Le français est obligatoire dans un champ multilingue. */
  | 'french_required'
  /** Champ obligatoire non renseigné. */
  | 'required'
  /** `platform.url` — adresse absolue http(s) attendue. */
  | 'url_format'
  /** Contenu de plateforme (`event_id` nul) demandé sans portée globale. */
  | 'global_scope_required'

/** Une erreur de validation, prête à être posée sur son champ. */
export interface ShowcaseValidationError {
  field: ShowcaseFormField
  code: ShowcaseValidationCode
}

/**
 * Le retour de toute écriture de la vitrine.
 *
 * `ok: false` porte les erreurs et laisse `row` nul ; `ok: true` rend la LIGNE
 * telle que la liste doit désormais l'afficher — l'écran n'a rien à recomposer.
 *
 * `placement_rows` n'est renseigné que par les écritures qui touchent à
 * l'ORDRE — le réordonnancement, la création, le changement d'emplacement : un
 * déplacement change au moins deux lignes, et rendre la seule ligne déplacée
 * laisserait l'écran afficher deux fois le même rang. Nul ailleurs.
 */
export interface ShowcaseWriteResult {
  ok: boolean
  errors: ShowcaseValidationError[]
  row: ShowcaseListRow | null
  /** L'emplacement concerné, entièrement renuméroté et retrié. Nul quand
   *  l'écriture ne change aucun ordre. */
  placement_rows: ShowcaseListRow[] | null
}
