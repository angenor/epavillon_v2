/**
 * BACK-OFFICE DE LA VITRINE (A15) — les compositions et les quatre écritures.
 *
 * `content.ts` porte la DONNÉE, `home.ts` la composition de l'accueil public et
 * le journal d'écritures ; ce fichier porte ce que l'ADMINISTRATION en fait :
 * la liste filtrée par périmètre, le formulaire et son aperçu, l'ordre, la
 * publication et la duplication.
 *
 * LA DÉPENDANCE NE VA QUE DANS UN SENS : `admin-showcase.ts → home.ts →
 * content.ts`. Aucun cycle — l'accueil ne connaît aucune écriture, et le
 * back-office écrit dans le journal que l'accueil relit.
 *
 * ── CE QUE CE FICHIER REJOUE, ET DANS LE MÊME ORDRE QUE LA BASE ─────────────
 *
 *   ADR-14 / règle n° 8            le périmètre par `event_id`, `event_id` nul
 *                                  demandant la portée globale
 *   `ck_highlights_*`              les quatre contraintes, traduites en messages
 *                                  français exploitables AVANT l'écriture
 *   `tg_highlight_normalize`       la séance impose son édition ; `published_at`
 *                                  se pose une fois et ne se rejoue pas
 *   `content.v_showcase`           l'aperçu, dans le contrat EXACT du bandeau
 */

import type { AdministeredEvents } from '~/types/identity'
import type { Highlight, HighlightPlacement } from '~/types/content'
import type {
  ShowcaseBroadcastState,
  ShowcaseEventOption,
  ShowcaseFormScreen,
  ShowcaseFormValues,
  ShowcaseListRow,
  ShowcaseListScreen,
  ShowcaseMediaSlot,
  ShowcaseNatureOption,
  ShowcaseReorderPayload,
  ShowcaseSavePayload,
  ShowcaseStatusPayload,
  ShowcaseValidationError,
  ShowcaseWriteResult,
} from '~/types/admin-showcase'
import type { ScheduleThemeBadge, ShowcaseRow } from '~/types/views'
import type { EventId, I18nText, Uuid } from '~/types/shared'
import {
  highlightMediaRules,
  highlightNatureTerms,
  highlightThemeBadges,
  showcaseRowOf,
} from './content'
import { assetOf, attachedImage, attachmentOf } from './covers'
import {
  addedHighlights,
  countryById,
  editions,
  effectiveHighlights,
  nextHighlightId,
  patches,
  rawHighlight,
} from './home'
import { taxonomyTerms, countries } from './reference'
import { organizations } from './org'
import { people } from './people'
import { allSessions } from './sessions'

/**
 * LE PÉRIMÈTRE D'ADMINISTRATION — règle métier n° 8, ADR-14.
 *
 * Une diapositive rattachée à une édition n'appartient qu'à qui administre cette
 * édition. Une diapositive SANS édition parle de la plateforme entière : elle
 * demande la portée GLOBALE. C'est le seul endroit du fichier où ce filtre est
 * écrit — le dupliquer, c'est l'oublier quelque part.
 */
function inScope(highlight: Highlight, scope: AdministeredEvents): boolean {
  if (scope.is_global) return true
  if (highlight.event_id === null) return false
  return scope.event_ids.includes(highlight.event_id)
}

/**
 * L'état RÉEL de diffusion : statut et fenêtre combinés.
 *
 * `published` ne suffit pas à dire ce qui est à l'écran, et c'est la source de
 * l'incompréhension la plus fréquente au back-office — « je l'ai publiée, elle
 * ne s'affiche pas ». Elle est publiée, sa fenêtre est close.
 */
function broadcastState(highlight: Highlight, at: number): ShowcaseBroadcastState {
  if (highlight.status === 'draft') return 'draft'
  if (highlight.status === 'archived') return 'archived'
  if (highlight.starts_at !== null && Date.parse(highlight.starts_at) > at) return 'scheduled'
  if (highlight.ends_at !== null && Date.parse(highlight.ends_at) <= at) return 'expired'
  return 'live'
}

const organizationById = new Map(organizations.map((org) => [org.id, org]))
const personById = new Map(people.map((person) => [person.id, person]))
const eventById = new Map(editions.map((edition) => [edition.id, edition]))
const sessionById = new Map(allSessions.map((session) => [session.id, session]))
const natureByCode = new Map(highlightNatureTerms.map((term) => [term.code, term]))

/** Une ligne du tableau dense, attribution et rattachement résolus. */
function listRow(highlight: Highlight, at: number): ShowcaseListRow {
  const nature = natureByCode.get(highlight.nature_code)
  const person = highlight.person_id ? personById.get(highlight.person_id) : undefined
  const organization = highlight.organization_id
    ? organizationById.get(highlight.organization_id)
    : undefined
  const countryId = highlight.country_id ?? organization?.country_id ?? null
  const country = countryId ? countryById.get(countryId) : undefined
  const event = highlight.event_id ? eventById.get(highlight.event_id) : undefined
  const session = highlight.session_id ? sessionById.get(highlight.session_id) : undefined

  return {
    id: highlight.id,
    placement: highlight.placement,
    status: highlight.status,
    broadcast_state: broadcastState(highlight, at),
    sort_order: highlight.sort_order,

    nature_code: highlight.nature_code,
    nature_label: nature?.label ?? null,
    nature_color: nature?.color_hex ?? null,
    nature_icon: nature?.icon ?? null,

    title: highlight.title,

    author_name: person?.display_name ?? highlight.author_name,
    author_title: highlight.author_title,
    organization_name: organization?.legal_name ?? highlight.organization_label,
    organization_acronym: organization?.acronym ?? null,
    country_name: country?.name ?? null,

    event_id: highlight.event_id,
    event_title: event?.title ?? null,
    event_slug: event?.slug ?? null,
    session_id: highlight.session_id,
    session_title: session?.title ?? null,

    thumbnail: attachedImage('content', 'highlights', highlight.id, 'cover'),
    background_image: attachedImage('content', 'highlights', highlight.id, 'banner'),
    // `attachedImage()` n'en rend un que si l'objet est PRÊT : une vidéo encore
    // en traitement ne compte pas, et la colonne dit ce que le public voit.
    has_video: attachedImage('content', 'highlights', highlight.id, 'video') !== null,
    background_color_hex: highlight.background_color_hex,

    starts_at: highlight.starts_at,
    ends_at: highlight.ends_at,
    published_at: highlight.published_at,
    updated_at: highlight.updated_at,

    // Renseignés par `withRank()` : la place dans l'emplacement n'est connue
    // qu'une fois l'ensemble trié.
    is_first: false,
    is_last: false,
  }
}

/**
 * Pose `is_first` et `is_last` par emplacement, après tri.
 *
 * C'est ce qui désactive les boutons monter et descendre aux extrémités. Le
 * calculer dans l'écran obligerait chaque onglet à refaire le tri, et le premier
 * qui l'oublierait offrirait un bouton sans effet.
 */
function withRank(rows: ShowcaseListRow[]): ShowcaseListRow[] {
  const ordered = [...rows].sort(
    (a, b) => a.placement.localeCompare(b.placement) || a.sort_order - b.sort_order,
  )
  for (const placement of ['home_hero'] as HighlightPlacement[]) {
    const bucket = ordered.filter((row) => row.placement === placement)
    bucket.forEach((row, index) => {
      row.is_first = index === 0
      row.is_last = index === bucket.length - 1
    })
  }
  return ordered
}

function natureOptions(): ShowcaseNatureOption[] {
  return [...highlightNatureTerms]
    .filter((term) => term.is_active)
    .sort((a, b) => a.sort_order - b.sort_order)
    .map((term) => ({
      code: term.code,
      label: term.label,
      color: term.color_hex,
      icon: term.icon,
    }))
}

/** Les éditions OFFERTES au filtre et au formulaire — le périmètre, et rien d'autre. */
function eventOptions(scope: AdministeredEvents): ShowcaseEventOption[] {
  return editions
    .filter((edition) => scope.is_global || scope.event_ids.includes(edition.id))
    .map((edition) => ({
      id: edition.id,
      title: edition.title,
      acronym: edition.acronym,
      edition_year: edition.edition_year,
      slug: edition.slug,
    }))
    .sort((a, b) => b.edition_year - a.edition_year)
}

/**
 * L'écran `/admin/vitrine`, filtré par périmètre.
 *
 * Rend `null` quand la personne n'administre rien : l'écran affiche alors
 * `UiForbiddenState`, et non une liste vide — une liste vide laisserait croire
 * qu'il n'y a rien à voir, là où il n'y a rien de PERMIS.
 */
export function showcaseList(
  scope: AdministeredEvents,
  at: number = Date.now(),
): ShowcaseListScreen | null {
  if (!scope.is_global && scope.event_ids.length === 0) return null

  const rows = withRank(
    effectiveHighlights()
      .filter((highlight) => inScope(highlight, scope))
      .map((highlight) => listRow(highlight, at)),
  )

  const broadcastCounts: Record<ShowcaseBroadcastState, number> = {
    live: 0,
    scheduled: 0,
    expired: 0,
    draft: 0,
    archived: 0,
  }
  for (const row of rows) broadcastCounts[row.broadcast_state] += 1

  return {
    rows,
    counts: {
      home_hero: rows.filter((row) => row.placement === 'home_hero').length,
    },
    broadcast_counts: broadcastCounts,
    natures: natureOptions(),
    events: eventOptions(scope),
    is_global_scope: scope.is_global,
  }
}

/** Les valeurs du formulaire, telles que `[id].vue` les charge. */
function formValues(highlight: Highlight): ShowcaseFormValues {
  return {
    id: highlight.id,
    placement: highlight.placement,
    status: highlight.status,
    nature_code: highlight.nature_code,
    sort_order: highlight.sort_order,
    title: highlight.title,
    quote: highlight.quote,
    body: highlight.body,
    person_id: highlight.person_id,
    author_name: highlight.author_name,
    author_title: highlight.author_title,
    organization_id: highlight.organization_id,
    organization_label: highlight.organization_label,
    country_id: highlight.country_id,
    event_id: highlight.event_id,
    session_id: highlight.session_id,
    link_url: highlight.link_url,
    link_label: highlight.link_label,
    background_color_hex: highlight.background_color_hex,
    starts_at: highlight.starts_at,
    ends_at: highlight.ends_at,
    theme_codes: highlightThemeBadges(highlight.id).map((theme) => theme.code),
  }
}

/** Les valeurs d'une diapositive NEUVE — le formulaire de création. */
export function blankShowcase(
  placement: HighlightPlacement = 'home_hero',
  eventId: EventId | null = null,
): ShowcaseFormValues {
  return {
    id: null,
    placement,
    status: 'draft',
    nature_code: 'testimonial',
    sort_order: 0,
    title: { fr: '' },
    quote: null,
    body: null,
    person_id: null,
    author_name: null,
    author_title: null,
    organization_id: null,
    organization_label: null,
    country_id: null,
    event_id: eventId,
    session_id: null,
    link_url: null,
    link_label: null,
    background_color_hex: null,
    starts_at: null,
    ends_at: null,
    theme_codes: [],
  }
}

/**
 * Les trois emplacements de média, avec leur contrainte et leur état.
 *
 * `is_pending` distingue « aucune vidéo » de « la vidéo arrive » : un objet
 * rattaché mais non `ready` n'est pas servi, et un emplacement qui paraîtrait
 * vide ferait téléverser une seconde fois.
 */
function mediaSlots(highlightId: Uuid): ShowcaseMediaSlot[] {
  return highlightMediaRules.map((rule) => {
    const current = attachedImage('content', 'highlights', highlightId, rule.role)
    const asset = assetOf(attachmentOf('content', 'highlights', highlightId, rule.role))
    return { ...rule, current, is_pending: current === null && asset !== null }
  })
}

/** Les thématiques `activity_theme` offertes au formulaire. */
function availableThemes(): ScheduleThemeBadge[] {
  return taxonomyTerms
    .filter((term) => term.taxonomy_code === 'activity_theme' && term.is_active)
    .sort((a, b) => a.sort_order - b.sort_order)
    .map((term) => ({
      code: term.code,
      label: term.label,
      color: term.color_hex,
      icon: term.icon,
    }))
}

/**
 * L'écran de formulaire, création comme modification.
 *
 * `highlightId` nul : création. L'APERÇU est rendu dans le contrat exact du
 * bandeau public (`ShowcaseRow`) — c'est le même composant qui l'affiche, et
 * c'est ce qui rend l'écran utilisable.
 */
export function showcaseForm(
  highlightId: Uuid | null,
  scope: AdministeredEvents,
  options: { placement?: HighlightPlacement } = {},
): ShowcaseFormScreen | null {
  if (!scope.is_global && scope.event_ids.length === 0) return null

  let values: ShowcaseFormValues
  let preview: ShowcaseRow

  if (highlightId === null) {
    // Une administratrice détachée sur une seule édition ne peut pas créer de
    // contenu de plateforme : le formulaire s'ouvre donc sur SON édition.
    const defaultEvent = scope.is_global ? null : (scope.event_ids[0] ?? null)
    values = blankShowcase(options.placement ?? 'home_hero', defaultEvent)
    preview = showcaseRowOf(draftHighlight(values))
  } else {
    const highlight = rawHighlight(highlightId)
    if (!highlight || !inScope(highlight, scope)) return null
    values = formValues(highlight)
    preview = showcaseRowOf(highlight)
  }

  const eventForSessions = values.event_id
  return {
    values,
    preview,
    natures: natureOptions(),
    events: eventOptions(scope),
    sessions: eventForSessions
      ? allSessions
          .filter((session) => session.event_id === eventForSessions)
          .map((session) => ({
            id: session.id,
            event_id: session.event_id,
            title: session.title,
            starts_at: session.starts_at,
            timezone: session.timezone,
          }))
          .sort((a, b) => a.starts_at.localeCompare(b.starts_at))
      : [],
    organizations: organizations.map((org) => ({
      id: org.id,
      legal_name: org.legal_name,
      acronym: org.acronym,
    })),
    people: people.map((person) => ({ id: person.id, display_name: person.display_name })),
    countries: countries.map((country) => ({
      id: country.id,
      iso2: country.iso2,
      name: country.name,
    })),
    available_themes: availableThemes(),
    media: mediaSlots(values.id ?? ''),
    is_global_scope: scope.is_global,
  }
}

/**
 * Une diapositive par son identifiant, telle que le formulaire la lit.
 *
 * NE FILTRE PAS par périmètre : c'est `useApi()` qui refuse, parce que lui seul
 * dispose de `ForbiddenError` — et refuser vaut mieux que rendre `null`, qui se
 * confondrait avec « supprimée ». Le contrat attendu côté API :
 *
 *   const found = m.showcaseById(id)
 *   if (!found) return null
 *   if (found.event_id === null && !scope.is_global) throw new ForbiddenError()
 *   if (found.event_id !== null) assertEventInScope(found.event_id, scope)
 */
export function showcaseById(highlightId: Uuid): ShowcaseFormValues | null {
  const highlight = rawHighlight(highlightId)
  return highlight ? formValues(highlight) : null
}

// ---------------------------------------------------------------------------
// Les écritures
// ---------------------------------------------------------------------------

/** Une diapositive éphémère, composée d'un formulaire — pour l'aperçu et le tri. */
function draftHighlight(values: ShowcaseFormValues, existing?: Highlight): Highlight {
  const now = new Date().toISOString()
  return {
    id: values.id ?? existing?.id ?? nextHighlightId(),
    placement: values.placement,
    status: values.status,
    nature_code: values.nature_code,
    sort_order: values.sort_order,
    title: values.title,
    quote: values.quote,
    body: values.body,
    person_id: values.person_id,
    author_name: values.author_name,
    author_title: values.author_title,
    organization_id: values.organization_id,
    organization_label: values.organization_label,
    country_id: values.country_id,
    event_id: values.event_id,
    session_id: values.session_id,
    link_url: values.link_url,
    link_label: values.link_label,
    background_color_hex: values.background_color_hex,
    starts_at: values.starts_at,
    ends_at: values.ends_at,
    // `tg_highlight_normalize` : la date se pose au PREMIER passage en
    // `published` et ne se rejoue pas — republier après archivage ne réécrit
    // pas l'histoire.
    published_at:
      values.status === 'published' ? (existing?.published_at ?? now) : (existing?.published_at ?? null),
    created_by: existing?.created_by ?? null,
    created_at: existing?.created_at ?? now,
    updated_at: now,
  }
}

/** Le français est-il rempli ? Une locale absente est le cas normal, pas le fr. */
function hasFrench(text: I18nText | null): boolean {
  return text !== null && text.fr.trim().length > 0
}

/**
 * Les contraintes de `115_content.sql`, rejouées AVANT l'écriture.
 *
 * Le code ne réimplémente pas ce que la base porte : il le devance pour rendre
 * un message français exploitable là où PostgreSQL rendrait
 * « ck_highlights_organization_shape ». La base reste l'autorité — ces
 * vérifications ne la remplacent pas.
 */
function validate(
  values: ShowcaseFormValues,
  scope: AdministeredEvents,
): ShowcaseValidationError[] {
  const errors: ShowcaseValidationError[] = []

  if (!hasFrench(values.title)) errors.push({ field: 'title', code: 'french_required' })
  if (values.nature_code.trim().length === 0)
    errors.push({ field: 'nature_code', code: 'required' })

  // `ck_highlights_window`
  if (
    values.starts_at !== null &&
    values.ends_at !== null &&
    Date.parse(values.ends_at) <= Date.parse(values.starts_at)
  ) {
    errors.push({ field: 'ends_at', code: 'window_inverted' })
  }

  // `ck_highlights_organization_shape` — règle métier n° 1.
  if (values.organization_id !== null && values.organization_label !== null) {
    errors.push({ field: 'organization_label', code: 'organization_both' })
  }

  // `ck_highlights_link_shape`
  if (values.link_label !== null && values.link_url === null) {
    errors.push({ field: 'link_label', code: 'link_label_without_url' })
  }
  if (values.link_url !== null && !/^https?:\/\/\S+$/.test(values.link_url)) {
    errors.push({ field: 'link_url', code: 'url_format' })
  }

  if (
    values.background_color_hex !== null &&
    !/^#[0-9a-fA-F]{6}$/.test(values.background_color_hex)
  ) {
    errors.push({ field: 'background_color_hex', code: 'color_format' })
  }

  // `tg_highlight_normalize` : la séance impose son édition.
  if (values.session_id !== null) {
    const session = sessionById.get(values.session_id)
    if (session && values.event_id !== null && values.event_id !== session.event_id) {
      errors.push({ field: 'session_id', code: 'session_event_mismatch' })
    }
  }

  // ADR-14 : un contenu de plateforme demande la portée globale.
  if (values.event_id === null && !scope.is_global) {
    errors.push({ field: 'event_id', code: 'global_scope_required' })
  }
  if (values.event_id !== null && !scope.is_global && !scope.event_ids.includes(values.event_id)) {
    errors.push({ field: 'event_id', code: 'global_scope_required' })
  }

  return errors
}

function refused(errors: ShowcaseValidationError[]): ShowcaseWriteResult {
  return { ok: false, errors, row: null, placement_rows: null }
}

/** L'emplacement entier, renuméroté de 10 en 10 puis rendu à l'écran. */
function renumber(placement: HighlightPlacement, scope: AdministeredEvents, at: number): ShowcaseListRow[] {
  const bucket = effectiveHighlights()
    .filter((highlight) => highlight.placement === placement && inScope(highlight, scope))
    .sort((a, b) => a.sort_order - b.sort_order)

  bucket.forEach((highlight, index) => {
    const next = (index + 1) * 10
    if (highlight.sort_order !== next) {
      patches.set(highlight.id, { ...patches.get(highlight.id), sort_order: next })
    }
  })

  return withRank(
    effectiveHighlights()
      .filter((highlight) => highlight.placement === placement && inScope(highlight, scope))
      .map((highlight) => listRow(highlight, at)),
  )
}

/**
 * Créer ou modifier une diapositive.
 *
 * `payload.id` nul : création — la nouvelle ligne se place EN FIN de son
 * emplacement, ce que l'éditeur corrige ensuite avec les boutons d'ordre. La
 * placer en tête déplacerait silencieusement tout le reste.
 */
export function saveShowcase(
  payload: ShowcaseSavePayload,
  scope: AdministeredEvents,
  at: number = Date.now(),
): ShowcaseWriteResult {
  const errors = validate(payload, scope)
  if (errors.length > 0) return refused(errors)

  if (payload.id === null) {
    const last = effectiveHighlights()
      .filter((highlight) => highlight.placement === payload.placement)
      .reduce((max, highlight) => Math.max(max, highlight.sort_order), 0)
    const created = draftHighlight({
      ...payload,
      id: nextHighlightId(),
      sort_order: last + 10,
    })
    addedHighlights.push(created)
    return {
      ok: true,
      errors: [],
      row: listRow(created, at),
      placement_rows: renumber(created.placement, scope, at),
    }
  }

  const existing = rawHighlight(payload.id)
  if (!existing || !inScope(existing, scope)) return refused([{ field: 'event_id', code: 'global_scope_required' }])

  const updated = draftHighlight(payload, existing)
  patches.set(existing.id, updated)
  return {
    ok: true,
    errors: [],
    row: listRow(updated, at),
    // L'emplacement peut avoir changé : on renumérote celui d'arrivée.
    placement_rows: renumber(updated.placement, scope, at),
  }
}

/**
 * Publier, retirer (retour en brouillon) ou archiver, depuis la liste.
 *
 * `published_at` se pose au premier passage en `published` et ne se rejoue
 * jamais : c'est le trigger qui le dit, et le back-office ne peut pas le
 * contredire.
 */
export function setShowcaseStatus(
  payload: ShowcaseStatusPayload,
  scope: AdministeredEvents,
  at: number = Date.now(),
): ShowcaseWriteResult {
  const existing = rawHighlight(payload.id)
  if (!existing || !inScope(existing, scope))
    return refused([{ field: 'event_id', code: 'global_scope_required' }])

  const patch: Partial<Highlight> = {
    status: payload.status,
    published_at:
      payload.status === 'published'
        ? (existing.published_at ?? new Date(at).toISOString())
        : existing.published_at,
    updated_at: new Date(at).toISOString(),
  }
  patches.set(existing.id, { ...patches.get(existing.id), ...patch })

  return {
    ok: true,
    errors: [],
    row: listRow({ ...existing, ...patch }, at),
    placement_rows: null,
  }
}

/**
 * Monter ou descendre d'un cran, DANS SON EMPLACEMENT.
 *
 * L'ordre est la fonction principale de cet écran — son absence était le défaut
 * n° 6 de la v1. Deux boutons de 44 px, jamais un glisser-déposer seul : il doit
 * rester utilisable au clavier.
 */
export function moveShowcase(
  payload: ShowcaseReorderPayload,
  scope: AdministeredEvents,
  at: number = Date.now(),
): ShowcaseWriteResult {
  const existing = rawHighlight(payload.id)
  if (!existing || !inScope(existing, scope))
    return refused([{ field: 'event_id', code: 'global_scope_required' }])

  const bucket = effectiveHighlights()
    .filter(
      (highlight) => highlight.placement === existing.placement && inScope(highlight, scope),
    )
    .sort((a, b) => a.sort_order - b.sort_order)

  const index = bucket.findIndex((highlight) => highlight.id === existing.id)
  const target = payload.direction === 'up' ? index - 1 : index + 1
  const neighbour = bucket[target]
  // Aux extrémités, on ne refuse pas : on ne fait rien. Les boutons sont déjà
  // désactivés par `is_first` / `is_last` ; un refus ici serait un message
  // d'erreur pour une action que l'écran n'offrait pas.
  if (!neighbour) {
    return { ok: true, errors: [], row: listRow(existing, at), placement_rows: null }
  }

  const own = existing.sort_order
  patches.set(existing.id, { ...patches.get(existing.id), sort_order: neighbour.sort_order })
  patches.set(neighbour.id, { ...patches.get(neighbour.id), sort_order: own })

  const rows = renumber(existing.placement, scope, at)
  return {
    ok: true,
    errors: [],
    row: rows.find((row) => row.id === existing.id) ?? null,
    placement_rows: rows,
  }
}

/**
 * Dupliquer une diapositive.
 *
 * La copie part en BROUILLON, en fin d'emplacement, titre suffixé côté écran :
 * dupliquer un contenu publié et le voir sortir aussitôt serait une publication
 * que personne n'a demandée. C'est le geste qui remet en avant un témoignage de
 * la COP30 à la COP31.
 */
export function duplicateShowcase(
  highlightId: Uuid,
  scope: AdministeredEvents,
  at: number = Date.now(),
): ShowcaseWriteResult {
  const existing = rawHighlight(highlightId)
  if (!existing || !inScope(existing, scope))
    return refused([{ field: 'event_id', code: 'global_scope_required' }])

  const last = effectiveHighlights()
    .filter((highlight) => highlight.placement === existing.placement)
    .reduce((max, highlight) => Math.max(max, highlight.sort_order), 0)

  const copy: Highlight = {
    ...existing,
    id: nextHighlightId(),
    status: 'draft',
    sort_order: last + 10,
    published_at: null,
    created_at: new Date(at).toISOString(),
    updated_at: new Date(at).toISOString(),
  }
  addedHighlights.push(copy)

  return {
    ok: true,
    errors: [],
    row: listRow(copy, at),
    placement_rows: renumber(copy.placement, scope, at),
  }
}
