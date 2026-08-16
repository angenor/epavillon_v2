import type { I18nText, LocaleCode } from '~/types/shared'

/**
 * Résolution d'un texte multilingue venu de la BASE — colonne du domaine
 * `platform.i18n_text`. Transposition fidèle de `platform.t()`
 * (docs/database/000_bootstrap.sql § 5.1).
 *
 * Ordre de repli, identique côté SQL et côté interface :
 *   1. la locale demandée telle quelle          (`fr-CA`)
 *   2. sa langue de base                        (`fr`)
 *   3. le français, langue pivot de la plateforme
 *   4. la première valeur non vide disponible
 *   5. la chaîne vide — jamais `undefined`, jamais une clé brute affichée
 *
 * Ne JAMAIS lire `value.fr` directement dans un composant : le repli est une
 * règle, pas une commodité.
 */
export function resolveI18nText(
  value: I18nText | null | undefined,
  locale: LocaleCode | string = 'fr',
): string {
  if (!value) return ''

  const exact = value[locale]
  if (isFilled(exact)) return exact

  const base = locale.split('-')[0]
  if (base && base !== locale) {
    const baseValue = value[base]
    if (isFilled(baseValue)) return baseValue
  }

  if (isFilled(value.fr)) return value.fr

  for (const candidate of Object.values(value)) {
    if (isFilled(candidate)) return candidate
  }

  return ''
}

/**
 * Indique si une traduction existe réellement dans la locale demandée — utile
 * pour signaler « traduction manquante » dans les écrans d'administration
 * plutôt que d'afficher silencieusement le repli français.
 */
export function hasI18nTranslation(
  value: I18nText | null | undefined,
  locale: LocaleCode | string,
): boolean {
  if (!value) return false
  if (isFilled(value[locale])) return true
  const base = locale.split('-')[0]
  return base !== undefined && base !== locale && isFilled(value[base])
}

/**
 * Liste des locales réellement renseignées, dans l'ordre où elles apparaissent.
 */
export function filledI18nLocales(value: I18nText | null | undefined): string[] {
  if (!value) return []
  return Object.entries(value)
    .filter(([, text]) => isFilled(text))
    .map(([locale]) => locale)
}

function isFilled(value: string | undefined): value is string {
  return typeof value === 'string' && value.trim().length > 0
}
