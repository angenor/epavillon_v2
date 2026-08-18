/**
 * Données simulées de `live.incidents` — 080_live.sql § 5.
 *
 * SEPT MESSAGES, ET CINQ D'ENTRE EUX NE DOIVENT PAS S'AFFICHER AUJOURD'HUI.
 * C'est tout l'intérêt du jeu : un incident ne se lit pas à sa seule existence
 * mais à quatre conditions cumulées, qu'une implémentation naïve oublie une par
 * une — publié, non dépublié, fenêtre d'affichage ouverte, portée concernée. Le
 * jeu porte donc les CINQ états que `live.event_incidents()` distingue : deux
 * actifs, deux programmés, un rédigé jamais publié, un expiré tout seul, un
 * dépublié à la main avec son motif. Et les CINQ portées de
 * `live.incident_scope`, la contrainte `ck_incidents_scope_target` étant l'autre
 * piège de cette table.
 *
 * LE MODULE DIRECT EST HORS JALON, mais le bandeau d'incident est transverse —
 * même raison que pour `types/live.ts`. Ces messages alimentent le bloc
 * d'actions du tableau de bord (A6) et l'écran des messages d'incident (A13).
 *
 * CE FICHIER NE PORTE QUE LA DONNÉE. Les compositions d'écran — état calculé,
 * balayage de portée, cible résolue, écritures de la session — vivent dans
 * `admin-incidents.ts` : elles appartiennent à l'écran, pas à la table.
 *
 * LE TEXTE EST UNE DONNÉE, PAS UNE TRADUCTION. `title` et `message` sont des
 * `platform.i18n_text` : un administrateur les écrit depuis le back-office, ils
 * n'ont donc rien à faire dans un fichier i18n.
 */

import type { Incident } from '~/types/live'
import { EVENT, EVENT_DAY, INCIDENT, ORG, PERSON, SESSION } from './ids'

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
    // PORTÉE PLATEFORME : c'est la visionneuse elle-même qui est en panne, donc
    // toutes les diffusions, de toutes les éditions. Le seul cas qui justifie
    // vraiment cette portée — un incident de diffusion se dit d'ordinaire sur
    // l'activité concernée.
    id: INCIDENT.visionneusePanne,
    scope: 'global',
    incident_kind_code: 'technical_issue',
    severity: 'warning',
    title: {
      fr: 'Lecteur vidéo indisponible',
      en: 'Video player unavailable',
    },
    message: {
      fr: "Le lecteur vidéo intégré est indisponible depuis 09:15 UTC : les diffusions en direct ne s'affichent pas sur la plateforme. Elles restent suivables sur la chaîne YouTube de l'IFDD.",
      en: 'The embedded video player has been unavailable since 09:15 UTC: live broadcasts do not display on the platform. They can still be followed on the IFDD YouTube channel.',
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
    // PORTÉE ÉDITION : la régie du pavillon est en panne, donc TOUTES les
    // activités de cette COP, pas une seule.
    id: INCIDENT.directPavillonCoupe,
    scope: 'event',
    event_id: EVENT.cop31,
    incident_kind_code: 'connection_issue',
    severity: 'error',
    title: { fr: 'Direct du pavillon interrompu', en: 'Pavilion live stream interrupted' },
    message: {
      fr: "La diffusion en direct du pavillon est interrompue : la liaison satellite du site est coupée. Les activités se tiennent normalement sur place et les enregistrements seront mis en ligne dès le rétablissement.",
      en: 'The pavilion live stream is interrupted: the venue satellite link is down. Activities are taking place on site as planned, and recordings will be published once service is restored.',
    },
    is_dismissible: false,
    display_from: '2026-08-18T11:30:00Z',
    display_until: null,
    published_at: '2026-08-18T11:32:00Z',
    published_by: PERSON.tremblay,
    created_at: '2026-08-18T11:30:00Z',
    updated_at: '2026-08-18T11:32:00Z',
  },
  {
    // PUBLIÉ, MAIS SA FENÊTRE EST CLOSE. Il ne s'affiche plus, et c'est le
    // `display_until` qui l'a retiré tout seul — personne n'a eu à y penser.
    // C'est la correction de la v1, où les bandeaux restaient en ligne des mois.
    ...base,
    id: INCIDENT.sonCoupePleniere,
    scope: 'event',
    event_id: EVENT.cop31,
    incident_kind_code: 'technical_issue',
    severity: 'error',
    title: { fr: 'Son coupé sur la diffusion', en: 'Sound cut on the broadcast' },
    message: {
      fr: "Le son de la diffusion a été coupé le 3 août entre 14:00 et 16:30 UTC. Le direct est rétabli et l'enregistrement corrigé sera mis en ligne.",
      en: 'Broadcast sound was cut on 3 August between 14:00 and 16:30 UTC. The live stream is restored and a corrected recording will be published.',
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
  {
    // PORTÉE SÉANCE, ET LE CAS QUE LE COMMANDITAIRE NOMME EN PREMIER : une
    // activité qui déborde sur la suivante. Sa fenêtre est celle du créneau
    // concerné, en novembre 2027 : aujourd'hui, l'écran le range donc en
    // « programmé » — un message d'incident se date de l'incident, pas de sa
    // saisie.
    ...base,
    id: INCIDENT.debordementAtelier,
    scope: 'session',
    session_id: SESSION.atelierNegociation1,
    incident_kind_code: 'overrun',
    severity: 'warning',
    title: { fr: 'Atelier prolongé de 15 minutes', en: 'Workshop running 15 minutes late' },
    message: {
      fr: "L'atelier de négociation se prolonge de quinze minutes. L'activité suivante débutera à 11:45, heure de Belém.",
      en: 'The negotiation workshop is running fifteen minutes over. The next activity will start at 11:45 Belém time.',
    },
    is_dismissible: true,
    // La fenêtre est celle du créneau débordé : l'atelier finit à 11:30 heure de
    // Belém (14:30 UTC), le message paraît juste avant et s'efface tout seul un
    // quart d'heure après. Un incident se date de l'incident, pas de sa saisie.
    display_from: '2027-11-13T14:25:00Z',
    display_until: '2027-11-13T15:00:00Z',
    published_at: '2027-11-13T14:26:00Z',
    published_by: PERSON.nkoDiop,
    created_at: '2027-11-13T14:25:00Z',
    updated_at: '2027-11-13T14:26:00Z',
  },
  {
    // PORTÉE JOURNÉE : tout ce qui se tient ce jour-là change de salle.
    ...base,
    id: INCIDENT.changementSalleNov12,
    scope: 'event_day',
    event_day_id: EVENT_DAY.nov12,
    incident_kind_code: 'room_change',
    severity: 'info',
    title: { fr: 'Activités déplacées en salle Amazonie', en: 'Activities moved to Amazonia room' },
    message: {
      fr: "Les activités du 12 novembre se tiennent en salle Amazonie : le stand est mobilisé par la journée finance durable.",
      en: 'Activities on 12 November take place in the Amazonia room: the stand hosts the sustainable finance day.',
    },
    is_dismissible: true,
    display_from: '2027-11-11T12:00:00Z',
    display_until: '2027-11-13T03:00:00Z',
    published_at: '2027-11-11T12:02:00Z',
    published_by: PERSON.perretAdmin,
    created_at: '2027-11-11T11:50:00Z',
    updated_at: '2027-11-11T12:02:00Z',
  },
  {
    // PUBLIÉ PUIS DÉPUBLIÉ À LA MAIN, AVEC MOTIF. C'est la dépublication en un
    // clic de l'écran A13 : la ligne reste, elle ne disparaît pas de
    // l'historique — même principe qu'une attribution de rôle révoquée.
    ...base,
    id: INCIDENT.panneVisioOrg,
    scope: 'organization',
    organization_id: ORG.roac,
    incident_kind_code: 'connection_issue',
    severity: 'error',
    title: { fr: 'Diffusion interrompue', en: 'Broadcast interrupted' },
    message: {
      fr: "La diffusion des activités du Réseau ouest-africain pour l'adaptation côtière est interrompue. L'enregistrement sera mis en ligne dès que possible.",
      en: 'The broadcast of the West African Coastal Adaptation Network activities is interrupted. The recording will be published as soon as possible.',
    },
    is_dismissible: false,
    display_from: '2026-08-14T13:00:00Z',
    display_until: null,
    published_at: '2026-08-14T13:04:00Z',
    published_by: PERSON.bakayoko,
    unpublished_at: '2026-08-14T15:20:00Z',
    unpublished_by: PERSON.bakayoko,
    unpublish_reason: 'Diffusion rétablie à 15:15 UTC.',
    created_at: '2026-08-14T13:00:00Z',
    updated_at: '2026-08-14T15:20:00Z',
  },
] satisfies Incident[]
