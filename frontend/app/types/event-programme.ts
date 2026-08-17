/**
 * Contrats de la PAGE PUBLIQUE D'UNE ÉDITION (écran A3).
 *
 * Ces types ne décrivent AUCUNE table : ce sont les compositions dont l'écran a
 * besoin, au même titre que `types/organization-join.ts` pour l'écran A2. Tout
 * ce qui vient du modèle est importé tel quel — `PublicScheduleRow`, `EventDay`,
 * `Room` — et jamais recopié : une ligne de programmation reste une ligne de
 * `programme.v_public_schedule`, colonnes comprises.
 *
 * POURQUOI CE FICHIER EXISTE. La section « Programmation » charge trois choses
 * ensemble et doit pouvoir les recharger ensemble quand on change d'année. Les
 * nommer une fois évite qu'un composant en oublie une, et rend visible ce que
 * l'API devra rendre en une réponse le jour venu.
 */

import type { EventDay } from './event/edition'
import type { Room } from './event/venue'
import type { EventId, IsoDate, TaxonomyTermCode } from './shared'
import type { PublicScheduleRow } from './views'

/** Ce qu'il faut charger pour afficher le programme d'une édition. */
export interface ProgrammeData {
  /** Lignes de `programme.v_public_schedule`, séances publiées uniquement. */
  schedule: PublicScheduleRow[]
  /** `event.event_days` — le calendrier. Vide pour un cycle de webinaires. */
  days: EventDay[]
  /** `event.rooms` de l'édition, pour le filtre par salle. */
  rooms: Room[]
}

/**
 * Filtres de la section, PARTAGÉS par les deux vues. `null` signifie « tous » :
 * c'est ce qui permet de vider un filtre sans le distinguer d'une valeur.
 */
export interface ProgrammeFilterState {
  /** Jour civil dans le fuseau de l'édition (`AAAA-MM-JJ`). */
  day: IsoDate | null
  /** Code de `reference.taxonomy_terms`, taxonomie `activity_theme`. */
  theme: TaxonomyTermCode | null
  /** `event.participation_mode`. */
  format: string | null
  /** `event.rooms.id`. */
  room: string | null
}

/** Une journée de programmation, telle que la vue grille et les filtres la voient. */
export interface ProgrammeDay {
  /** Jour civil dans le fuseau de l'édition. */
  date: IsoDate
  /** `event.event_days.id` quand le jour existe au calendrier ; `null` sinon —
   *  c'est le cas d'un cycle de webinaires, qui n'a pas de calendrier. */
  dayId: string | null
  /** Séances de ce jour, déjà triées. */
  sessions: PublicScheduleRow[]
}

/**
 * Une entrée du sélecteur d'année. Les éditions de conférence sont rangées par
 * année ; ce qui n'en relève pas — webinaires, cycles — forme le groupe
 * « Autres », qui ne disparaît pas de la programmation publique pour autant.
 */
export interface ProgrammeEditionOption {
  id: EventId
  slug: string
  /** « COP31 », « PACO 2026 » : l'acronyme, à défaut le libellé d'édition. */
  label: string
  year: number
  /** Faux pour un cycle de webinaires ou un rendez-vous ponctuel. */
  isConference: boolean
  /** Le programme est-il publié ? Sinon, la section l'annonce et n'invente rien. */
  isPublished: boolean
}
