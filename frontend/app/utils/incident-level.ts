import type { ActiveIncident, IncidentScope, IncidentSeverity } from '~/types/live'
import type { Uuid } from '~/types/shared'

/**
 * Trois niveaux pour le visiteur, nommés par ce qu'ils changent pour lui
 * (maquette `direction-condensee/messages-regie.html`, validée le 02/11) :
 * l'activité a lieu (information), il peut être gêné (attention), elle ne se
 * suit pas (interruption). La base garde ses quatre gravités.
 */
export type IncidentLevel = 'information' | 'attention' | 'interruption'

const LEVELS: Record<IncidentSeverity, IncidentLevel> = {
  info: 'information',
  warning: 'attention',
  error: 'interruption',
  critical: 'interruption',
}

export const incidentLevel = (severity: IncidentSeverity): IncidentLevel => LEVELS[severity]

export const INCIDENT_LEVEL_ICONS: Record<IncidentLevel, string> = {
  information: 'info',
  attention: 'warning',
  interruption: 'error',
}

const EDITION_WIDE: IncidentScope[] = ['global', 'event', 'event_day']

/** Toute l'édition, une journée ou la plateforme : la bande sous la navigation. */
export const isEditionWide = (incident: ActiveIncident): boolean => EDITION_WIDE.includes(incident.scope)

/** Seule une information se referme ; ce qui gêne ou interrompt reste lu. */
export const canDismissIncident = (incident: ActiveIncident): boolean =>
  incident.is_dismissible && incidentLevel(incident.severity) === 'information'

/** Les messages qui visent une activité ou son organisation, pour sa carte. L'API trie déjà, le plus grave en tête. */
export function incidentFor(
  incidents: ActiveIncident[],
  session: { id: Uuid; organization_id?: Uuid | null },
): ActiveIncident | null {
  return (
    incidents.find(
      (incident) =>
        (incident.scope === 'session' && incident.target_id === session.id)
        || (incident.scope === 'organization' && !!session.organization_id && incident.target_id === session.organization_id),
    ) ?? null
  )
}
