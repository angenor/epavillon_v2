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
 *   views.ts                   les deux vues consommées telles quelles
 *
 * Modules hors du jalon en cours — Publications, Négociations, Formations,
 * Outils, Direct, Engagement, Analytique — : leurs types viendront avec leurs
 * écrans, dans leur propre fichier.
 *
 * `navigation.ts` n'est PAS ré-exporté : il ne décrit pas le modèle et porte une
 * augmentation de `PageMeta` que les layouts importent directement.
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

export type * from './views'
