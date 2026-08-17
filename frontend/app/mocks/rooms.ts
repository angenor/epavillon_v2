/**
 * Données simulées du schéma `event`, partie 2 — lieux, salles, canal de
 * diffusion.
 *
 * CES TROIS TABLES DONNENT UN SUJET AUX CONFLITS. Sans elles, le planificateur
 * (A9) saurait dire « deux activités à 14 h » mais pas « la salle Baobab est
 * réservée deux fois ». Deux règles matérielles s'y lisent directement :
 *
 *   - UN SEUL STAND. Le pavillon ne compte que deux salles physiques ; deux
 *     activités ne peuvent pas s'y tenir au même moment dans la même salle. La
 *     salle « Atelier en ligne » est virtuelle : elle accepte les créneaux
 *     simultanés, `detect_conflicts()` n'y voit pas de double réservation.
 *   - UN SEUL DIRECT. Un unique canal de diffusion pour toute l'édition : deux
 *     sessions diffusées sur des créneaux qui se recouvrent sont remontées en
 *     gravité `blocking`. Le cas est délibérément présent dans `mocks/sessions/`.
 *
 * Signaler n'est pas interdire : ces chevauchements s'écrivent sans obstacle.
 */

import type { BroadcastChannel, Room, Venue } from '~/types/event/venue'
import { CHANNEL, EVENT, ROOM, VENUE } from './ids'

// ---------------------------------------------------------------------------
// Lieux
// ---------------------------------------------------------------------------

export const venues = [
  {
    id: VENUE.pavillon,
    event_id: EVENT.cop31,
    name: { fr: 'Pavillon de la Francophonie', en: 'Francophonie Pavilion' },
    kind: 'pavilion',
    address: 'Zone bleue, hall B, emplacement B-214, parc des expositions du Hangar, Belém',
    map_url: 'https://www.ifdd.francophonie.org/cop31/plan-pavillon',
    created_at: '2026-02-10T10:10:00Z',
  },
  {
    id: VENUE.enLigne,
    event_id: EVENT.cop31,
    name: { fr: 'Diffusion en ligne', en: 'Online venue' },
    kind: 'virtual',
    address: null,
    map_url: null,
    created_at: '2026-02-10T10:12:00Z',
  },
  // Éditions passées et cycle hors COP (prompt A3) : une seule salle chacune,
  // ce qui suffit à leur programmation archivée. Le pavillon d'une COP terminée
  // n'a plus à être décrit dans le détail ; ce qui compte est que la séance
  // sache dire où elle s'est tenue.
  {
    id: VENUE.pavillonCop30,
    event_id: EVENT.cop30,
    name: { fr: 'Pavillon de la Francophonie', en: 'Francophonie Pavilion' },
    kind: 'pavilion',
    address: 'Zone bleue, parc du Hangar, Belém',
    map_url: null,
    created_at: '2025-03-11T10:10:00Z',
  },
  {
    id: VENUE.pavillonCop29,
    event_id: EVENT.cop29,
    name: { fr: 'Pavillon de la Francophonie', en: 'Francophonie Pavilion' },
    kind: 'pavilion',
    address: 'Zone bleue, stade olympique de Bakou',
    map_url: null,
    created_at: '2024-03-18T10:10:00Z',
  },
  {
    id: VENUE.pacoEnLigne,
    event_id: EVENT.paco2026,
    name: { fr: 'Salle de visioconférence', en: 'Video-conference room' },
    kind: 'virtual',
    address: null,
    map_url: null,
    created_at: '2025-12-08T14:05:00Z',
  },
] satisfies Venue[]

// ---------------------------------------------------------------------------
// Salles
// ---------------------------------------------------------------------------

export const rooms = [
  {
    id: ROOM.baobab,
    venue_id: VENUE.pavillon,
    name: { fr: 'Salle Baobab', en: 'Baobab Room' },
    code: 'baobab',
    capacity: 80,
    is_virtual: false,
    has_streaming: true,
    equipment: ['projecteur', 'sonorisation', 'cabine d’interprétation', 'caméra fixe'],
    sort_order: 10,
    created_at: '2026-02-10T10:15:00Z',
  },
  {
    id: ROOM.fromager,
    venue_id: VENUE.pavillon,
    name: { fr: 'Salle Fromager', en: 'Fromager Room' },
    code: 'fromager',
    capacity: 35,
    is_virtual: false,
    has_streaming: false,
    equipment: ['écran', 'tableau blanc'],
    sort_order: 20,
    created_at: '2026-02-10T10:16:00Z',
  },
  {
    // Salle VIRTUELLE : deux sessions peuvent l'occuper en même temps sans que
    // ce soit un conflit. C'est ce qui distingue une contrainte matérielle d'une
    // simple concomitance.
    id: ROOM.atelier,
    venue_id: VENUE.enLigne,
    name: { fr: 'Atelier en ligne', en: 'Online workshop room' },
    code: 'atelier-en-ligne',
    capacity: 300,
    is_virtual: true,
    has_streaming: true,
    equipment: ['visioconférence', 'sous-titrage automatique'],
    sort_order: 30,
    created_at: '2026-02-10T10:17:00Z',
  },
  {
    id: ROOM.cop30Principale,
    venue_id: VENUE.pavillonCop30,
    name: { fr: 'Salle principale', en: 'Main room' },
    code: 'principale',
    capacity: 90,
    is_virtual: false,
    has_streaming: true,
    equipment: ['projecteur', 'sonorisation', 'caméra fixe'],
    sort_order: 10,
    created_at: '2025-03-11T10:15:00Z',
  },
  {
    id: ROOM.cop29Principale,
    venue_id: VENUE.pavillonCop29,
    name: { fr: 'Salle principale', en: 'Main room' },
    code: 'principale',
    capacity: 70,
    is_virtual: false,
    has_streaming: true,
    equipment: ['projecteur', 'sonorisation'],
    sort_order: 10,
    created_at: '2024-03-18T10:15:00Z',
  },
  {
    id: ROOM.pacoVisio,
    venue_id: VENUE.pacoEnLigne,
    name: { fr: 'Visioconférence PACO', en: 'PACO video-conference' },
    code: 'paco-visio',
    capacity: 500,
    is_virtual: true,
    has_streaming: true,
    equipment: ['visioconférence', 'interprétation simultanée'],
    sort_order: 10,
    created_at: '2025-12-08T14:10:00Z',
  },
] satisfies Room[]

// ---------------------------------------------------------------------------
// Canal de diffusion
//
// Un seul canal pour l'édition, marqué par défaut : toute session dont
// `is_streamed` passe à vrai s'y voit rattachée par la base, sans saisie. C'est
// ce qui rend la règle « un seul direct à la fois » effective dès la première
// programmation, plutôt que dépendante de la vigilance d'un opérateur.
// ---------------------------------------------------------------------------

export const broadcastChannels = [
  {
    id: CHANNEL.cop31,
    event_id: EVENT.cop31,
    code: 'cop31_youtube',
    name: { fr: 'Chaîne COP31 de la Francophonie', en: 'Francophonie COP31 channel' },
    provider: 'youtube',
    channel_ref: '@ifddoif',
    locale: 'fr',
    is_default: true,
    is_active: true,
    created_at: '2026-02-10T10:20:00Z',
    updated_at: '2026-02-10T10:20:00Z',
  },
  // Un canal par édition, chacun par défaut sur la sienne : l'index unique
  // `ux_broadcast_channels_default` porte sur (event_id), pas sur la plateforme.
  {
    id: CHANNEL.cop30,
    event_id: EVENT.cop30,
    code: 'cop30_youtube',
    name: { fr: 'Chaîne COP30 de la Francophonie', en: 'Francophonie COP30 channel' },
    provider: 'youtube',
    channel_ref: '@ifddoif',
    locale: 'fr',
    is_default: true,
    is_active: true,
    created_at: '2025-03-11T10:20:00Z',
    updated_at: '2025-03-11T10:20:00Z',
  },
  {
    id: CHANNEL.cop29,
    event_id: EVENT.cop29,
    code: 'cop29_youtube',
    name: { fr: 'Chaîne COP29 de la Francophonie', en: 'Francophonie COP29 channel' },
    provider: 'youtube',
    channel_ref: '@ifddoif',
    locale: 'fr',
    is_default: true,
    is_active: true,
    created_at: '2024-03-18T10:20:00Z',
    updated_at: '2024-03-18T10:20:00Z',
  },
  {
    id: CHANNEL.paco,
    event_id: EVENT.paco2026,
    code: 'paco_youtube',
    name: { fr: 'Chaîne PACO', en: 'PACO channel' },
    provider: 'youtube',
    channel_ref: '@ifddoif',
    locale: 'fr',
    is_default: true,
    is_active: true,
    created_at: '2025-12-08T14:15:00Z',
    updated_at: '2025-12-08T14:15:00Z',
  },
] satisfies BroadcastChannel[]
