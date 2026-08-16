/** Types de navigation partagés par les deux layouts. */

export interface NavItem {
  /** Clé i18n du libellé — jamais le libellé lui-même. */
  labelKey: string
  /** Chemin sans préfixe de langue : `localePath()` s'en charge. */
  to: string
  /** Fonctionnalité pilotée par `platform.feature_flags` ; masquée tant qu'elle est absente. */
  featureFlag?: string
}

export interface NavSection {
  labelKey: string
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
  }
}

// Nuxt reporte `PageMeta` sur `RouteMeta` de vue-router dans ses types générés :
// l'augmentation ci-dessus suffit à typer `route.meta.breadcrumb`.
export {}
