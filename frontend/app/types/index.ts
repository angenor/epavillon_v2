/**
 * Point d'entrée des types du modèle. Ne contient AUCUNE définition : il ne fait
 * que ré-exporter.
 *
 * Pour un écran donné, importer directement le fichier concerné plutôt que ce
 * point d'entrée — `import type { Proposal } from '~/types/programme/proposal'`.
 * On sait alors, à la lecture des imports, quelles tables l'écran touche.
 *
 * Correspondance fichier ↔ source SQL (voir `docs/MODELE_INDEX.md`) :
 *
 *   shared.ts                  000_bootstrap.sql · 010_platform.sql
 *   reference.ts               020_reference.sql
 *   identity.ts                030_identity.sql
 *   org.ts                     040_organizations.sql
 *   media.ts                   050_media.sql
 *   event/series.ts            060_events.sql § 1
 *   event/edition.ts           060_events.sql § 2, 3, 3 bis
 *   event/venue.ts             060_events.sql § 4, 4 bis
 *   event/call.ts              060_events.sql § 5, 6, 7
 *   programme/proposal.ts      070_programme_proposals.sql
 *   programme/review.ts        070_programme_proposals.sql § 5
 *   programme/session.ts       075_programme_sessions.sql § 1, 2, 7
 *   programme/registration.ts  075_programme_sessions.sql § 3, 4, 5
 *   engagement.ts              110_engagement.sql § 6 — les rappels seuls
 *   live.ts                    080_live.sql § 5 — les messages d'incident seuls
 *   analytics.ts               130_analytics.sql — les projections du tableau de bord
 *   views.ts                   les deux vues consommées telles quelles
 *
 * Modules hors du jalon en cours — Publications, Négociations, Formations,
 * Outils — : leurs types viendront avec leurs écrans, dans leur propre fichier.
 * De trois modules, seule la part réellement consommée par le jalon est
 * couverte : du Direct, le bandeau d'incident, motif transverse de toute la
 * plateforme (voir l'en-tête de `live.ts`) ; de l'Engagement, le calendrier des
 * rappels d'une séance, que l'espace organisation rend ; de l'Analytique, les
 * projections que le tableau de bord du back-office consomme (A6) — la mesure
 * d'audience et les fiches de performance viendront avec les leurs.
 *
 * `navigation.ts`, `ui.ts`, `auth.ts`, `organization-join.ts`,
 * `organization-workspace.ts`, `proposal-form.ts`, `event-programme.ts` et
 * `admin-dashboard.ts` ne
 * sont PAS ré-exportés : ils ne décrivent pas le modèle. Le premier porte une
 * augmentation de `PageMeta` que les layouts importent directement, le deuxième
 * le vocabulaire des composants d'interface, les autres les contrats d'un écran
 * — authentification (A1), rattachement (A2), page publique d'une édition (A3),
 * formulaire de dépôt (A4), espace organisation (A5), tableau de bord du
 * back-office (A6) : requêtes, réponses et compositions, sans aucune table.
 */

export type * from './shared'
export type * from './reference'
export type * from './identity'
export type * from './org'
export type * from './media'

export type * from './event/series'
export type * from './event/edition'
export type * from './event/venue'
export type * from './event/call'

export type * from './programme/proposal'
export type * from './programme/review'
export type * from './programme/session'
export type * from './programme/registration'

export type * from './engagement'
export type * from './live'
export type * from './analytics'
export type * from './views'
