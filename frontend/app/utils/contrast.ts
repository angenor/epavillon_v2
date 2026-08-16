/**
 * Contraste WCAG 2.1 — calculé, jamais recopié.
 *
 * Le guide de style affiche le rapport de contraste de chaque nuance de la
 * palette. Ces valeurs sont DÉJÀ annotées dans `design-tokens.css`, et les
 * recopier dans la page produirait deux vérités qui divergeraient à la première
 * retouche de palette. La page lit donc les variables CSS réellement appliquées
 * et calcule le rapport ici : elle reste vraie par construction, y compris en
 * thème sombre où les rôles changent de valeur.
 *
 * Formule : WCAG 2.1, § 1.4.3. Luminance relative sRGB linéarisée, rapport
 * `(L_clair + 0,05) / (L_sombre + 0,05)`, borné entre 1 et 21.
 */

/** Composantes 0–255. */
export interface Rgb {
  r: number
  g: number
  b: number
}

/** Verdict WCAG d'un rapport de contraste, pour une taille de texte donnée. */
export type ContrastVerdict = 'aaa' | 'aa' | 'aa-large' | 'fail'

/**
 * Analyse une couleur CSS. Accepte `#rgb`, `#rrggbb`, `rgb()` et `rgba()` — les
 * trois formes que renvoie `getComputedStyle` selon les navigateurs. Renvoie
 * `null` sur toute autre notation plutôt que d'inventer une valeur.
 */
export function parseColor(value: string): Rgb | null {
  const input = value.trim().toLowerCase()
  if (input === '') return null

  const hex = input.match(/^#([0-9a-f]{3}|[0-9a-f]{6})$/)
  if (hex?.[1]) {
    const digits = hex[1]
    const expanded =
      digits.length === 3
        ? digits
            .split('')
            .map((d) => d + d)
            .join('')
        : digits
    return {
      r: Number.parseInt(expanded.slice(0, 2), 16),
      g: Number.parseInt(expanded.slice(2, 4), 16),
      b: Number.parseInt(expanded.slice(4, 6), 16),
    }
  }

  const rgb = input.match(/^rgba?\(([^)]+)\)$/)
  if (rgb?.[1]) {
    const parts = rgb[1]
      .split(/[\s,/]+/)
      .filter(Boolean)
      .map((part) => Number.parseFloat(part))
    const [r, g, b] = parts
    if (r === undefined || g === undefined || b === undefined) return null
    if (![r, g, b].every(Number.isFinite)) return null
    return { r, g, b }
  }

  return null
}

/** Luminance relative sRGB — WCAG 2.1 § 1.4.3. */
export function relativeLuminance({ r, g, b }: Rgb): number {
  const channel = (value: number): number => {
    const normalized = value / 255
    return normalized <= 0.03928 ? normalized / 12.92 : ((normalized + 0.055) / 1.055) ** 2.4
  }
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

/** Rapport de contraste entre deux couleurs CSS : de 1 (identiques) à 21. */
export function contrastRatio(foreground: string, background: string): number | null {
  const front = parseColor(foreground)
  const back = parseColor(background)
  if (!front || !back) return null

  const a = relativeLuminance(front)
  const b = relativeLuminance(back)
  const lighter = Math.max(a, b)
  const darker = Math.min(a, b)
  return (lighter + 0.05) / (darker + 0.05)
}

/**
 * Verdict WCAG AA/AAA.
 * `largeText` : texte d'au moins 24 px, ou 18,66 px en gras — c'est aussi le
 * seuil retenu pour les bordures et les icônes (3:1).
 */
export function contrastVerdict(ratio: number | null, largeText = false): ContrastVerdict {
  if (ratio === null) return 'fail'
  if (largeText) {
    if (ratio >= 4.5) return 'aaa'
    if (ratio >= 3) return 'aa'
    return 'fail'
  }
  if (ratio >= 7) return 'aaa'
  if (ratio >= 4.5) return 'aa'
  if (ratio >= 3) return 'aa-large'
  return 'fail'
}

/** « 6,23:1 » — deux décimales, virgule décimale française via `Intl`. */
export function formatRatio(ratio: number | null, locale = 'fr-FR'): string {
  if (ratio === null) return '—'
  return `${new Intl.NumberFormat(locale, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(ratio)}:1`
}

/**
 * Valeur effective d'une variable CSS sur un élément — celle que le navigateur
 * applique VRAIMENT, thème actif compris. Sans DOM (rendu serveur), renvoie
 * une chaîne vide : l'appelant affiche alors son état de chargement.
 */
export function cssVariableValue(name: string, element?: Element | null): string {
  if (typeof window === 'undefined' || typeof document === 'undefined') return ''
  const target = element ?? document.documentElement
  return getComputedStyle(target).getPropertyValue(name).trim()
}
