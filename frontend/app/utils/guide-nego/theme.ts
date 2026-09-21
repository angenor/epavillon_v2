/**
 * Le thème de Guide Négo — et lui seul.
 *
 * Il ne lit ni n'écrit le réglage du site : autre clé, autre élément, autre valeur.
 * Quelqu'un qui lit le site en clair et l'application en sombre, tard en salle, a
 * raison des deux côtés.
 */
export const CHOIX_DE_THEME = ['clair', 'sombre', 'systeme'] as const
export type ChoixDeTheme = (typeof CHOIX_DE_THEME)[number]
export type ThemeAffiche = 'clair' | 'sombre'

/** Par défaut, et devant toute valeur abîmée : le réglage du téléphone. */
export function lireChoix(valeur: string | null): ChoixDeTheme {
  return CHOIX_DE_THEME.includes(valeur as ChoixDeTheme) ? (valeur as ChoixDeTheme) : 'systeme'
}

export function themeAffiche(choix: ChoixDeTheme, telephoneEnSombre: boolean): ThemeAffiche {
  if (choix === 'systeme') return telephoneEnSombre ? 'sombre' : 'clair'
  return choix
}
