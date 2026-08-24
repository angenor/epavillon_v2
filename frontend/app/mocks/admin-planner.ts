/**
 * LE PLANIFICATEUR DE CRÉNEAUX (A9) — composition de l'écran et ses quatre
 * écritures, rejouées sur les données simulées.
 *
 * ── CE QUE CE FICHIER NE FAIT PAS ───────────────────────────────────────────
 *
 * IL NE REFUSE AUCUN PLACEMENT. Pas une ligne ici ne peut rendre « créneau
 * occupé » : le modèle ne pose aucune contrainte d'exclusion sur les créneaux,
 * et c'est une décision structurante du projet (`075` § 1, ADR-13). Les
 * organisations proposent sans se coordonner, l'équipe réorganise par
 * déplacements successifs, et un état transitoire incohérent — deux blocs
 * superposés le temps de recaler le second — fait partie du travail. On détecte
 * et on affiche ; on n'empêche pas.
 *
 * Le seul endroit où un refus a du sens est `publishProgramme()`, qui rejoue
 * `publication_readiness()` : là, un point bloquant retient la publication.
 *
 * ── LES ÉCRITURES MUTENT LES SÉANCES EN MÉMOIRE ─────────────────────────────
 *
 * Comme les autres mocks d'écriture du projet, les changements vivent le temps
 * de la session du navigateur et disparaissent au rechargement. Ils modifient
 * l'objet `Session` lui-même, partagé par `allSessions`, `publicSchedule()` et
 * `detectConflicts()` : un bloc déplacé change donc les conflits de toute
 * l'édition dans la même seconde, ce qui est précisément le comportement attendu
 * de l'écran.
 *
 * ── LES TROIS DÉRIVATIONS DU TRIGGER SONT REJOUÉES ──────────────────────────
 *
 * `tg_sessions_derive_fields()` pose en base trois valeurs qu'aucun formulaire
 * n'envoie : l'exclusivité de salle (depuis `event.rooms.is_virtual`), le jour
 * de calendrier (depuis la date locale) et le canal de diffusion par défaut. Les
 * rejouer ici n'est pas un ornement : sans la première, un placement en salle
 * virtuelle produirait un faux conflit de stand ; sans la troisième, une séance
 * marquée « diffusée » échapperait à la règle « un seul direct ».
 */

import type {
  PlannerChannel,
  PlannerDay,
  PlannerMutationResult,
  PlannerRoom,
  PlannerScreen,
  PlannerSession,
  PlannerTrack,
  PublishProgrammeResult,
  ScheduleSessionPayload,
  SessionBroadcastPayload,
  SessionTracksPayload,
} from '~/types/admin-planner'
import type { Session, SessionStatus, SessionTrack } from '~/types/programme/session'
import type { Uuid } from '~/types/shared'
import { detectConflicts, publicationReadiness } from './conflicts'
import { events, eventDays } from './event'
import { organizations } from './org'
import { countries } from './reference'
import { allProposals } from './proposals'
import { broadcastChannels, rooms, venues } from './rooms'
import { allSessions, sessionSpeakers, sessionTracks } from './sessions'
import { programmeTracks } from './tracks'
import { termBadges } from './views'

const organizationById = new Map(organizations.map((o) => [o.id, o]))
const countryById = new Map(countries.map((c) => [c.id, c]))
const roomById = new Map(rooms.map((r) => [r.id, r]))
const proposalById = new Map(allProposals.map((p) => [p.id, p]))
/**
 * Une salle ne porte pas son édition : elle porte son LIEU, et c'est le lieu qui
 * porte l'édition (`event.venues.event_id`). Sans cet index, le planificateur de
 * la COP31 offrirait les salles de la COP29.
 */
const eventIdByVenue = new Map(venues.map((venue) => [venue.id, venue.event_id]))

/**
 * Écriture d'une séance : `time_range` et `enforce_room_exclusivity` sont
 * `readonly` dans les types parce que la BASE les écrit seule. Ce type dit qu'on
 * se met ici à la place de la base — et il le dit à un seul endroit.
 */
type MutableSession = { -readonly [K in keyof Session]: Session[K] }

function sessionById(id: Uuid): MutableSession | undefined {
  return allSessions.find((s) => s.id === id) as MutableSession | undefined
}

// ---------------------------------------------------------------------------
// Les dérivations que la base fait par trigger
// ---------------------------------------------------------------------------

/** Date civile d'un instant dans un fuseau donné, au format `AAAA-MM-JJ`. */
function dayKey(instant: string, timeZone: string): string {
  return new Intl.DateTimeFormat('en-CA', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    timeZone,
  }).format(new Date(instant))
}

/**
 * `tg_sessions_derive_fields()`, les trois branches.
 *
 * Le jour de calendrier peut rester nul : une séance tenue hors des douze jours
 * de l'édition — un webinaire préparatoire, par exemple — n'en a aucun, et
 * inventer le jour le plus proche fausserait la programmation publique.
 */
function deriveFields(session: MutableSession): void {
  const room = session.room_id ? roomById.get(session.room_id) : undefined
  session.enforce_room_exclusivity = room !== undefined && !room.is_virtual

  session.time_range = `["${session.starts_at}","${session.ends_at}")`

  const event = events.find((e) => e.id === session.event_id)
  const key = event ? dayKey(session.starts_at, event.timezone) : null
  session.event_day_id =
    eventDays.find((day) => day.event_id === session.event_id && day.day_date === key)?.id ?? null

  if (session.is_streamed && session.broadcast_channel_id === null) {
    session.broadcast_channel_id =
      broadcastChannels.find((c) => c.is_active && c.is_default && c.event_id === session.event_id)?.id ??
      broadcastChannels.find((c) => c.is_active && c.is_default && c.event_id === null)?.id ??
      null
  } else if (!session.is_streamed) {
    session.broadcast_channel_id = null
  }
}

// ---------------------------------------------------------------------------
// Lecture
// ---------------------------------------------------------------------------

/**
 * Une séance, telle que le planificateur la manipule.
 *
 * La note, la durée souhaitée et les contraintes de calendrier viennent du
 * DOSSIER : ce sont elles qui font trier le panneau latéral et qui expliquent un
 * placement. Une séance programmée directement par l'IFDD n'en a aucune, et
 * l'écran doit le tenir sans afficher « 0/20 ».
 */
function toPlannerSession(session: Session): PlannerSession {
  const organization = session.organization_id ? organizationById.get(session.organization_id) : undefined
  const country = organization?.country_id ? countryById.get(organization.country_id) : undefined
  const room = session.room_id ? roomById.get(session.room_id) : undefined
  const proposal = session.proposal_id ? proposalById.get(session.proposal_id) : undefined

  return {
    id: session.id,
    event_id: session.event_id,
    proposal_id: session.proposal_id,
    event_day_id: session.event_day_id,
    title: session.title,
    slug: session.slug,
    summary: session.summary,
    status: session.status,
    format: session.format,
    starts_at: session.starts_at,
    ends_at: session.ends_at,
    timezone: session.timezone,

    room_id: session.room_id,
    room_name: room?.name ?? null,
    enforce_room_exclusivity: session.enforce_room_exclusivity,
    location_note: session.location_note,

    organization_id: session.organization_id,
    organization_name: organization?.legal_name ?? null,
    organization_acronym: organization?.acronym ?? null,
    organization_country_code: country?.iso2 ?? null,

    reference_code: proposal?.reference_code ?? null,
    average_score: proposal?.average_score ?? null,
    requested_duration_minutes: proposal?.duration_minutes ?? null,
    preferred_start_at: proposal?.preferred_start_at ?? null,
    scheduling_constraints: proposal?.scheduling_constraints ?? null,

    is_streamed: session.is_streamed,
    broadcast_channel_id: session.broadcast_channel_id,

    track_ids: (sessionTracks as SessionTrack[])
      .filter((link) => link.session_id === session.id)
      .sort((a, b) => a.sort_order - b.sort_order)
      .map((link) => link.track_id),
    themes: termBadges('sessions', session.id),
    speaker_count: sessionSpeakers.filter((speaker) => speaker.session_id === session.id).length,
    published_at: session.published_at,
  }
}

/**
 * Les séances que le planificateur montre : celles de l'édition, DANS TOUS LEURS
 * ÉTATS.
 *
 * `seances_de_ledition` ne filtre aucun statut, et l'écran doit voir ce que
 * l'API lui enverra : une séance annulée qui occupe encore une salle apparaît
 * bel et bien dans la grille, et c'est justement ce qu'il faut arbitrer. Le tri
 * par statut se fait ailleurs, et deux fois plutôt qu'une : `detect_conflicts()`
 * ne retient que `planned`, `scheduled` et `live` ; la publication, que les deux
 * premiers.
 */
function editionSessions(eventId: Uuid): Session[] {
  return allSessions.filter((s) => s.event_id === eventId)
}

/** TOUT L'ÉCRAN EN UNE RÉPONSE, conflits compris. */
export function plannerScreen(eventId: Uuid): PlannerScreen | null {
  const event = events.find((e) => e.id === eventId)
  if (!event) return null

  const sessions = editionSessions(eventId).map(toPlannerSession)

  const days: PlannerDay[] = eventDays
    .filter((day) => day.event_id === eventId)
    .sort((a, b) => a.day_date.localeCompare(b.day_date))
    .map((day) => ({
      id: day.id,
      day_date: day.day_date,
      title: day.title,
      is_featured: day.is_featured,
      color_hex: day.color_hex,
    }))

  // Les salles de l'édition, virtuelles comprises : une séance en ligne se pose
  // dans une salle virtuelle, qui accepte les créneaux simultanés.
  const eventRooms: PlannerRoom[] = rooms
    .filter((room) => eventIdByVenue.get(room.venue_id) === eventId)
    .sort((a, b) => a.sort_order - b.sort_order)
    .map((room) => ({
      id: room.id,
      name: room.name,
      code: room.code,
      capacity: room.capacity,
      is_virtual: room.is_virtual,
      has_streaming: room.has_streaming,
      sort_order: room.sort_order,
    }))

  const tracks: PlannerTrack[] = programmeTracks
    .filter((track) => track.event_id === eventId)
    .sort((a, b) => a.sort_order - b.sort_order)
    .map((track) => ({
      id: track.id,
      title: track.title,
      kind: track.kind,
      color_hex: track.color_hex,
      starts_on: track.starts_on,
      ends_on: track.ends_on,
    }))

  const channels: PlannerChannel[] = broadcastChannels
    .filter((channel) => channel.is_active && (channel.event_id === eventId || channel.event_id === null))
    .map((channel) => ({
      id: channel.id,
      name: channel.name,
      provider: channel.provider,
      is_default: channel.is_default,
    }))

  return {
    event_id: event.id,
    event_title: event.title,
    timezone: event.timezone,
    zone_label: event.city,
    programme_published_at: event.programme_published_at,
    days,
    rooms: eventRooms,
    tracks,
    channels,
    placed: sessions
      .filter((s) => s.room_id !== null)
      .sort((a, b) => a.starts_at.localeCompare(b.starts_at)),
    unplaced: sessions
      .filter((s) => s.room_id === null)
      // Note décroissante d'abord : c'est l'ordre du comité, et le tri par
      // défaut que le prompt demande. Les séances sans dossier — donc sans note —
      // ferment la liste plutôt que de l'ouvrir.
      .sort((a, b) => (b.average_score ?? -1) - (a.average_score ?? -1)),
    conflicts: detectConflicts(eventId),
  }
}

// ---------------------------------------------------------------------------
// Écritures
// ---------------------------------------------------------------------------

/**
 * PLACER, DÉPLACER, REDIMENSIONNER, RETIRER — la même écriture.
 *
 * AUCUN CONTRÔLE DE CHEVAUCHEMENT, et ce n'est pas un oubli : voir l'en-tête.
 * La réponse rend les conflits recalculés pour TOUTE l'édition, parce qu'un
 * déplacement peut résoudre le conflit d'un autre bloc à l'autre bout de la
 * semaine.
 *
 * PLACER NE CHANGE PAS LE STATUT. `ecrire_le_creneau` n'écrit que `room_id`,
 * `starts_at`, `ends_at` et `event_day_id` ; `scheduled` veut dire « programmé
 * ET publié » (`075` l. 43), et c'est la PUBLICATION qui fait passer `planned` à
 * `scheduled`. Le faire ici marquerait comme publique une séance qui n'est que
 * posée, et le calendrier lui retirerait le fond neutre de l'état de travail.
 */
export function scheduleSession(payload: ScheduleSessionPayload): PlannerMutationResult {
  const session = sessionById(payload.session_id)
  if (!session) throw new Error(`Séance ${payload.session_id} introuvable.`)

  session.room_id = payload.room_id
  session.starts_at = payload.starts_at
  session.ends_at = payload.ends_at
  session.updated_at = new Date().toISOString()
  deriveFields(session)

  return {
    session: toPlannerSession(session),
    conflicts: detectConflicts(session.event_id),
  }
}

/**
 * RATTACHER À UNE JOURNÉE SPÉCIALE — `programme.session_tracks`.
 *
 * MANUEL, et indépendant de la date : toutes les activités du 12 novembre ne
 * font pas partie de la « Journée finance durable ». La liste reçue remplace la
 * précédente. Un fil d'une AUTRE édition est refusé, comme le trigger
 * `tg_session_tracks_check_event()` le refuse en base — c'est le seul refus de
 * ce fichier avec la publication, et il ne porte pas sur un créneau.
 */
export function setSessionTracks(payload: SessionTracksPayload, actorId: Uuid | null): PlannerMutationResult {
  const session = sessionById(payload.session_id)
  if (!session) throw new Error(`Séance ${payload.session_id} introuvable.`)

  const allowed = new Set(
    programmeTracks.filter((track) => track.event_id === session.event_id).map((track) => track.id),
  )
  const kept = payload.track_ids.filter((id) => allowed.has(id))

  // Le tableau importé est muté, comme le ferait un DELETE suivi d'un INSERT :
  // la programmation publique et la page d'une journée spéciale lisent la même
  // source, et deux registres finiraient par diverger.
  const links = sessionTracks as SessionTrack[]
  const others = links.filter((link) => link.session_id !== payload.session_id)
  const now = new Date().toISOString()
  const next: SessionTrack[] = kept.map((trackId, index) => {
    const existing = links.find(
      (link) => link.session_id === payload.session_id && link.track_id === trackId,
    )
    return (
      existing ?? {
        session_id: payload.session_id,
        track_id: trackId,
        sort_order: (index + 1) * 10,
        is_highlight: false,
        // QUI a rattaché QUOI : la composition d'une journée spéciale est un
        // choix éditorial, et il arrive de l'expliquer.
        added_by: actorId,
        added_at: now,
      }
    )
  })

  links.length = 0
  links.push(...others, ...next)

  return { session: toPlannerSession(session), conflicts: detectConflicts(session.event_id) }
}

/**
 * MARQUER UNE SÉANCE COMME DIFFUSÉE, avec son canal.
 *
 * Le canal est posé d'office quand il n'est pas choisi, comme le fait le trigger
 * en base : une séance « diffusée » sans canal échapperait à la détection du
 * double direct, et le contrôle ne vaudrait que pour ceux qui connaissent la
 * règle. Deux directs simultanés restent parfaitement écrivables — ils
 * apparaissent alors au bandeau, en gravité bloquante.
 */
export function setSessionBroadcast(payload: SessionBroadcastPayload): PlannerMutationResult {
  const session = sessionById(payload.session_id)
  if (!session) throw new Error(`Séance ${payload.session_id} introuvable.`)

  session.is_streamed = payload.is_streamed
  session.broadcast_channel_id = payload.is_streamed ? payload.broadcast_channel_id : null
  session.updated_at = new Date().toISOString()
  deriveFields(session)

  return { session: toPlannerSession(session), conflicts: detectConflicts(session.event_id) }
}

/** Le prédicat de la publication, celui de l'API — `STATUTS_A_PUBLIER`. */
const PUBLISHABLE_STATUSES: ReadonlySet<SessionStatus> = new Set(['planned', 'scheduled'])

/**
 * PUBLIER LA PROGRAMMATION — le seul contrôle bloquant de l'écran.
 *
 * `publication_readiness()` rend les points à régler ; un seul de gravité
 * `blocking` retient toute la publication. Les avertissements l'accompagnent
 * sans la retenir : un intervenant attendu à deux endroits est un problème que
 * l'équipe juge, pas une impossibilité matérielle.
 *
 * CE QUI DEVIENT PUBLIC : les séances `planned` ou `scheduled` pas encore
 * publiées — avec ou sans salle. Une séance sans salle mais portant une
 * précision de lieu a passé le contrôle : rien ne justifie de l'écarter ici.
 *
 * REPUBLIER NE PUBLIE RIEN. L'API estampille sous `WHERE
 * programme_published_at IS NULL` : une édition déjà publiée rend sa date
 * d'origine, zéro séance, et n'annonce rien.
 */
export function publishProgramme(eventId: Uuid): PublishProgrammeResult {
  const issues = publicationReadiness(eventId)
  const blocked = issues.some((issue) => issue.severity === 'blocking')

  if (blocked) {
    return { blocked: true, published_count: 0, published_at: null, issues }
  }

  const event = events.find((e) => e.id === eventId)
  if (event?.programme_published_at) {
    return {
      blocked: false,
      published_count: 0,
      published_at: event.programme_published_at,
      issues,
    }
  }

  const now = new Date().toISOString()
  let published = 0
  for (const session of editionSessions(eventId) as MutableSession[]) {
    if (session.published_at !== null) continue
    if (!PUBLISHABLE_STATUSES.has(session.status)) continue
    session.published_at = now
    // `planned` devient `scheduled` — « programmé et publié ». Le consommateur
    // de l'annonce l'écrit dans le même UPDATE que la date, et c'est le seul
    // endroit où ce passage se fait.
    if (session.status === 'planned') session.status = 'scheduled'
    published += 1
  }

  if (event) (event as { programme_published_at: string | null }).programme_published_at = now

  return { blocked: false, published_count: published, published_at: now, issues }
}
