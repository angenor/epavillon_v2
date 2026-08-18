/**
 * Données simulées de `live.incidents` — 080_live.sql § 5.
 *
 * QUATRE MESSAGES, ET TROIS D'ENTRE EUX NE DOIVENT PAS S'AFFICHER. C'est tout
 * l'intérêt du jeu : un incident ne se lit pas à sa seule existence mais à
 * quatre conditions cumulées, qu'une implémentation naïve oublie une par une —
 * publié, non dépublié, fenêtre d'affichage ouverte, portée concernée. Le jeu
 * porte donc un incident publié dont la fenêtre est close et un incident jamais
 * publié : deux pièges à ne pas afficher.
 *
 * LE MODULE DIRECT EST HORS JALON, mais le bandeau d'incident est transverse —
 * même raison que pour `types/live.ts`. Ces messages alimentent aujourd'hui le
 * bloc d'actions du tableau de bord (A6) et alimenteront l'écran A13.
 *
 * LE TEXTE EST UNE DONNÉE, PAS UNE TRADUCTION. `title` et `message` sont des
 * `platform.i18n_text` : un administrateur les écrit depuis le back-office, ils
 * n'ont donc rien à faire dans un fichier i18n.
 */

import type { Incident } from '~/types/live'
import type { EventIncident } from '~/types/admin-dashboard'
import { events } from './event'
import { EVENT, INCIDENT, PERSON } from './ids'

/** Valeurs communes à tous les messages : évite de répéter dix colonnes nulles. */
const base = {
  event_id: null,
  event_day_id: null,
  session_id: null,
  organization_id: null,
  action_url: null,
  title: null,
  unpublished_at: null,
  unpublished_by: null,
  unpublish_reason: null,
  created_by: PERSON.bakayoko,
} as const

export const incidents = [
  {
    ...base,
    id: INCIDENT.maintenanceDepot,
    scope: 'global',
    incident_kind_code: 'technical_issue',
    severity: 'warning',
    title: {
      fr: 'Téléversement de documents indisponible',
      en: 'Document upload unavailable',
    },
    message: {
      fr: "Le téléversement des documents joints aux propositions est interrompu depuis 09:15 UTC. Les dossiers restent enregistrables ; les pièces jointes pourront être ajoutées après rétablissement.",
      en: 'Uploading documents attached to proposals has been interrupted since 09:15 UTC. Proposals can still be saved; attachments can be added once service is restored.',
    },
    is_dismissible: false,
    display_from: '2026-08-17T09:20:00Z',
    display_until: null,
    published_at: '2026-08-17T09:22:00Z',
    published_by: PERSON.bakayoko,
    created_at: '2026-08-17T09:18:00Z',
    updated_at: '2026-08-17T09:22:00Z',
  },
  {
    ...base,
    id: INCIDENT.prolongationAppel,
    scope: 'event',
    event_id: EVENT.cop31,
    incident_kind_code: 'information',
    severity: 'info',
    title: { fr: "Appel à propositions prolongé", en: 'Call for proposals extended' },
    message: {
      fr: "L'appel à propositions de la COP31 est prolongé jusqu'au 30 septembre 2026 à 23:59, heure de Belém.",
      en: 'The COP31 call for proposals has been extended to 30 September 2026, 23:59 Belém time.',
    },
    is_dismissible: true,
    display_from: '2026-08-12T12:00:00Z',
    display_until: '2026-10-01T02:59:59Z',
    published_at: '2026-08-12T12:05:00Z',
    published_by: PERSON.tremblay,
    created_at: '2026-08-12T11:40:00Z',
    updated_at: '2026-08-12T12:05:00Z',
  },
  {
    // PUBLIÉ, MAIS SA FENÊTRE EST CLOSE. Il ne s'affiche plus, et c'est le
    // `display_until` qui l'a retiré tout seul — personne n'a eu à y penser.
    // C'est la correction de la v1, où les bandeaux restaient en ligne des mois.
    ...base,
    id: INCIDENT.incidentClos,
    scope: 'event',
    event_id: EVENT.cop31,
    incident_kind_code: 'connection_issue',
    severity: 'error',
    title: { fr: 'Lenteurs sur le formulaire de dépôt', en: 'Submission form slowdowns' },
    message: {
      fr: 'Des lenteurs ont affecté le formulaire de dépôt le 3 août entre 14:00 et 16:30 UTC. Le service est rétabli.',
      en: 'The submission form experienced slowdowns on 3 August between 14:00 and 16:30 UTC. Service has been restored.',
    },
    is_dismissible: true,
    display_from: '2026-08-03T14:10:00Z',
    display_until: '2026-08-05T00:00:00Z',
    published_at: '2026-08-03T14:12:00Z',
    published_by: PERSON.bakayoko,
    created_at: '2026-08-03T14:05:00Z',
    updated_at: '2026-08-05T08:00:00Z',
  },
  {
    // RÉDIGÉ, JAMAIS PUBLIÉ. La publication est une décision tracée
    // (`live.publish_incident`), pas un effet de bord de l'enregistrement.
    ...base,
    id: INCIDENT.brouillonNonPublie,
    scope: 'event',
    event_id: EVENT.cop31,
    incident_kind_code: 'schedule_change',
    severity: 'warning',
    title: { fr: 'Décalage possible de la séance d’ouverture', en: 'Possible opening session shift' },
    message: {
      fr: "Un décalage d'une heure de la séance d'ouverture est à l'étude. Message à publier une fois la décision prise.",
      en: 'A one-hour shift of the opening session is under consideration. To be published once decided.',
    },
    is_dismissible: true,
    display_from: '2026-08-16T08:00:00Z',
    display_until: null,
    published_at: null,
    published_by: null,
    created_at: '2026-08-16T07:55:00Z',
    updated_at: '2026-08-16T07:55:00Z',
  },
] satisfies Incident[]

/**
 * Équivalent de `live.active_incidents_for_event(event_id, at)`, ajoutée au
 * modèle le 17/08.
 *
 * ELLE DESCEND LA HIÉRARCHIE — édition, ses journées, ses séances, les
 * organisations qui y animent — là où `live.active_incidents(session)` la
 * remonte. Les quatre conditions d'affichage sont reproduites dans le même
 * ordre que la fonction SQL ; l'ordre de tri aussi, gravité décroissante puis
 * date d'affichage décroissante.
 */
const SEVERITY_ORDER = { info: 0, warning: 1, error: 2, critical: 3 } as const

export function activeIncidentsForEvent(eventId: string, at: number = Date.now()): EventIncident[] {
  return incidents
    .filter((incident) => {
      if (incident.published_at === null || incident.unpublished_at !== null) return false
      if (Date.parse(incident.display_from) > at) return false
      if (incident.display_until !== null && Date.parse(incident.display_until) <= at) return false
      // Le jeu ne porte pas d'incident de portée `event_day`, `session` ni
      // `organization` : les trois cas sont couverts par la fonction SQL, dont
      // le comportement a été vérifié en base le 17/08.
      return incident.scope === 'global' || incident.event_id === eventId
    })
    .sort(
      (a, b) =>
        SEVERITY_ORDER[b.severity] - SEVERITY_ORDER[a.severity] ||
        b.display_from.localeCompare(a.display_from),
    )
    .map((incident) => ({
      incident_id: incident.id,
      scope: incident.scope,
      severity: incident.severity,
      kind_code: incident.incident_kind_code,
      title: incident.title,
      message: incident.message,
      // La fonction SQL résout la cible pour que le back-office affiche
      // « Séance d'ouverture » et non un identifiant. À portée globale, il n'y
      // en a aucune : le message couvre toute la plateforme.
      target_label:
        incident.scope === 'event'
          ? (events.find((e) => e.id === incident.event_id)?.title.fr ?? null)
          : null,
      display_from: incident.display_from,
      display_until: incident.display_until,
    }))
}
