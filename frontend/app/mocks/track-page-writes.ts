/**
 * Les écritures de la page d'un fil, hors ligne (30/10). Elles mutent
 * `trackPageSettings` comme l'API recalcule ses réglages : chaque écriture rend
 * `TrackPageSettings` entier. Les contraintes de 060 § 4 ter refusent ici comme
 * en base, avec un message en français tel que l'API l'écrirait.
 *
 * Un fil créé pendant la session (onglet « Journées spéciales ») reçoit des
 * réglages vides à la première lecture : on peut alors saisir sa page de bout
 * en bout sans API.
 */

import type {
  TrackMomentPayload,
  TrackPageSettings,
  TrackPagePayload,
  TrackPartPayload,
} from '~/types/event/track-page'
import type { TrackSessionPlacement, TrackSessionPlacementPayload } from '~/types/programme/track-page'
import type { TaxonomyTerm } from '~/types/reference'
import type { EventId, I18nText, TrackId, Uuid } from '~/types/shared'
import { trackPageOrganizations, trackPageSettings } from './track-pages'
import { organizationById } from './organization-search'
import { taxonomyTerms } from './reference'
import { ApiRequestError } from '~/utils/api-error'

/** Les quatre rôles semés par 060 § 4 ter, quand le jeu de référence ne les porte pas. */
const ROLES: Array<[string, I18nText, number]> = [
  ['organizer', { fr: 'Organisateur', en: 'Organiser' }, 10],
  ['partner', { fr: 'Partenaire', en: 'Partner' }, 20],
  ['technical_partner', { fr: 'Partenaire technique', en: 'Technical partner' }, 30],
  ['financial_support', { fr: 'Soutien financier', en: 'Financial support' }, 40],
]

export function trackPartnerRoles(): TaxonomyTerm[] {
  const seeded = (taxonomyTerms as TaxonomyTerm[]).filter(
    (term) => term.taxonomy_code === 'track_partner_role' && term.is_active,
  )
  if (seeded.length > 0) return seeded.sort((a, b) => a.sort_order - b.sort_order)
  const now = new Date().toISOString()
  return ROLES.map(([code, label, sort_order]) => ({
    id: `track-partner-role-${code}`,
    taxonomy_code: 'track_partner_role',
    parent_id: null,
    code,
    label,
    description: null,
    color_hex: null,
    icon: null,
    sort_order,
    is_active: true,
    superseded_by: null,
    metadata: {},
    created_at: now,
    updated_at: now,
  }))
}

/** Un refus tel que l'API le rendrait : 422, message français du modèle. */
class Refus extends ApiRequestError {
  constructor(message: string, status = 422) {
    super({ code: status === 404 ? 'NOT_FOUND' : 'VALIDATION_FAILED', message }, status)
  }
}

function settingsOf(trackId: TrackId, eventId: EventId): TrackPageSettings {
  const found = Object.values(trackPageSettings).find((s) => s.track_id === trackId)
  if (found) return found
  const blank: TrackPageSettings = {
    track_id: trackId,
    event_id: eventId,
    venue_id: null,
    tagline: null,
    target_audiences: [],
    support_note: null,
    feature_kicker: null,
    feature_title: null,
    feature_body: null,
    feature_link_label: null,
    feature_link_url: null,
    parts: [],
    moments: [],
    partners: [],
    sessions: [],
  }
  trackPageSettings[trackId] = blank
  return blank
}

// Par JSON et non `structuredClone` : une charge utile venue d'un brouillon
// réactif porte des proxys, que `structuredClone` refuse.
const copy = (s: TrackPageSettings): TrackPageSettings => JSON.parse(JSON.stringify(s))

export function readTrackPage(trackId: TrackId, eventId: EventId): TrackPageSettings {
  return copy(settingsOf(trackId, eventId))
}

function organizationOf(id: Uuid, settings: TrackPageSettings) {
  const known = settings.partners.find((p) => p.organization_id === id)
  if (known) return { name: known.name, acronym: known.acronym, logo: known.logo }
  const seeded = trackPageOrganizations.find((o) => o.id === id)
  if (seeded) return { name: seeded.name, acronym: seeded.acronym, logo: null }
  const listed = organizationById(id)
  if (listed) return { name: listed.legal_name, acronym: listed.acronym, logo: null }
  throw new Refus('Organisation inconnue.')
}

export function saveTrackPage(trackId: TrackId, eventId: EventId, payload: TrackPagePayload): TrackPageSettings {
  const settings = settingsOf(trackId, eventId)
  if ((payload.feature_link_url === null) !== (payload.feature_link_label === null)) {
    throw new Refus('Un lien se donne avec son libellé, ou pas du tout.')
  }
  if (payload.feature_link_url && !/^(https?:\/\/|\/)\S*$/.test(payload.feature_link_url)) {
    throw new Refus('Adresse du lien refusée : une adresse complète (https://…) ou un chemin du site (/…).')
  }
  if (!payload.feature_title && (payload.feature_kicker || payload.feature_body || payload.feature_link_url)) {
    throw new Refus("L'encart mis en avant demande un titre.")
  }
  if (payload.target_audiences.some((a) => !a.fr?.trim())) {
    throw new Refus('Chaque public cible demande son libellé en français.')
  }
  const roles = trackPartnerRoles()
  const seen = new Set<string>()
  const partners = payload.partners.flatMap((p, index) => {
    if (seen.has(p.organization_id)) return []
    seen.add(p.organization_id)
    const code = p.role_code ?? 'partner'
    const role = roles.find((r) => r.code === code)
    if (!role) throw new Refus('Rôle refusé : il ne relève pas des rôles de partenaire d’une journée.')
    return [{
      organization_id: p.organization_id,
      ...organizationOf(p.organization_id, settings),
      role_code: code,
      role_label: role.label,
      sort_order: index,
    }]
  })
  Object.assign(settings, {
    venue_id: payload.venue_id,
    tagline: payload.tagline,
    target_audiences: payload.target_audiences,
    support_note: payload.support_note,
    feature_kicker: payload.feature_kicker,
    feature_title: payload.feature_title,
    feature_body: payload.feature_body,
    feature_link_label: payload.feature_link_label,
    feature_link_url: payload.feature_link_url,
    partners,
  })
  return copy(settings)
}

function checkPart(payload: TrackPartPayload): void {
  if (!payload.label && !payload.title) throw new Refus('Une partie demande un nom court ou un intitulé.')
}

export function createTrackPart(trackId: TrackId, eventId: EventId, payload: TrackPartPayload): TrackPageSettings {
  checkPart(payload)
  const settings = settingsOf(trackId, eventId)
  settings.parts.push({ id: crypto.randomUUID(), track_id: trackId, ...payload })
  settings.parts.sort((a, b) => a.sort_order - b.sort_order)
  return copy(settings)
}

export function updateTrackPart(trackId: TrackId, eventId: EventId, partId: Uuid, payload: TrackPartPayload): TrackPageSettings {
  checkPart(payload)
  const settings = settingsOf(trackId, eventId)
  const part = settings.parts.find((p) => p.id === partId)
  if (!part) throw new Refus('Cette partie n’existe plus.', 404)
  Object.assign(part, payload)
  settings.parts.sort((a, b) => a.sort_order - b.sort_order)
  return copy(settings)
}

export function removeTrackPart(trackId: TrackId, eventId: EventId, partId: Uuid): TrackPageSettings {
  const settings = settingsOf(trackId, eventId)
  settings.parts = settings.parts.filter((p) => p.id !== partId)
  // ON DELETE SET NULL (part_id) : moments et activités restent dans le fil.
  for (const moment of settings.moments) if (moment.part_id === partId) moment.part_id = null
  for (const session of settings.sessions) if (session.part_id === partId) session.part_id = null
  return copy(settings)
}

function checkMoment(settings: TrackPageSettings, payload: TrackMomentPayload): void {
  if (!payload.label.fr?.trim()) throw new Refus('Un moment demande son libellé en français.')
  if (payload.ends_at && Date.parse(payload.ends_at) <= Date.parse(payload.starts_at)) {
    throw new Refus('La fin du moment doit suivre son début.')
  }
  if (payload.part_id && !settings.parts.some((p) => p.id === payload.part_id)) {
    throw new Refus('Partie refusée : elle appartient à un autre fil.')
  }
}

const byStart = (a: { starts_at: string }, b: { starts_at: string }) =>
  Date.parse(a.starts_at) - Date.parse(b.starts_at)

export function createTrackMoment(trackId: TrackId, eventId: EventId, payload: TrackMomentPayload): TrackPageSettings {
  const settings = settingsOf(trackId, eventId)
  checkMoment(settings, payload)
  settings.moments.push({ id: crypto.randomUUID(), track_id: trackId, ...payload })
  settings.moments.sort(byStart)
  return copy(settings)
}

export function updateTrackMoment(trackId: TrackId, eventId: EventId, momentId: Uuid, payload: TrackMomentPayload): TrackPageSettings {
  const settings = settingsOf(trackId, eventId)
  checkMoment(settings, payload)
  const moment = settings.moments.find((m) => m.id === momentId)
  if (!moment) throw new Refus('Ce moment n’existe plus.', 404)
  Object.assign(moment, payload)
  settings.moments.sort(byStart)
  return copy(settings)
}

export function removeTrackMoment(trackId: TrackId, eventId: EventId, momentId: Uuid): TrackPageSettings {
  const settings = settingsOf(trackId, eventId)
  settings.moments = settings.moments.filter((m) => m.id !== momentId)
  return copy(settings)
}

export function placeSessionInTrack(
  trackId: TrackId,
  sessionId: Uuid,
  payload: TrackSessionPlacementPayload,
): TrackSessionPlacement {
  const settings = Object.values(trackPageSettings).find((s) => s.track_id === trackId)
  const entry = settings?.sessions.find((s) => s.session_id === sessionId)
  if (!settings || !entry) throw new Refus('Cette activité n’est pas rattachée au fil.', 404)
  if (payload.part_id && !settings.parts.some((p) => p.id === payload.part_id)) {
    throw new Refus('Partie refusée : elle appartient à un autre fil.')
  }
  Object.assign(entry, payload)
  return { session_id: sessionId, track_id: trackId, ...payload }
}
