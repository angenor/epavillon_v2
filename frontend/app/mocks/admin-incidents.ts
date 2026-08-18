/**
 * MESSAGES D'INCIDENT (A13) — les compositions de l'écran et ses écritures.
 *
 * `incidents.ts` porte la DONNÉE, ce fichier porte ce que l'écran en fait :
 * l'état calculé, le balayage de portée, la cible résolue, la liste, et les
 * quatre écritures. La séparation suit celle du modèle — une table d'un côté,
 * `live.event_incidents()` et les deux fonctions de publication de l'autre.
 *
 * ── CE QUE CE FICHIER REJOUE, ET DANS LE MÊME ORDRE QUE LA BASE ─────────────
 *
 *   `live.event_incidents(event, at)`         état, portée, cible résolue
 *   `live.active_incidents_for_event(...)`    sa part active, rien d'autre
 *   `live.publish_incident(id)`               publication horodatée et attribuée
 *   `live.unpublish_incident(id, motif)`      dépublication tracée, avec motif
 *
 * ── LA PERSISTANCE DE SESSION ───────────────────────────────────────────────
 *
 * Même principe qu'en A11 et A12 : l'EFFET de l'action est le sujet de l'écran.
 * Publier un message et voir la liste réapparaître inchangée donnerait à voir un
 * formulaire qui ne fait rien — or ce message est censé parler à toute une COP
 * dans la seconde. Portée : un module, donc jusqu'au prochain rechargement.
 * Rien de ce qui est écrit dans `incidents.ts` n'est modifié.
 */

import type {
  CreateIncidentPayload,
  IncidentListScreen,
  IncidentState,
  IncidentTargetOption,
  IncidentTargets,
  IncidentWriteResult,
  LiveDesk,
  LiveDeskSession,
  ManagedIncident,
  UnpublishIncidentPayload,
  UpdateIncidentPayload,
} from '~/types/admin-incidents'
import type { TemporalState } from '~/types/views'
import type { EventIncident } from '~/types/admin-dashboard'
import type { EffectivePermission } from '~/types/identity'
import type { Incident } from '~/types/live'
import type { Uuid } from '~/types/shared'
import { hasPermission } from '~/utils/permissions'
import { resolveI18nText } from '~/utils/i18n-text'
import { dayKeyInZone } from '~/utils/datetime'
import { validateIncident } from '~/utils/incident-list'
import { eventDays, events } from './event'
import { incidents } from './incidents'
import { organizations } from './org'
import { people } from './people'
import { taxonomyTerms } from './reference'
import { rooms } from './rooms'
import { allSessions } from './sessions'

// ---------------------------------------------------------------------------
// Le journal d'écritures de la session de démonstration
// ---------------------------------------------------------------------------

/** Messages rédigés pendant la session. */
const addedIncidents: Incident[] = []
/** Corrections apportées à un message existant, par identifiant. */
const patches = new Map<Uuid, Partial<Incident>>()

/**
 * Compteur d'identifiants de la session. Les identifiants écrits à la main
 * s'arrêtent à `INCIDENT` n° 7 ; ceux-ci partent de 900 pour qu'un identifiant
 * croisé dans une console se rattache sans ambiguïté à une écriture de
 * démonstration plutôt qu'à une donnée du jeu.
 */
let nextIncidentSeq = 900

function nextIncidentId(): Uuid {
  nextIncidentSeq += 1
  return `01930000-7090-7000-8000-${String(nextIncidentSeq).padStart(12, '0')}`
}

/** Tous les messages, écritures de la session comprises. */
function effectiveIncidents(): Incident[] {
  return [...incidents, ...addedIncidents].map((incident) => {
    const patch = patches.get(incident.id)
    return patch ? { ...incident, ...patch } : incident
  })
}

const now = (): string => new Date().toISOString()

// ---------------------------------------------------------------------------
// live.event_incidents()
// ---------------------------------------------------------------------------

/**
 * Équivalent de `live.event_incidents(event_id, at)` — 080_live.sql § 5,
 * ajoutée au modèle le 18/08 pour cet écran.
 *
 * ELLE DESCEND LA HIÉRARCHIE — édition, ses journées, ses séances, les
 * organisations qui y animent — là où `live.active_incidents(session)` la
 * remonte, et elle rend TOUS les états : le back-office doit voir ce qui va
 * parler, ce qui attend une décision et ce qui a parlé.
 */
export function eventIncidents(eventId: string, at: number = Date.now()): ManagedIncident[] {
  return effectiveIncidents()
    .filter((incident) => concernsEvent(incident, eventId))
    .map((incident) => managedIncident(incident, at))
    .sort(compareIncidents)
}

/**
 * Équivalent de `live.active_incidents_for_event(event_id, at)`.
 *
 * ÉCRITE AU-DESSUS DE LA PRÉCÉDENTE, comme en base : deux balayages de portée
 * qui divergent, et le même incident s'affiche dans le tableau de bord sans
 * apparaître dans l'écran des messages.
 */
export function activeIncidentsForEvent(eventId: string, at: number = Date.now()): EventIncident[] {
  return eventIncidents(eventId, at)
    .filter((incident) => incident.state === 'active')
    .map((incident) => ({
      incident_id: incident.incident_id,
      scope: incident.scope,
      severity: incident.severity,
      kind_code: incident.kind_code,
      title: incident.title,
      message: incident.message,
      target_label: incident.target_label,
      display_from: incident.display_from,
      display_until: incident.display_until,
    }))
}

function managedIncident(incident: Incident, at: number): ManagedIncident {
  return {
    incident_id: incident.id,
    scope: incident.scope,
    severity: incident.severity,
    kind_code: incident.incident_kind_code,
    title: incident.title,
    message: incident.message,
    action_url: incident.action_url,
    is_dismissible: incident.is_dismissible,
    display_from: incident.display_from,
    display_until: incident.display_until,
    target_id:
      incident.session_id ?? incident.event_day_id ?? incident.organization_id ?? incident.event_id,
    target_label: targetLabel(incident),
    state: incidentState(incident, at),
    published_at: incident.published_at,
    published_by: incident.published_by,
    published_by_name: personName(incident.published_by),
    unpublished_at: incident.unpublished_at,
    unpublished_by_name: personName(incident.unpublished_by),
    unpublish_reason: incident.unpublish_reason,
    created_at: incident.created_at,
    updated_at: incident.updated_at,
  }
}

/** Les cinq états, dans l'ordre exact où la fonction SQL les décide. */
function incidentState(incident: Incident, at: number): IncidentState {
  if (incident.unpublished_at !== null) return 'unpublished'
  if (incident.published_at === null) return 'draft'
  if (incident.display_until !== null && Date.parse(incident.display_until) <= at) return 'expired'
  if (Date.parse(incident.display_from) > at) return 'scheduled'
  return 'active'
}

/**
 * La portée, prise par l'autre bout : cet incident concerne-t-il cette édition ?
 *
 * Portée `organization` : l'incident ne la concerne que si l'organisation y
 * anime effectivement une séance. Une ONG en panne de visioconférence sur une
 * autre COP n'a rien à faire ici.
 */
function concernsEvent(incident: Incident, eventId: string): boolean {
  switch (incident.scope) {
    case 'global':
      return true
    case 'event':
      return incident.event_id === eventId
    case 'event_day':
      return eventDays.some((day) => day.id === incident.event_day_id && day.event_id === eventId)
    case 'session':
      return allSessions.some((s) => s.id === incident.session_id && s.event_id === eventId)
    case 'organization':
      return allSessions.some(
        (s) => s.organization_id === incident.organization_id && s.event_id === eventId,
      )
  }
}

/**
 * La cible, résolue : le back-office affiche « Atelier de négociation », pas un
 * identifiant. Une journée sans titre est désignée par sa date, exactement comme
 * le fait `to_char(d.day_date, 'DD/MM/YYYY')` en base.
 */
function targetLabel(incident: Incident): string | null {
  if (incident.session_id) {
    const session = allSessions.find((s) => s.id === incident.session_id)
    return session ? resolveI18nText(session.title) : null
  }
  if (incident.event_day_id) {
    const day = eventDays.find((d) => d.id === incident.event_day_id)
    if (!day) return null
    return day.title ? resolveI18nText(day.title) : frenchDate(day.day_date)
  }
  if (incident.organization_id) {
    return organizations.find((o) => o.id === incident.organization_id)?.legal_name ?? null
  }
  if (incident.event_id) {
    const event = events.find((e) => e.id === incident.event_id)
    return event ? resolveI18nText(event.title) : null
  }
  return null
}

function frenchDate(isoDate: string): string {
  const [year, month, day] = isoDate.split('-')
  return `${day}/${month}/${year}`
}

function personName(personId: string | null): string | null {
  if (!personId) return null
  return people.find((p) => p.id === personId)?.display_name ?? null
}

/** Les actifs d'abord, puis ce qui va parler, puis ce qui attend, puis l'historique. */
const STATE_RANK: Record<IncidentState, number> = {
  active: 0,
  scheduled: 1,
  draft: 2,
  expired: 4,
  unpublished: 4,
}
const SEVERITY_ORDER = { info: 0, warning: 1, error: 2, critical: 3 } as const

function compareIncidents(a: ManagedIncident, b: ManagedIncident): number {
  return (
    STATE_RANK[a.state] - STATE_RANK[b.state] ||
    SEVERITY_ORDER[b.severity] - SEVERITY_ORDER[a.severity] ||
    b.display_from.localeCompare(a.display_from)
  )
}

// ---------------------------------------------------------------------------
// L'écran de liste
// ---------------------------------------------------------------------------

export function incidentListScreen(eventId: string, at: number = Date.now()): IncidentListScreen | null {
  const event = events.find((e) => e.id === eventId)
  if (!event) return null

  const rows = eventIncidents(eventId, at)

  return {
    event_id: event.id,
    event_title: event.title,
    timezone: event.timezone,
    zone_label: event.city,
    rows,
    desk: liveDesk(eventId, event.timezone, at, rows),
    counts: {
      active: rows.filter((r) => r.state === 'active').length,
      scheduled: rows.filter((r) => r.state === 'scheduled').length,
      draft: rows.filter((r) => r.state === 'draft').length,
      expired: rows.filter((r) => r.state === 'expired').length,
      unpublished: rows.filter((r) => r.state === 'unpublished').length,
    },
    kinds: taxonomyTerms
      .filter((term) => term.taxonomy_code === 'incident_kind' && term.is_active)
      .sort((a, b) => a.sort_order - b.sort_order),
    targets: incidentTargets(eventId),
  }
}

/** Un message par son identifiant, tel que l'écran de modification le lit. */
export function incidentById(incidentId: string, at: number = Date.now()): ManagedIncident | null {
  const incident = effectiveIncidents().find((row) => row.id === incidentId)
  return incident ? managedIncident(incident, at) : null
}

/**
 * LES CIBLES DE L'ÉDITION, ET RIEN D'AUTRE — règle métier n° 8.
 *
 * Les organisations offertes sont celles qui ANIMENT une séance de l'édition :
 * même critère que la portée `organization` de `live.event_incidents()`. En
 * proposer d'autres donnerait une portée qui ne s'affiche nulle part.
 */
function incidentTargets(eventId: string): IncidentTargets {
  const event = events.find((e) => e.id === eventId)
  const sessions = allSessions.filter((s) => s.event_id === eventId)

  const days: IncidentTargetOption[] = eventDays
    .filter((day) => day.event_id === eventId)
    .sort((a, b) => a.day_date.localeCompare(b.day_date))
    .map((day) => ({
      id: day.id,
      label: day.title ? resolveI18nText(day.title) : frenchDate(day.day_date),
      hint: day.title ? frenchDate(day.day_date) : null,
      starts_at: null,
    }))

  const sessionOptions: IncidentTargetOption[] = sessions
    .slice()
    .sort((a, b) => a.starts_at.localeCompare(b.starts_at))
    .map((session) => ({
      id: session.id,
      label: resolveI18nText(session.title),
      hint: null,
      starts_at: session.starts_at,
    }))

  const organizationIds = [
    ...new Set(sessions.map((s) => s.organization_id).filter((id): id is string => id !== null)),
  ]
  const organizationOptions: IncidentTargetOption[] = organizationIds
    .map((id) => organizations.find((o) => o.id === id))
    .filter((org): org is NonNullable<typeof org> => Boolean(org))
    .map((org) => ({ id: org.id, label: org.legal_name, hint: org.acronym, starts_at: null }))
    .sort((a, b) => a.label.localeCompare(b.label, 'fr'))

  return {
    event: {
      id: eventId,
      label: event ? resolveI18nText(event.title) : '',
      hint: event?.acronym ?? null,
      starts_at: null,
    },
    days,
    sessions: sessionOptions,
    organizations: organizationOptions,
  }
}

// ---------------------------------------------------------------------------
// Le poste de direct
// ---------------------------------------------------------------------------

/** Nombre d'activités montrées quand l'édition n'a rien aujourd'hui. */
const FALLBACK_SESSIONS = 4

/**
 * CE QUI SE JOUE MAINTENANT — les activités du jour, dans le fuseau de
 * l'ÉDITION et non celui du navigateur : à Belém il est 06:00 quand il est
 * 11:00 à Paris, et une équipe qui pilote depuis Québec ne doit pas voir la
 * journée de la veille.
 *
 * LE REPLI EST ASSUMÉ ET DIT. Hors période — et c'est le cas de la COP31, qui se
 * tient en novembre 2027 —, aucune activité ne se tient aujourd'hui. Montrer un
 * bloc vide rendrait le poste inutile onze mois sur douze ; montrer les
 * prochaines activités sans le dire ferait croire à un direct en cours. Le
 * drapeau `is_fallback` porte la différence jusqu'à l'écran.
 *
 * L'ÉTAT TEMPOREL EST CELUI DE `v_public_schedule`, dans le même ordre de
 * décision : annulé, reporté, à venir, en cours, passé. Le recomposer autrement
 * ferait diverger le poste de direct et la programmation publique.
 */
function liveDesk(
  eventId: string,
  timezone: string,
  at: number,
  rows: ManagedIncident[],
): LiveDesk {
  const today = dayKeyInZone(new Date(at), timezone)
  const sessions = allSessions.filter((session) => session.event_id === eventId)

  const ofToday = sessions
    .filter((session) => dayKeyInZone(session.starts_at, timezone) === today)
    .sort((a, b) => a.starts_at.localeCompare(b.starts_at))

  const upcoming = sessions
    .filter((session) => Date.parse(session.starts_at) > at)
    .sort((a, b) => a.starts_at.localeCompare(b.starts_at))
    .slice(0, FALLBACK_SESSIONS)

  const retained = ofToday.length > 0 ? ofToday : upcoming

  return {
    day: today,
    is_fallback: ofToday.length === 0,
    sessions: retained.map((session) => ({
      session_id: session.id,
      title: session.title,
      starts_at: session.starts_at,
      ends_at: session.ends_at,
      room_name: rooms.find((room) => room.id === session.room_id)?.name ?? null,
      is_streamed: session.is_streamed,
      status: session.status,
      temporal_state: temporalStateOf(session.status, session.starts_at, session.ends_at, at),
      // Ce qui est DÉJÀ dit sur cette activité : publier deux fois la même panne
      // est le meilleur moyen que le public cesse de lire les bandeaux.
      active_incident_count: rows.filter(
        (row) => row.state === 'active' && row.scope === 'session' && row.target_id === session.id,
      ).length,
    })),
  }
}

function temporalStateOf(
  status: string,
  startsAt: string,
  endsAt: string,
  at: number,
): TemporalState {
  if (status === 'cancelled') return 'cancelled'
  if (status === 'postponed') return 'postponed'
  if (at < Date.parse(startsAt)) return 'upcoming'
  if (at <= Date.parse(endsAt)) return 'ongoing'
  return 'past'
}

// ---------------------------------------------------------------------------
// Les quatre écritures
// ---------------------------------------------------------------------------

/**
 * RÉDIGER UN MESSAGE, ET ÉVENTUELLEMENT LE PUBLIER.
 *
 * Les trois refus de validation traduisent les contraintes de la table, dans
 * l'ordre où la base les vérifierait. Le quatrième — `forbidden` — n'est pas une
 * contrainte : c'est `live.incident.publish` sur l'édition visée, qui manque.
 * Le bouton n'est pas offert sans elle, mais masquer un bouton n'a jamais
 * empêché une requête.
 */
export function createIncident(
  payload: CreateIncidentPayload,
  actorId: string | null,
  granted: EffectivePermission[],
): IncidentWriteResult {
  if (!hasPermission(granted, 'live.incident.publish', payload.from_event_id)) {
    return { status: 'forbidden', incident: null }
  }

  const issues = validateIncident(payload)
  if (issues.length > 0) return { status: issues[0]!, incident: null }

  const timestamp = now()
  const incident: Incident = {
    id: nextIncidentId(),
    scope: payload.scope,
    event_id: payload.scope === 'event' ? payload.event_id : null,
    event_day_id: payload.scope === 'event_day' ? payload.event_day_id : null,
    session_id: payload.scope === 'session' ? payload.session_id : null,
    organization_id: payload.scope === 'organization' ? payload.organization_id : null,
    incident_kind_code: payload.incident_kind_code,
    severity: payload.severity,
    title: payload.title,
    message: payload.message,
    action_url: payload.action_url,
    is_dismissible: payload.is_dismissible,
    display_from: payload.display_from,
    display_until: payload.display_until,
    published_at: payload.publish ? timestamp : null,
    published_by: payload.publish ? actorId : null,
    unpublished_at: null,
    unpublished_by: null,
    unpublish_reason: null,
    created_by: actorId,
    created_at: timestamp,
    updated_at: timestamp,
  }

  addedIncidents.push(incident)

  return {
    status: payload.publish ? 'published' : 'created',
    incident: managedIncident(incident, Date.now()),
  }
}

/**
 * CORRIGER UN MESSAGE.
 *
 * PUBLIER À NOUVEAU EFFACE LA DÉPUBLICATION, exactement comme
 * `live.publish_incident()`, qui remet `unpublished_at`, `unpublished_by` et
 * `unpublish_reason` à NULL. Un message rétabli n'est pas un message qui reste
 * marqué comme retiré.
 */
export function updateIncident(
  payload: UpdateIncidentPayload,
  actorId: string | null,
  granted: EffectivePermission[],
): IncidentWriteResult {
  if (!hasPermission(granted, 'live.incident.publish', payload.from_event_id)) {
    return { status: 'forbidden', incident: null }
  }

  const current = effectiveIncidents().find((row) => row.id === payload.incident_id)
  if (!current) return { status: 'not_found', incident: null }

  const issues = validateIncident(payload)
  if (issues.length > 0) return { status: issues[0]!, incident: null }

  const timestamp = now()
  const patch: Partial<Incident> = {
    scope: payload.scope,
    event_id: payload.scope === 'event' ? payload.event_id : null,
    event_day_id: payload.scope === 'event_day' ? payload.event_day_id : null,
    session_id: payload.scope === 'session' ? payload.session_id : null,
    organization_id: payload.scope === 'organization' ? payload.organization_id : null,
    incident_kind_code: payload.incident_kind_code,
    severity: payload.severity,
    title: payload.title,
    message: payload.message,
    action_url: payload.action_url,
    is_dismissible: payload.is_dismissible,
    display_from: payload.display_from,
    display_until: payload.display_until,
    updated_at: timestamp,
  }

  if (payload.publish) {
    patch.published_at = current.published_at ?? timestamp
    patch.published_by = current.published_by ?? actorId
    patch.unpublished_at = null
    patch.unpublished_by = null
    patch.unpublish_reason = null
  }

  patches.set(payload.incident_id, { ...patches.get(payload.incident_id), ...patch })

  return {
    status: payload.publish && current.published_at === null ? 'published' : 'updated',
    incident: incidentById(payload.incident_id),
  }
}

/** PUBLIER — `live.publish_incident()`, horodatée et attribuée. */
export function publishIncident(
  incidentId: string,
  eventId: string,
  actorId: string | null,
  granted: EffectivePermission[],
): IncidentWriteResult {
  if (!hasPermission(granted, 'live.incident.publish', eventId)) {
    return { status: 'forbidden', incident: null }
  }

  const current = effectiveIncidents().find((row) => row.id === incidentId)
  if (!current) return { status: 'not_found', incident: null }

  const timestamp = now()
  patches.set(incidentId, {
    ...patches.get(incidentId),
    published_at: current.published_at ?? timestamp,
    published_by: current.published_by ?? actorId,
    unpublished_at: null,
    unpublished_by: null,
    unpublish_reason: null,
    updated_at: timestamp,
  })

  return { status: 'published', incident: incidentById(incidentId) }
}

/**
 * DÉPUBLIER EN UN CLIC — `live.unpublish_incident(id, motif)`.
 *
 * `not_published` traduit l'exception que la fonction lève sur un message jamais
 * publié : retirer un brouillon n'a aucun sens, et le silence laisserait croire
 * que le bandeau vient d'être retiré du site.
 *
 * LA LIGNE RESTE. Une dépublication n'efface rien : elle horodate, attribue et
 * garde le motif — c'est l'historique que l'écran affiche.
 */
export function unpublishIncident(
  payload: UnpublishIncidentPayload,
  eventId: string,
  actorId: string | null,
  granted: EffectivePermission[],
): IncidentWriteResult {
  if (!hasPermission(granted, 'live.incident.publish', eventId)) {
    return { status: 'forbidden', incident: null }
  }

  const current = effectiveIncidents().find((row) => row.id === payload.incident_id)
  if (!current) return { status: 'not_found', incident: null }
  if (current.published_at === null) return { status: 'not_published', incident: null }

  const timestamp = now()
  patches.set(payload.incident_id, {
    ...patches.get(payload.incident_id),
    unpublished_at: timestamp,
    unpublished_by: actorId,
    unpublish_reason: payload.reason?.trim() || null,
    updated_at: timestamp,
  })

  return { status: 'unpublished', incident: incidentById(payload.incident_id) }
}

/** Le brouillon que le raccourci « Signaler un débordement » pré-remplit. */
export function overrunTemplate(sessionId: string): {
  session_id: string
  title: string
  starts_at: string
  ends_at: string
  event_id: string
} | null {
  const session = allSessions.find((s) => s.id === sessionId)
  if (!session) return null

  return {
    session_id: session.id,
    title: resolveI18nText(session.title),
    starts_at: session.starts_at,
    ends_at: session.ends_at,
    event_id: session.event_id,
  }
}
