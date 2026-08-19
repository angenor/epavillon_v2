/**
 * LE FORMULAIRE DE LA VITRINE, SA PART CALCULABLE — fonctions PURES.
 *
 * Trois choses vivent ici plutôt que dans les composants, parce qu'elles se
 * raisonnent sans DOM et se relisent sans template :
 *
 *   1. LA VALIDATION, qui devance les contraintes de `115_content.sql`. Le code
 *      ne réimplémente pas ce que la base porte — il le DEVANCE, pour rendre un
 *      message français exploitable là où PostgreSQL rendrait
 *      « ck_highlights_organization_shape ». La base reste l'autorité : elle
 *      refusera de toute façon, et `ShowcaseWriteResult.errors` retombe sur les
 *      mêmes champs avec les mêmes codes.
 *
 *   2. L'APERÇU VIVANT. `ShowcaseFormScreen.preview` est l'état ENREGISTRÉ ;
 *      pendant la saisie, il faut recomposer un `ShowcaseRow` depuis les valeurs
 *      en cours et les référentiels de l'écran. Sans quoi l'aperçu ne bouge
 *      qu'après enregistrement, et il ne sert plus à rien.
 *
 *   3. L'ÉTAT DE DIFFUSION d'une saisie non enregistrée. Le formulaire doit
 *      pouvoir dire « publiée, mais sa fenêtre est passée : personne ne la
 *      verra » AVANT que l'éditeur ne l'enregistre pour s'en apercevoir.
 *
 * AUCUNE TRADUCTION ICI. La validation rend des CODES ; leur libellé français
 * vit dans `i18n/locales/fr/pages/admin.showcase.form.json`, et son jumeau
 * anglais à côté. Un message écrit dans ce fichier serait un texte d'interface
 * hors des fichiers i18n — exactement ce que le projet interdit.
 */

import type {
  ShowcaseCountryOption,
  ShowcaseEventOption,
  ShowcaseFormField,
  ShowcaseFormValues,
  ShowcaseMediaSlot,
  ShowcaseNatureOption,
  ShowcaseOrganizationOption,
  ShowcasePersonOption,
  ShowcaseSessionOption,
  ShowcaseValidationCode,
  ShowcaseValidationError,
  ShowcaseBroadcastState,
} from '~/types/admin-showcase'
import type { HighlightMediaRole } from '~/types/content'
import type { ScheduleThemeBadge, ShowcaseRow } from '~/types/views'
import type { I18nText } from '~/types/shared'

// ===========================================================================
// 1. Petites lectures partagées
// ===========================================================================

/**
 * Le français est-il RENSEIGNÉ ? Pas « la clé existe-t-elle » : une chaîne vide
 * est ce que produit un champ qu'on a ouvert puis vidé, et `platform.t()` la
 * servirait telle quelle au public.
 */
export function hasFrenchText(text: I18nText | null | undefined): boolean {
  return Boolean(text && text.fr.trim().length > 0)
}

/** Un texte multilingue vidé de toute langue ne vaut pas mieux que `null`. */
export function emptyToNull(text: I18nText | null | undefined): I18nText | null {
  if (!text) return null
  const hasAny = Object.values(text).some((value) => (value ?? '').trim().length > 0)
  return hasAny ? text : null
}

/** Une chaîne libre vidée par l'éditeur redevient `null`, jamais `''`. */
export function trimmedOrNull(value: string | null | undefined): string | null {
  const next = (value ?? '').trim()
  return next.length > 0 ? next : null
}

/** Le poids maximal d'un rôle média, en Mio — ce qui s'affiche à l'éditeur. */
export function mebibytes(bytes: number): number {
  return Math.round(bytes / (1024 * 1024))
}

/** Une icône par rôle de média, pour que les trois emplacements se distinguent. */
export const SHOWCASE_MEDIA_ICON: Record<HighlightMediaRole, string> = {
  banner: 'monitor',
  video: 'video',
  cover: 'grid',
}

// ===========================================================================
// 2. La validation
// ===========================================================================

/**
 * Ce que l'écran connaît et que les valeurs ne portent pas : la portée du
 * compte, et les séances offertes — dont on tire l'édition d'une séance choisie.
 */
export interface ShowcaseValidationContext {
  isGlobalScope: boolean
  sessions: ShowcaseSessionOption[]
}

/** Adresse absolue http(s) — le domaine `platform.url` de la base. */
const URL_PATTERN = /^https?:\/\/\S+$/
/** `#RRGGBB`, six chiffres hexadécimaux — la contrainte de colonne, telle quelle. */
const COLOR_PATTERN = /^#[0-9a-fA-F]{6}$/

/**
 * Les contraintes de `115_content.sql`, rejouées AVANT l'écriture.
 *
 * L'ordre des retours n'est pas indifférent : le formulaire pose le focus sur le
 * PREMIER champ en défaut, et l'ordre suit celui des sections à l'écran — nature,
 * textes, attribution, rattachement, diffusion. Trier par gravité ferait sauter
 * l'éditeur du bas du formulaire vers le haut sans qu'il comprenne pourquoi.
 */
export function validateShowcaseForm(
  values: ShowcaseFormValues,
  context: ShowcaseValidationContext,
): ShowcaseValidationError[] {
  const errors: ShowcaseValidationError[] = []

  // — Nature et textes
  if (values.nature_code.trim().length === 0) {
    errors.push({ field: 'nature_code', code: 'required' })
  }
  // `title` est NOT NULL en base, et le français est la langue pivot.
  if (!hasFrenchText(values.title)) {
    errors.push({ field: 'title', code: 'french_required' })
  }
  // Une citation ou un corps saisis en anglais seulement laisseraient le public
  // francophone devant un bloc vide : `platform.t()` se replie sur le français,
  // qui n'existerait pas.
  if (values.quote !== null && !hasFrenchText(values.quote)) {
    errors.push({ field: 'quote', code: 'french_required' })
  }
  if (values.body !== null && !hasFrenchText(values.body)) {
    errors.push({ field: 'body', code: 'french_required' })
  }
  if (values.author_title !== null && !hasFrenchText(values.author_title)) {
    errors.push({ field: 'author_title', code: 'french_required' })
  }

  // — `ck_highlights_organization_shape` : RÈGLE MÉTIER N° 1.
  //   Retaper « IFDD » à côté d'une fiche existante recrée le doublon que la v2
  //   corrige ; la base l'interdit, le formulaire le dit en clair.
  if (values.organization_id !== null && trimmedOrNull(values.organization_label) !== null) {
    errors.push({ field: 'organization_label', code: 'organization_both' })
  }

  // — `tg_highlights_normalize` : la séance impose SON édition.
  if (values.session_id !== null && values.event_id !== null) {
    const session = context.sessions.find((entry) => entry.id === values.session_id)
    if (session && session.event_id !== values.event_id) {
      errors.push({ field: 'session_id', code: 'session_event_mismatch' })
    }
  }

  // — ADR-14 : un contenu sans édition parle au nom de la plateforme entière.
  if (values.event_id === null && !context.isGlobalScope) {
    errors.push({ field: 'event_id', code: 'global_scope_required' })
  }

  // — `ck_highlights_link_shape` : un libellé de lien sans lien ne mène nulle part.
  if (values.link_url !== null && !URL_PATTERN.test(values.link_url)) {
    errors.push({ field: 'link_url', code: 'url_format' })
  }
  if (emptyToNull(values.link_label) !== null && trimmedOrNull(values.link_url) === null) {
    errors.push({ field: 'link_label', code: 'link_label_without_url' })
  }
  if (values.link_label !== null && !hasFrenchText(values.link_label)) {
    errors.push({ field: 'link_label', code: 'french_required' })
  }

  // — La couleur de repli est une DONNÉE saisie, contrainte par la colonne.
  if (values.background_color_hex !== null && !COLOR_PATTERN.test(values.background_color_hex)) {
    errors.push({ field: 'background_color_hex', code: 'color_format' })
  }

  // — `ck_highlights_window` : la fin ne précède pas le début.
  if (
    values.starts_at !== null &&
    values.ends_at !== null &&
    Date.parse(values.ends_at) <= Date.parse(values.starts_at)
  ) {
    errors.push({ field: 'ends_at', code: 'window_inverted' })
  }

  return errors
}

/** Le code d'erreur porté par ce champ, s'il y en a un. */
export function showcaseErrorOf(
  errors: ShowcaseValidationError[],
  field: ShowcaseFormField,
): ShowcaseValidationCode | undefined {
  return errors.find((entry) => entry.field === field)?.code
}

// ===========================================================================
// 3. L'état de diffusion d'une saisie
// ===========================================================================

/**
 * Ce que la vitrine ferait de ces valeurs SI on les enregistrait maintenant.
 *
 * Même recomposition que `ShowcaseBroadcastState` côté liste : le statut ne
 * suffit pas à dire ce qui est à l'écran, la fenêtre s'y ajoute. C'est ce qui
 * évite qu'un éditeur publie, ne voie rien sortir, et cherche la panne pendant
 * une heure alors que sa fenêtre se terminait la veille.
 */
export function showcaseBroadcastStateOf(
  values: Pick<ShowcaseFormValues, 'status' | 'starts_at' | 'ends_at'>,
  at: number,
): ShowcaseBroadcastState {
  if (values.status === 'draft') return 'draft'
  if (values.status === 'archived') return 'archived'
  if (values.starts_at !== null && Date.parse(values.starts_at) > at) return 'scheduled'
  if (values.ends_at !== null && Date.parse(values.ends_at) <= at) return 'expired'
  return 'live'
}

// ===========================================================================
// 4. L'aperçu vivant
// ===========================================================================

/** Les référentiels de l'écran de formulaire, tels que `form()` les rend. */
export interface ShowcasePreviewRefs {
  natures: ShowcaseNatureOption[]
  events: ShowcaseEventOption[]
  sessions: ShowcaseSessionOption[]
  organizations: ShowcaseOrganizationOption[]
  people: ShowcasePersonOption[]
  countries: ShowcaseCountryOption[]
  themes: ScheduleThemeBadge[]
  media: ShowcaseMediaSlot[]
}

/** Le média rattaché à un rôle, ou `null` — un objet en traitement n'est pas servi. */
function mediaOf(media: ShowcaseMediaSlot[], role: HighlightMediaRole) {
  return media.find((slot) => slot.role === role)?.current ?? null
}

/**
 * LES VALEURS EN COURS DE SAISIE, DANS LE CONTRAT DU BANDEAU PUBLIC.
 *
 * `base` est l'aperçu rendu par l'API — il porte ce que le formulaire ne saisit
 * pas (les identifiants résolus au moment du chargement, la date de publication).
 * Tout ce qui se saisit est recomposé par-dessus, et tout ce qui se RÉSOUT
 * (nom d'organisation, titre d'édition, libellé de nature) est relu dans les
 * référentiels : la vue publique fait exactement ces jointures, et l'aperçu
 * mentirait s'il gardait la valeur d'avant le changement.
 *
 * Deux résolutions reprennent le `COALESCE` de `content.v_showcase`, et c'est
 * important : la personne du répertoire PRIME sur le nom libre, l'organisation
 * désignée prime sur le libellé libre. L'éditeur qui remplit les deux doit voir
 * lequel sortira — sans quoi il croira que son texte libre s'affiche.
 */
export function showcasePreviewOf(
  values: ShowcaseFormValues,
  base: ShowcaseRow,
  refs: ShowcasePreviewRefs,
): ShowcaseRow {
  const nature = refs.natures.find((entry) => entry.code === values.nature_code) ?? null
  const person = refs.people.find((entry) => entry.id === values.person_id) ?? null
  const organization =
    refs.organizations.find((entry) => entry.id === values.organization_id) ?? null
  const country = refs.countries.find((entry) => entry.id === values.country_id) ?? null
  const event = refs.events.find((entry) => entry.id === values.event_id) ?? null
  const session = refs.sessions.find((entry) => entry.id === values.session_id) ?? null

  return {
    ...base,
    placement: values.placement,
    sort_order: values.sort_order,

    nature_code: values.nature_code,
    nature_label: nature?.label ?? null,
    nature_color: nature?.color ?? null,
    nature_icon: nature?.icon ?? null,

    title: values.title,
    quote: emptyToNull(values.quote),
    body: emptyToNull(values.body),

    // `COALESCE(people.display_name, highlights.author_name)`.
    author_name: person?.display_name ?? trimmedOrNull(values.author_name),
    author_title: emptyToNull(values.author_title),
    person_id: values.person_id,

    organization_id: values.organization_id,
    // `COALESCE(organizations.legal_name, highlights.organization_label)`.
    organization_name: organization?.legal_name ?? trimmedOrNull(values.organization_label),
    organization_acronym: organization?.acronym ?? null,

    country_code: country?.iso2 ?? null,
    country_name: country?.name ?? null,

    event_id: values.event_id,
    event_slug: event?.slug ?? null,
    event_title: event?.title ?? null,

    session_id: values.session_id,
    // Le créneau et son fuseau ne se recomposent que pour la séance CHOISIE ;
    // une séance changée en cours de saisie laisserait sinon les heures de la
    // précédente, ce qui est le pire des deux mondes.
    session_slug: session ? base.session_slug : null,
    session_title: session?.title ?? null,
    session_starts_at: session?.starts_at ?? null,
    session_ends_at: session && session.id === base.session_id ? base.session_ends_at : null,
    session_timezone: session?.timezone ?? null,

    link_url: trimmedOrNull(values.link_url),
    link_label: emptyToNull(values.link_label),

    background_image: mediaOf(refs.media, 'banner'),
    background_video: mediaOf(refs.media, 'video'),
    thumbnail: mediaOf(refs.media, 'cover'),
    background_color_hex: values.background_color_hex,

    theme_codes: values.theme_codes,
    themes: refs.themes.filter((theme) => values.theme_codes.includes(theme.code)),

    starts_at: values.starts_at,
    ends_at: values.ends_at,
  }
}
