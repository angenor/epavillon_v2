/** Types de navigation partagés par les deux layouts. */

import type { PermissionCode } from './shared'

export interface NavItem {
  /** Clé i18n du libellé — jamais le libellé lui-même. */
  labelKey: string
  /** Chemin sans préfixe de langue : `localePath()` s'en charge. */
  to: string
  /** Fonctionnalité pilotée par `platform.feature_flags` ; masquée tant qu'elle est absente. */
  featureFlag?: string
  /** Nom d'icône de `UiIcon`, pour la navigation latérale. Facultatif : une
   *  section peut n'en porter aucune sans décaler ses libellés. */
  icon?: string
  /** Compteur poussé à droite de l'entrée — ce qui attend d'être traité. Il vient
   *  toujours d'un décompte de l'API, jamais d'une valeur écrite en dur. */
  count?: number
  /**
   * L'une de ces permissions, **sur la portée globale**, fait paraître l'entrée.
   * Absente, l'entrée paraît à tout administrateur : le périmètre est celui de
   * l'édition choisie, que chaque écran fait respecter. Lue une fois par le
   * layout, pour ne pas montrer ce qu'un écran refuserait (SC-008) ; l'API
   * garde ses routes de toute façon.
   */
  permissions?: PermissionCode[]
}

export interface NavSection {
  labelKey: string
  /** Une section dont aucune entrée ne paraît disparaît. */
  items: NavItem[]
}

export interface BreadcrumbItem {
  /** Clé i18n du libellé. */
  labelKey?: string
  /** Libellé déjà résolu — pour un titre venu de la base (`platform.i18n_text`). */
  label?: string
  /** Chemin sans préfixe de langue ; absent pour le dernier maillon. */
  to?: string
}

declare module '#app' {
  interface PageMeta {
    /**
     * Fil d'Ariane du back-office, déclaré par la page dans `definePageMeta`.
     * Le layout n'invente rien : une page sans fil d'Ariane n'en affiche pas.
     */
    breadcrumb?: BreadcrumbItem[]
    /**
     * Pourquoi cette page exige un rattachement à une organisation — clé d'un
     * message de `organization.join.required.reasons`, lue par le middleware
     * `requires-organization`. Sans elle, l'étape intermédiaire s'affiche avec
     * un bandeau générique : elle dit alors qu'une organisation est nécessaire,
     * mais pas pour quoi faire, ce qui est le minimum utile et pas davantage.
     */
    organizationReason?: string
  }
}

// Nuxt reporte `PageMeta` sur `RouteMeta` de vue-router dans ses types générés :
// l'augmentation ci-dessus suffit à typer `route.meta.breadcrumb`.
export {}
