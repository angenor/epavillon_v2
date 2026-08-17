/**
 * Déclaration minimale pour `vue-cal` (v4), qui n'embarque pas de typages.
 *
 * Le paquet est retenu par le prompt A3 — « en réutilisant vue-cal comme le
 * planificateur A9 » : une seule bibliothèque de calendrier pour la
 * programmation publique et pour l'arbitrage du back-office, sans quoi les deux
 * écrans afficheraient les mêmes créneaux de deux façons différentes.
 *
 * On ne décrit ici QUE ce que la plateforme utilise. Élargir cette déclaration
 * à toute la surface de la bibliothèque donnerait une fausse assurance : ces
 * types ne sont pas vérifiés contre son code, ils sont écrits à la main.
 */
declare module 'vue-cal' {
  import type { DefineComponent } from 'vue'

  /**
   * Un bloc du calendrier. `start` et `end` sont des heures MURALES
   * (`AAAA-MM-JJ HH:MM`) : vue-cal ne connaît pas les fuseaux, la conversion
   * vers celui de l'édition est faite en amont par `wallClockInZone()`.
   */
  export interface VueCalEvent {
    start: string
    end: string
    title?: string
    content?: string
    class?: string
    background?: boolean
    allDay?: boolean
    /** Charge utile libre — la plateforme y range l'identifiant de la séance. */
    [key: string]: unknown
  }

  const VueCal: DefineComponent<Record<string, unknown>>
  export default VueCal
}

declare module 'vue-cal/dist/vuecal.css'
