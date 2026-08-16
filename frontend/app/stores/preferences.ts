import { defineStore } from 'pinia'

export type ThemeChoice = 'system' | 'light' | 'dark'

const THEME_COOKIE = 'epavillon_theme'
const ONE_YEAR = 60 * 60 * 24 * 365

/**
 * Préférences d'affichage de la personne connectée ou non.
 *
 * Le choix de thème est porté par un cookie plutôt que par `localStorage` :
 * c'est le seul moyen que le serveur pose déjà le bon `data-theme` sur `<html>`
 * lors du rendu, donc qu'aucune page ne s'affiche une fraction de seconde en
 * clair avant de basculer en sombre.
 *
 * « système » est volontairement le défaut : la plateforme suit la préférence
 * du navigateur tant que rien n'a été choisi explicitement. C'est ce que
 * traduisent les deux gardes CSS de `design-tokens.css`.
 */
export const usePreferencesStore = defineStore('preferences', () => {
  const cookie = useCookie<ThemeChoice>(THEME_COOKIE, {
    default: () => 'system',
    maxAge: ONE_YEAR,
    sameSite: 'lax',
    path: '/',
  })

  const theme = computed<ThemeChoice>(() => cookie.value ?? 'system')

  /**
   * Valeur de l'attribut `data-theme` du `<html>`. `null` en mode système :
   * l'absence d'attribut laisse la media query `prefers-color-scheme` décider.
   */
  const themeAttribute = computed<'light' | 'dark' | null>(() =>
    theme.value === 'system' ? null : theme.value,
  )

  function setTheme(choice: ThemeChoice): void {
    cookie.value = choice
  }

  /** Enchaînement de la bascule : système → clair → sombre → système. */
  function cycleTheme(): void {
    const order: ThemeChoice[] = ['system', 'light', 'dark']
    const index = order.indexOf(theme.value)
    setTheme(order[(index + 1) % order.length] ?? 'system')
  }

  return { theme, themeAttribute, setTheme, cycleTheme }
})
