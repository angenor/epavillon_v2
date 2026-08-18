/**
 * GESTION DES ÉVÉNEMENTS (A10) — les ÉCRITURES, et les contraintes qui refusent.
 *
 * ── CE QUE CE FICHIER REJOUE DE LA BASE ─────────────────────────────────────
 *
 * LES CONTRAINTES DE `060_events.sql`, ET AVEC LEUR NOM. `ck_events_period`,
 * `ck_events_physical_location`, `ux_events_slug`, `ux_events_series_edition`,
 * `ck_calls_window`, `ck_calls_extension`, `ck_calls_speakers`,
 * `ck_calls_duration_bounds`, `ck_calls_daily_window`, `ux_calls_one_per_event`,
 * `ux_broadcast_channels_default`, `ux_rooms_code`, `ux_review_criteria` : chacune
 * refuse ici comme elle refusera en base. Un écran qui n'aurait rencontré aucun
 * refus pendant tout son développement n'aurait aucun message à afficher le jour
 * du raccordement.
 *
 * À NE PAS CONFONDRE AVEC UN CHEVAUCHEMENT DE CRÉNEAUX, que le planificateur (A9)
 * n'a le droit de refuser dans aucun cas (règle métier n° 2). Ce sont ici des
 * invariants de DONNÉES, pas des arbitrages : un slug en double n'est pas une
 * décision d'équipe.
 *
 * ── LES CONSÉQUENCES `ON DELETE SET NULL`, LE VRAI PIÈGE DE CET ÉCRAN ───────
 *
 * Retirer une salle, supprimer un jour, désactiver un canal ne casse rien : les
 * séances concernées perdent leur rattachement (`xmod_fk_sessions_room`,
 * `xmod_fk_sessions_event_day`, `xmod_fk_sessions_broadcast_channel`, tous
 * `ON DELETE SET NULL`). C'est silencieux en base ; ce ne doit pas l'être à
 * l'écran, d'où le `sessions_detached` que chaque écriture rend. Supprimer un fil,
 * en revanche, CASCADE sur `programme.session_tracks` : la composition disparaît
 * avec lui.
 *
 * ── CE QU'IL NE FAIT PAS ────────────────────────────────────────────────────
 *
 * IL NE COMPOSE AUCUNE JOURNÉE SPÉCIALE. Les fils se créent ici, se colorent ici,
 * s'ouvrent au public ici ; le rattachement des séances appartient au
 * planificateur (A9), et aucune fonction de ce fichier n'écrit dans
 * `programme.session_tracks` — sauf la suppression d'un fil, qui en efface les
 * lignes parce que la base le ferait.
 *
 * IL NE GÉNÈRE PAS LES JOURS TOUT SEUL. `event.event_days` n'a aucun trigger de
 * dérivation : la génération depuis les dates de l'édition est un comportement
 * d'APPLICATION, pas du modèle. Elle est donc explicite — un bouton, un plan
 * annoncé avant d'agir — et non un effet de bord de la sauvegarde de l'édition.
 * Écart consigné dans `docs/PROGRESSION.md`.
 *
 * ── LES ÉCRITURES MUTENT LES TABLEAUX EN MÉMOIRE ────────────────────────────
 *
 * Comme les autres mocks d'écriture du projet, les changements vivent le temps de
 * la session du navigateur. Ils modifient les tableaux partagés de `core.ts`, si
 * bien qu'une salle ajoutée ici apparaît aussitôt dans le planificateur — ce qui
 * est exactement le comportement attendu.
 */

import type {
  CallFormError,
  CallSaveResult,
  CommitteePayload,
  CommitteeSaveResult,
  EditionCallPayload,
  EditionChannelPayload,
  EditionCriterion,
  EditionDayPayload,
  EditionFormError,
  EditionFormPayload,
  EditionRoomPayload,
  EditionSaveResult,
  EditionTabResult,
  EditionTrackPayload,
  EditionVenuePayload,
} from '~/types/admin-events'
import type { EventEdition, ProgrammeTrack } from '~/types/event/edition'
import type { BroadcastChannel, Room, Venue } from '~/types/event/venue'
import type { CallForProposals } from '~/types/event/call'
import type { Uuid } from '~/types/shared'
import { allSessions, sessionTracks } from '../sessions'
import { seedDefaultCriteria } from '../criteria'
import {
  calls,
  channelRows,
  criteria,
  datesBetween,
  days,
  editions,
  listRow,
  newId,
  periodOf,
  reviewers,
  roomRows,
  tracks,
  venueRows,
} from './core'
import {
  committeeOfCall,
  criterionRows,
  dayGenerationPlan,
  editionCall,
  editionDetail,
} from './detail'

// ---------------------------------------------------------------------------
// 1. L'ÉDITION
// ---------------------------------------------------------------------------

/**
 * Les refus de `event.events`, dans l'ordre où ils se lisent à l'écran.
 *
 * On rend TOUS les manquements d'un coup et non le premier : un formulaire de
 * dix-sept champs qui refuse un motif à la fois se corrige en dix-sept
 * allers-retours.
 */
function validateEdition(payload: EditionFormPayload): EditionFormError[] {
  const errors: EditionFormError[] = []

  if (!payload.title.fr?.trim()) errors.push({ code: 'required', field: 'title' })
  if (!payload.description.fr?.trim()) errors.push({ code: 'required', field: 'description' })
  if (!payload.slug.trim()) errors.push({ code: 'required', field: 'slug' })
  if (!payload.timezone.trim()) errors.push({ code: 'required', field: 'timezone' })

  if (payload.edition_year < 2000 || payload.edition_year > 2100) {
    errors.push({ code: 'year_range', field: 'edition_year' })
  }

  // `ck_events_period`
  if (Date.parse(payload.ends_at) <= Date.parse(payload.starts_at)) {
    errors.push({ code: 'period', field: 'ends_at' })
  }

  // `ck_events_physical_location` — hors ligne, le pays ET la ville sont exigés.
  // C'est la contrainte qui dit qu'un rendez-vous physique a un lieu : sans elle,
  // la page publique afficherait « en présentiel » sans dire où.
  if (payload.participation_mode !== 'online' && (!payload.country_id || !payload.city?.trim())) {
    errors.push({ code: 'physical_location', field: !payload.country_id ? 'country_id' : 'city' })
  }

  // `ck_events_coordinates` — un point se donne EN ENTIER ou pas du tout. Une
  // latitude seule ne désigne rien, et laisserait une carte s'ouvrir sur le
  // méridien de Greenwich.
  if ((payload.latitude === null) !== (payload.longitude === null)) {
    errors.push({
      code: 'coordinates',
      field: payload.latitude === null ? 'latitude' : 'longitude',
    })
  }
  // Les bornes du `CHECK` de chaque colonne, dites au bon champ.
  if (payload.latitude !== null && (payload.latitude < -90 || payload.latitude > 90)) {
    errors.push({ code: 'coordinates', field: 'latitude' })
  }
  if (payload.longitude !== null && (payload.longitude < -180 || payload.longitude > 180)) {
    errors.push({ code: 'coordinates', field: 'longitude' })
  }

  // `ux_events_slug` — unique sur TOUTE la plateforme, pas par série.
  if (editions.some((e) => e.slug === payload.slug && e.id !== payload.id)) {
    errors.push({ code: 'slug_taken', field: 'slug' })
  }

  // `ux_events_series_edition` — (série, année, libellé).
  const clash = editions.some(
    (e) =>
      e.id !== payload.id &&
      e.series_id === payload.series_id &&
      e.edition_year === payload.edition_year &&
      (e.edition_label ?? null) === (payload.edition_label ?? null),
  )
  if (clash) errors.push({ code: 'edition_taken', field: 'edition_label' })

  return errors
}

/**
 * ENREGISTRER UNE ÉDITION — création ou modification.
 *
 * LES JOURS SUIVENT LES DATES, MAIS PAS DE FORCE. À la CRÉATION, le calendrier
 * est généré d'office : une édition sans aucun jour n'est utilisable par aucun
 * autre écran. À la MODIFICATION, les jours qui manquent sont ajoutés et ceux qui
 * sortent de la période sont SIGNALÉS, jamais supprimés — un jour supprimé
 * détacherait les séances qu'il porte, et cette décision appartient à l'équipe,
 * dans l'onglet du calendrier.
 */
export function saveEdition(payload: EditionFormPayload, actorId: Uuid | null): EditionSaveResult {
  const errors = validateEdition(payload)
  if (errors.length > 0) {
    return { ok: false, edition: null, errors, days_created: 0, days_removed: 0, sessions_detached: 0 }
  }

  const now = new Date().toISOString()
  const isCreation = payload.id === null
  const existing = isCreation ? undefined : editions.find((e) => e.id === payload.id)

  if (!isCreation && !existing) {
    return {
      ok: false,
      edition: null,
      errors: [{ code: 'required', field: 'id' }],
      days_created: 0,
      days_removed: 0,
      sessions_detached: 0,
    }
  }

  const edition: EventEdition = existing ?? {
    id: newId('7021'),
    series_id: null,
    edition_label: null,
    edition_year: payload.edition_year,
    title: payload.title,
    acronym: null,
    slug: payload.slug,
    description: payload.description,
    status: 'draft',
    participation_mode: payload.participation_mode,
    timezone: payload.timezone,
    starts_at: payload.starts_at,
    ends_at: payload.ends_at,
    country_id: null,
    city: null,
    address: null,
    latitude: null,
    longitude: null,
    has_pavilion: false,
    programme_published_at: null,
    highlights: null,
    created_by: actorId,
    created_at: now,
    updated_at: now,
  }

  Object.assign(edition, {
    series_id: payload.series_id,
    edition_label: payload.edition_label,
    edition_year: payload.edition_year,
    title: payload.title,
    acronym: payload.acronym,
    slug: payload.slug,
    description: payload.description,
    status: payload.status,
    participation_mode: payload.participation_mode,
    timezone: payload.timezone,
    starts_at: payload.starts_at,
    ends_at: payload.ends_at,
    country_id: payload.country_id,
    city: payload.city,
    address: payload.address,
    latitude: payload.latitude,
    longitude: payload.longitude,
    has_pavilion: payload.has_pavilion,
    highlights: payload.highlights,
    updated_at: now,
  })

  if (isCreation) editions.push(edition)

  const plan = dayGenerationPlan(edition)
  const created = isCreation ? createDays(edition, plan.to_create) : createDays(edition, plan.to_create)

  return {
    ok: true,
    edition: listRow(edition),
    errors: [],
    days_created: created,
    // Rien n'est supprimé par une sauvegarde : les jours devenus hors période
    // sont rendus au calendrier, qui les marque et laisse décider.
    days_removed: 0,
    sessions_detached: 0,
  }
}

// ---------------------------------------------------------------------------
// 2. LES JOURNÉES DU CALENDRIER
// ---------------------------------------------------------------------------

function createDays(edition: EventEdition, dates: string[]): number {
  if (dates.length === 0) return 0
  const now = new Date().toISOString()
  const period = periodOf(edition)
  const ordered = datesBetween(period.first_day, period.last_day)

  for (const date of dates) {
    days.push({
      id: newId('7022'),
      event_id: edition.id,
      day_date: date,
      // Aucun contenu éditorial : un jour généré porte sa date et rien d'autre.
      // Inventer « Jour 3 » ferait un titre que personne n'a écrit et qui
      // s'afficherait tel quel sur la page publique.
      title: null,
      slug: null,
      description: null,
      is_featured: false,
      color_hex: null,
      sort_order: (ordered.indexOf(date) + 1) * 10,
      created_at: now,
      updated_at: now,
    })
  }
  return dates.length
}

/**
 * GÉNÉRER LE CALENDRIER, ET RETIRER OU NON LES JOURS HORS PÉRIODE.
 *
 * `removeOutsidePeriod` est un choix de l'équipe, pas un défaut : une édition
 * garde parfois une soirée d'ouverture la veille de son premier jour officiel.
 * Retirer un jour détache les séances qu'il portait — elles ne sont pas
 * supprimées, et l'écran annonce combien.
 */
export function generateEventDays(eventId: Uuid, removeOutsidePeriod: boolean): EditionTabResult {
  const edition = editions.find((e) => e.id === eventId)
  if (!edition) return { ok: false, detail: null, sessions_detached: 0, error_code: 'not_found' }

  const plan = dayGenerationPlan(edition)
  createDays(edition, plan.to_create)

  let detached = 0
  if (removeOutsidePeriod) {
    for (const stale of plan.to_review) {
      const index = days.findIndex((d) => d.id === stale.id)
      if (index === -1) continue
      days.splice(index, 1)
      // `xmod_fk_sessions_event_day ON DELETE SET NULL` : la séance survit.
      for (const session of allSessions) {
        if (session.event_day_id === stale.id) {
          ;(session as { event_day_id: Uuid | null }).event_day_id = null
          detached += 1
        }
      }
    }
  }

  return { ok: true, detail: editionDetail(eventId), sessions_detached: detached, error_code: null }
}

/** Le contenu ÉDITORIAL d'un jour. La date, elle, vient de la période. */
export function saveEventDay(eventId: Uuid, payload: EditionDayPayload): EditionTabResult {
  const day = days.find((d) => d.id === payload.id && d.event_id === eventId)
  if (!day) return { ok: false, detail: null, sessions_detached: 0, error_code: 'not_found' }

  // `ux_event_days_slug` — unique par édition, et un slug nul n'entre pas en jeu.
  if (
    payload.slug &&
    days.some((d) => d.event_id === eventId && d.id !== day.id && d.slug === payload.slug)
  ) {
    return { ok: false, detail: null, sessions_detached: 0, error_code: 'slug_taken' }
  }

  Object.assign(day, {
    title: payload.title,
    slug: payload.slug,
    description: payload.description,
    is_featured: payload.is_featured,
    color_hex: payload.color_hex,
    updated_at: new Date().toISOString(),
  })

  return { ok: true, detail: editionDetail(eventId), sessions_detached: 0, error_code: null }
}

// ---------------------------------------------------------------------------
// 3. LES JOURNÉES SPÉCIALES
// ---------------------------------------------------------------------------

/**
 * CRÉER OU MODIFIER UN FIL THÉMATIQUE.
 *
 * `is_published` est un BOOLÉEN À L'ÉCRAN et une DATE en base (`published_at`) :
 * ouvrir la page publique la date, la refermer la remet à nul. Le formulaire n'a
 * pas à saisir un horodatage pour dire « oui ».
 */
export function saveTrack(payload: EditionTrackPayload): EditionTabResult {
  if (!payload.title.fr?.trim() || !payload.code.trim() || !payload.slug.trim()) {
    return { ok: false, detail: null, sessions_detached: 0, error_code: 'required' }
  }

  // `ck_programme_tracks_period` — la portée indicative reste cohérente.
  if (payload.starts_on && payload.ends_on && payload.ends_on < payload.starts_on) {
    return { ok: false, detail: null, sessions_detached: 0, error_code: 'period' }
  }

  const siblings = tracks.filter((t) => t.event_id === payload.event_id && t.id !== payload.id)
  // `ux_programme_tracks_code` et `ux_programme_tracks_slug`, par édition.
  if (siblings.some((t) => t.code === payload.code)) {
    return { ok: false, detail: null, sessions_detached: 0, error_code: 'code_taken' }
  }
  if (siblings.some((t) => t.slug === payload.slug)) {
    return { ok: false, detail: null, sessions_detached: 0, error_code: 'slug_taken' }
  }

  const now = new Date().toISOString()
  const existing = payload.id ? tracks.find((t) => t.id === payload.id) : undefined

  const track: ProgrammeTrack =
    existing ??
    {
      id: newId('7026'),
      event_id: payload.event_id,
      code: payload.code,
      slug: payload.slug,
      kind: payload.kind,
      title: payload.title,
      subtitle: null,
      description: null,
      starts_on: null,
      ends_on: null,
      color_hex: null,
      curated_by: null,
      published_at: null,
      sort_order: payload.sort_order,
      created_at: now,
      updated_at: now,
    }

  Object.assign(track, {
    code: payload.code,
    slug: payload.slug,
    kind: payload.kind,
    title: payload.title,
    subtitle: payload.subtitle,
    description: payload.description,
    starts_on: payload.starts_on,
    ends_on: payload.ends_on,
    color_hex: payload.color_hex,
    curated_by: payload.curated_by,
    // La date de première ouverture est conservée : refermer puis rouvrir ne
    // réécrit pas l'histoire de la page.
    published_at: payload.is_published ? (track.published_at ?? now) : null,
    sort_order: payload.sort_order,
    updated_at: now,
  })

  if (!existing) tracks.push(track)

  return { ok: true, detail: editionDetail(payload.event_id), sessions_detached: 0, error_code: null }
}

/**
 * SUPPRIMER UN FIL — la seule suppression de cet écran qui CASCADE.
 *
 * `xmod_fk_session_tracks_track ON DELETE CASCADE` : la composition du fil
 * disparaît avec lui. Les séances restent programmées, elles cessent seulement
 * d'appartenir au fil. C'est un travail éditorial perdu, et l'écran le chiffre
 * avant de confirmer.
 */
export function removeTrack(eventId: Uuid, trackId: Uuid): EditionTabResult {
  const index = tracks.findIndex((t) => t.id === trackId && t.event_id === eventId)
  if (index === -1) return { ok: false, detail: null, sessions_detached: 0, error_code: 'not_found' }

  tracks.splice(index, 1)
  let detached = 0
  for (let i = sessionTracks.length - 1; i >= 0; i -= 1) {
    if (sessionTracks[i]!.track_id === trackId) {
      ;(sessionTracks as unknown[]).splice(i, 1)
      detached += 1
    }
  }

  return { ok: true, detail: editionDetail(eventId), sessions_detached: detached, error_code: null }
}

// ---------------------------------------------------------------------------
// 4. LES LIEUX ET LES SALLES
// ---------------------------------------------------------------------------

export function saveVenue(payload: EditionVenuePayload): EditionTabResult {
  if (!payload.name.fr?.trim()) {
    return { ok: false, detail: null, sessions_detached: 0, error_code: 'required' }
  }

  const existing = payload.id ? venueRows.find((v) => v.id === payload.id) : undefined
  const venue: Venue =
    existing ??
    {
      id: newId('7023'),
      event_id: payload.event_id,
      name: payload.name,
      kind: payload.kind,
      address: null,
      map_url: null,
      created_at: new Date().toISOString(),
    }

  Object.assign(venue, {
    name: payload.name,
    kind: payload.kind,
    address: payload.address,
    map_url: payload.map_url,
  })
  if (!existing) venueRows.push(venue)

  return { ok: true, detail: editionDetail(payload.event_id), sessions_detached: 0, error_code: null }
}

/** Retirer un lieu retire ses salles : `event.rooms.venue_id ON DELETE CASCADE`. */
export function removeVenue(eventId: Uuid, venueId: Uuid): EditionTabResult {
  const index = venueRows.findIndex((v) => v.id === venueId && v.event_id === eventId)
  if (index === -1) return { ok: false, detail: null, sessions_detached: 0, error_code: 'not_found' }

  const doomed = roomRows.filter((r) => r.venue_id === venueId).map((r) => r.id)
  venueRows.splice(index, 1)
  for (let i = roomRows.length - 1; i >= 0; i -= 1) {
    if (roomRows[i]!.venue_id === venueId) roomRows.splice(i, 1)
  }

  const detached = detachSessionsFromRooms(doomed)
  return { ok: true, detail: editionDetail(eventId), sessions_detached: detached, error_code: null }
}

function detachSessionsFromRooms(roomIds: Uuid[]): number {
  let detached = 0
  const targets = new Set(roomIds)
  for (const session of allSessions) {
    if (session.room_id && targets.has(session.room_id)) {
      // `xmod_fk_sessions_room ON DELETE SET NULL`, et le trigger de dérivation
      // remet l'exclusivité de salle à faux — une séance sans salle n'occupe rien.
      const mutable = session as { room_id: Uuid | null; enforce_room_exclusivity: boolean }
      mutable.room_id = null
      mutable.enforce_room_exclusivity = false
      detached += 1
    }
  }
  return detached
}

export function saveRoom(eventId: Uuid, payload: EditionRoomPayload): EditionTabResult {
  if (!payload.name.fr?.trim() || !payload.code.trim()) {
    return { ok: false, detail: null, sessions_detached: 0, error_code: 'required' }
  }
  if (payload.capacity !== null && payload.capacity <= 0) {
    return { ok: false, detail: null, sessions_detached: 0, error_code: 'capacity' }
  }
  // `ux_rooms_code` — unique par LIEU, pas par édition.
  if (
    roomRows.some(
      (r) => r.venue_id === payload.venue_id && r.code === payload.code && r.id !== payload.id,
    )
  ) {
    return { ok: false, detail: null, sessions_detached: 0, error_code: 'code_taken' }
  }

  const existing = payload.id ? roomRows.find((r) => r.id === payload.id) : undefined
  const room: Room =
    existing ??
    {
      id: newId('7024'),
      venue_id: payload.venue_id,
      name: payload.name,
      code: payload.code,
      capacity: null,
      is_virtual: false,
      has_streaming: false,
      equipment: [],
      sort_order: payload.sort_order,
      created_at: new Date().toISOString(),
    }

  const becameVirtual = existing !== undefined && !existing.is_virtual && payload.is_virtual

  Object.assign(room, {
    name: payload.name,
    code: payload.code,
    capacity: payload.capacity,
    is_virtual: payload.is_virtual,
    has_streaming: payload.has_streaming,
    equipment: payload.equipment,
    sort_order: payload.sort_order,
  })
  if (!existing) roomRows.push(room)

  // `tg_sessions_derive_fields()` recalcule `enforce_room_exclusivity` depuis
  // `is_virtual`. Basculer une salle en virtuelle fait donc taire ses conflits de
  // double réservation : c'est voulu, et l'écran l'annonce plutôt que de laisser
  // le planificateur se vider de ses signalements sans explication.
  if (becameVirtual || (existing && existing.is_virtual !== payload.is_virtual)) {
    for (const session of allSessions) {
      if (session.room_id === room.id) {
        ;(session as { enforce_room_exclusivity: boolean }).enforce_room_exclusivity =
          !payload.is_virtual
      }
    }
  }

  return { ok: true, detail: editionDetail(eventId), sessions_detached: 0, error_code: null }
}

export function removeRoom(eventId: Uuid, roomId: Uuid): EditionTabResult {
  const index = roomRows.findIndex((r) => r.id === roomId)
  if (index === -1) return { ok: false, detail: null, sessions_detached: 0, error_code: 'not_found' }

  roomRows.splice(index, 1)
  const detached = detachSessionsFromRooms([roomId])
  return { ok: true, detail: editionDetail(eventId), sessions_detached: detached, error_code: null }
}

// ---------------------------------------------------------------------------
// 5. LES CANAUX DE DIFFUSION
// ---------------------------------------------------------------------------

/**
 * CRÉER OU MODIFIER UN CANAL.
 *
 * `ux_broadcast_channels_default` n'autorise QU'UN SEUL canal par défaut et par
 * édition, parmi les canaux actifs. Poser le défaut sur un canal le retire donc
 * du précédent — l'écran le fait explicitement plutôt que de heurter l'index.
 */
export function saveChannel(payload: EditionChannelPayload): EditionTabResult {
  if (!payload.name.fr?.trim() || !payload.code.trim()) {
    return { ok: false, detail: null, sessions_detached: 0, error_code: 'required' }
  }
  // `ux_broadcast_channels_code UNIQUE NULLS NOT DISTINCT (event_id, code)`.
  if (
    channelRows.some(
      (c) => c.event_id === payload.event_id && c.code === payload.code && c.id !== payload.id,
    )
  ) {
    return { ok: false, detail: null, sessions_detached: 0, error_code: 'code_taken' }
  }

  const existing = payload.id ? channelRows.find((c) => c.id === payload.id) : undefined
  if (existing && existing.event_id === null) {
    // Un canal général de la plateforme ne se modifie pas depuis une édition.
    return { ok: false, detail: null, sessions_detached: 0, error_code: 'platform_channel' }
  }

  const now = new Date().toISOString()
  const channel: BroadcastChannel =
    existing ??
    {
      id: newId('7025'),
      event_id: payload.event_id,
      code: payload.code,
      name: payload.name,
      provider: payload.provider,
      channel_ref: null,
      locale: null,
      is_default: false,
      is_active: true,
      created_at: now,
      updated_at: now,
    }

  Object.assign(channel, {
    code: payload.code,
    name: payload.name,
    provider: payload.provider,
    channel_ref: payload.channel_ref,
    locale: payload.locale,
    is_default: payload.is_default,
    is_active: payload.is_active,
    updated_at: now,
  })
  if (!existing) channelRows.push(channel)

  if (payload.is_default && payload.is_active) {
    for (const other of channelRows) {
      if (other.id !== channel.id && other.event_id === payload.event_id) other.is_default = false
    }
  }

  return { ok: true, detail: editionDetail(payload.event_id), sessions_detached: 0, error_code: null }
}

/**
 * DÉSACTIVER UN CANAL plutôt que le supprimer.
 *
 * `is_active` existe pour cela : les séances passées gardent la trace du canal
 * sur lequel elles ont été diffusées, et une suppression les détacherait
 * (`ON DELETE SET NULL`). On ne supprime que ce qui n'a jamais servi.
 */
export function removeChannel(eventId: Uuid, channelId: Uuid): EditionTabResult {
  const channel = channelRows.find((c) => c.id === channelId && c.event_id === eventId)
  if (!channel) return { ok: false, detail: null, sessions_detached: 0, error_code: 'not_found' }

  const used = allSessions.filter((s) => s.broadcast_channel_id === channelId).length
  if (used > 0) {
    channel.is_active = false
    channel.is_default = false
    channel.updated_at = new Date().toISOString()
    return { ok: true, detail: editionDetail(eventId), sessions_detached: 0, error_code: 'deactivated' }
  }

  channelRows.splice(channelRows.indexOf(channel), 1)
  return { ok: true, detail: editionDetail(eventId), sessions_detached: 0, error_code: null }
}

// ---------------------------------------------------------------------------
// 6. L'APPEL À PROPOSITIONS ET SA GRILLE
// ---------------------------------------------------------------------------

function validateCall(payload: EditionCallPayload): CallFormError[] {
  const errors: CallFormError[] = []
  const push = (code: CallFormError['code'], field: string | null, index: number | null = null) =>
    errors.push({ code, field, criterion_index: index })

  if (!payload.title.fr?.trim()) push('required', 'title')
  if (!payload.code.trim()) push('required', 'code')

  // `ck_calls_window`
  if (Date.parse(payload.closes_at) <= Date.parse(payload.opens_at)) push('window', 'closes_at')
  // `ck_calls_extension` — une prolongation dépasse toujours la clôture annoncée.
  if (payload.extended_until && Date.parse(payload.extended_until) <= Date.parse(payload.closes_at)) {
    push('extension', 'extended_until')
  }
  // `ck_calls_speakers`
  if (payload.max_speakers < payload.min_speakers) push('speakers', 'max_speakers')
  // `ck_calls_duration_bounds` — bornes, puis durée par défaut PROPOSABLE.
  if (
    payload.min_duration_minutes < 15 ||
    payload.min_duration_minutes > 600 ||
    payload.max_duration_minutes < payload.min_duration_minutes ||
    payload.max_duration_minutes > 600 ||
    payload.default_duration_minutes < payload.min_duration_minutes ||
    payload.default_duration_minutes > payload.max_duration_minutes
  ) {
    push('duration_bounds', 'default_duration_minutes')
  }
  // `ck_calls_daily_window`
  if (payload.daily_end_time <= payload.daily_start_time) push('daily_window', 'daily_end_time')

  // `ux_calls_one_per_event` — la cardinalité 0..1, tenue par la base.
  const rival = calls.find(
    (c) => c.event_id === payload.event_id && c.status !== 'cancelled' && c.id !== payload.id,
  )
  if (rival) push('already_exists', null)

  // `ux_calls_code`
  if (calls.some((c) => c.event_id === payload.event_id && c.code === payload.code && c.id !== payload.id)) {
    push('code_taken', 'code')
  }

  // Une grille vide n'évalue rien : `refresh_proposal_score()` ne pourrait poser
  // aucune note, et le comité se retrouverait devant une fiche sans critère.
  if (payload.criteria.length === 0) push('criteria_empty', 'criteria')

  const seen = new Set<string>()
  payload.criteria.forEach((criterion, index) => {
    if (!criterion.code.trim() || !criterion.label.fr?.trim()) push('required', 'criteria', index)
    if (seen.has(criterion.code)) push('criterion_code_duplicate', 'criteria', index)
    seen.add(criterion.code)
  })

  return errors
}

/**
 * ENREGISTRER L'APPEL ET SA GRILLE.
 *
 * LA GRILLE PART AVEC L'APPEL. Deux enregistrements distincts laisseraient
 * exister un appel sans critère, ce qu'aucun écran ne pourrait ensuite évaluer.
 *
 * UN CRITÈRE DÉJÀ NOTÉ N'EST PAS SUPPRIMÉ EN SILENCE : `programme.review_scores`
 * le référence, et la réponse porte `scores_affected` pour que l'écran dise que
 * les moyennes vont bouger. La v1 laissait modifier un barème sans rien annoncer,
 * et les notes affichées cessaient de correspondre à la grille.
 */
export function saveCall(payload: EditionCallPayload, actorId: Uuid | null): CallSaveResult {
  const errors = validateCall(payload)
  if (errors.length > 0) return { ok: false, call: null, errors, scores_affected: false }

  const now = new Date().toISOString()
  const existing = payload.id ? calls.find((c) => c.id === payload.id) : undefined

  const call: CallForProposals =
    existing ??
    {
      id: newId('7030'),
      event_id: payload.event_id,
      code: payload.code,
      title: payload.title,
      description: null,
      status: 'draft',
      opens_at: payload.opens_at,
      closes_at: payload.closes_at,
      extended_until: null,
      results_expected_at: null,
      max_proposals_per_organization: null,
      requires_verified_organization: false,
      min_speakers: 1,
      max_speakers: 10,
      default_duration_minutes: 60,
      min_duration_minutes: 45,
      max_duration_minutes: 150,
      daily_start_time: '09:00:00',
      daily_end_time: '17:00:00',
      allowed_formats: ['online', 'in_person', 'hybrid'],
      required_reviews: 2,
      blind_review: true,
      guidelines_url: null,
      created_by: actorId,
      created_at: now,
      updated_at: now,
    }

  Object.assign(call, {
    code: payload.code,
    title: payload.title,
    description: payload.description,
    status: payload.status,
    opens_at: payload.opens_at,
    closes_at: payload.closes_at,
    extended_until: payload.extended_until,
    results_expected_at: payload.results_expected_at,
    max_proposals_per_organization: payload.max_proposals_per_organization,
    requires_verified_organization: payload.requires_verified_organization,
    min_speakers: payload.min_speakers,
    max_speakers: payload.max_speakers,
    default_duration_minutes: payload.default_duration_minutes,
    min_duration_minutes: payload.min_duration_minutes,
    max_duration_minutes: payload.max_duration_minutes,
    daily_start_time: payload.daily_start_time,
    daily_end_time: payload.daily_end_time,
    allowed_formats: payload.allowed_formats,
    required_reviews: payload.required_reviews,
    blind_review: payload.blind_review,
    guidelines_url: payload.guidelines_url,
    updated_at: now,
  })

  const isCreation = existing === undefined
  if (isCreation) calls.push(call)

  const affected = applyCriteria(call.id, payload.criteria)

  return { ok: true, call: editionCall(payload.event_id), errors: [], scores_affected: affected }
}

/** Remplace la grille d'un appel. Rend vrai si des notes existantes sont concernées. */
function applyCriteria(callId: Uuid, wanted: EditionCriterion[]): boolean {
  const before = criterionRows(callId)
  const keptIds = new Set(wanted.map((c) => c.id).filter((id): id is Uuid => id !== null))
  let affected = false

  for (let i = criteria.length - 1; i >= 0; i -= 1) {
    const criterion = criteria[i]!
    if (criterion.call_id !== callId || keptIds.has(criterion.id)) continue
    if ((before.find((c) => c.id === criterion.id)?.score_count ?? 0) > 0) affected = true
    criteria.splice(i, 1)
  }

  wanted.forEach((entry, index) => {
    const sort_order = (index + 1) * 10
    const existing = entry.id ? criteria.find((c) => c.id === entry.id) : undefined
    if (existing) {
      const changed =
        existing.max_score !== entry.max_score ||
        existing.weight !== entry.weight ||
        existing.is_knockout !== entry.is_knockout
      if (changed && (before.find((c) => c.id === existing.id)?.score_count ?? 0) > 0) {
        affected = true
      }
      Object.assign(existing, {
        code: entry.code,
        label: entry.label,
        description: entry.description,
        max_score: entry.max_score,
        weight: entry.weight,
        is_knockout: entry.is_knockout,
        sort_order,
      })
      return
    }
    criteria.push({
      id: newId('7031'),
      call_id: callId,
      code: entry.code,
      label: entry.label,
      description: entry.description,
      max_score: entry.max_score,
      weight: entry.weight,
      is_knockout: entry.is_knockout,
      sort_order,
    })
  })

  return affected
}

/**
 * LA GRILLE PAR DÉFAUT — `event.seed_default_criteria()`.
 *
 * Proposée à la création d'un appel, et rien de plus : six critères alignés sur
 * les usages de l'IFDD, modifiables appel par appel. Le formulaire s'ouvre ainsi
 * pré-rempli plutôt que devant six lignes vides — la grille par défaut existe en
 * base précisément pour cela.
 */
export function defaultCriteriaGrid(): EditionCriterion[] {
  return seedDefaultCriteria('', 0).map((criterion, index) => ({
    // Aucun identifiant : ces lignes ne sont pas encore en base.
    id: null,
    code: criterion.code,
    label: criterion.label,
    description: criterion.description,
    max_score: criterion.max_score,
    weight: criterion.weight,
    is_knockout: criterion.is_knockout,
    sort_order: (index + 1) * 10,
    score_count: 0,
  }))
}

// ---------------------------------------------------------------------------
// 7. LE COMITÉ DE SÉLECTION
// ---------------------------------------------------------------------------

/**
 * ENREGISTRER LA COMPOSITION DU COMITÉ, D'UN SEUL GESTE.
 *
 * La liste envoyée REMPLACE la précédente : ajouts, retraits et plafonds partent
 * ensemble. Un comité se compose en le regardant en entier — répartir la charge
 * suppose de voir tout le monde —, et trois appels séparés auraient permis
 * d'enregistrer un plafond sur quelqu'un qu'on vient de retirer.
 *
 * SIÉGER N'ACCORDE AUCUN DROIT : `event.call_reviewers` dit la composition,
 * `identity.role_assignments` dit l'accès. La réponse rend donc, pour chaque
 * membre, s'il détient bien `programme.review.write` sur l'édition — sans quoi on
 * confierait des dossiers à quelqu'un qui ne peut pas les ouvrir.
 */
export function saveCommittee(payload: CommitteePayload, eventId: Uuid): CommitteeSaveResult {
  const before = committeeOfCall(payload.call_id, eventId)
  const wanted = new Map(payload.members.map((m) => [m.person_id, m]))
  const now = new Date().toISOString()

  const removed = before
    .filter((member) => !wanted.has(member.person_id) && member.assigned_count > 0)
    .map((member) => ({ full_name: member.full_name, assigned_count: member.assigned_count }))

  for (let i = reviewers.length - 1; i >= 0; i -= 1) {
    const row = reviewers[i]!
    if (row.call_id === payload.call_id && !wanted.has(row.person_id)) reviewers.splice(i, 1)
  }

  for (const member of payload.members) {
    const existing = reviewers.find(
      (r) => r.call_id === payload.call_id && r.person_id === member.person_id,
    )
    if (existing) {
      existing.is_lead = member.is_lead
      existing.workload_cap = member.workload_cap
      continue
    }
    reviewers.push({
      call_id: payload.call_id,
      person_id: member.person_id,
      is_lead: member.is_lead,
      workload_cap: member.workload_cap,
      added_at: now,
    })
  }

  return {
    ok: true,
    members: committeeOfCall(payload.call_id, eventId),
    removed_with_assignments: removed,
  }
}
