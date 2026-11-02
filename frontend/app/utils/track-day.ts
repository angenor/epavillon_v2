/**
 * La couleur d'une journée spéciale (`programme_tracks.color_hex`, saisie au
 * back-office) déclinée en rôles, comme les autres couleurs : `--color-day`
 * (texte, grands numéros), `-line` (filets), `-surface` (fond teinté),
 * `-surface-strong` (une ligne sur deux du cadre de `/programmations`),
 * `-surface-hover` (le cadre survolé), `-solid` et `-mark` (aplats) avec leur
 * texte.
 *
 * Une teinte saisie n'a aucune garantie de contraste : le texte posé sur un
 * aplat est choisi d'après sa luminance, et le texte coloré est mêlé à celui
 * du thème, ce qui le fonce en clair et l'éclaircit en sombre.
 *
 * Sans couleur (l'atelier préparatoire), le bleu nuit institutionnel seul.
 */
export function trackDayStyle(colorHex: string | null): Record<string, string> {
  const rgb = colorHex ? parseColor(colorHex) : null
  if (!colorHex || !rgb) {
    return {
      '--color-day': 'var(--color-heading)',
      '--color-day-line': 'var(--color-heading)',
      '--color-day-surface': 'var(--color-surface-sunken)',
      '--color-day-surface-strong': 'color-mix(in srgb, var(--color-text) 6%, var(--color-surface-sunken))',
      '--color-day-surface-hover': 'color-mix(in srgb, var(--color-text) 3%, var(--color-surface-sunken))',
      '--color-day-solid': 'var(--color-surface-inverse)',
      '--color-day-on-solid': 'var(--color-text-on-inverse)',
      '--color-day-mark': 'var(--color-surface-inverse-selected)',
      '--color-day-on-mark': 'var(--color-text-on-inverse-selected)',
    }
  }
  // Au-delà de 0,18, un texte bleu nuit contraste mieux que le blanc (cyan et vert de la charte).
  const onColor = relativeLuminance(rgb) > 0.18 ? 'var(--color-text-on-inverse-selected)' : 'var(--color-text-on-inverse)'
  return {
    '--color-day': `color-mix(in srgb, ${colorHex} 55%, var(--color-text))`,
    '--color-day-line': colorHex,
    '--color-day-surface': `color-mix(in srgb, ${colorHex} 12%, var(--color-surface-raised))`,
    '--color-day-surface-strong': `color-mix(in srgb, ${colorHex} 20%, var(--color-surface-raised))`,
    '--color-day-surface-hover': `color-mix(in srgb, ${colorHex} 16%, var(--color-surface-raised))`,
    '--color-day-solid': colorHex,
    '--color-day-on-solid': onColor,
    '--color-day-mark': colorHex,
    '--color-day-on-mark': onColor,
  }
}

/**
 * Le papier d'un marque-page du panneau « À venir » (accueil) : la couleur de la
 * journée éclaircie de blanc, l'encre foncée de bleu nuit. Le panneau est un
 * aplat sombre dans les deux thèmes : le papier ne s'inverse pas. Sans journée,
 * papier blanc (le jaune dirait « en cours »).
 */
export function trackPaperStyle(colorHex: string | null): Record<string, string> {
  const paper = 'var(--color-surface-inverse-selected)'
  const ink = 'var(--color-text-on-inverse-selected)'
  if (!colorHex || !parseColor(colorHex)) return { '--paper': paper, '--paper-ink': ink }
  return {
    '--paper': `color-mix(in srgb, ${colorHex} 30%, ${paper})`,
    '--paper-ink': `color-mix(in srgb, ${colorHex} 45%, ${ink})`,
  }
}

/**
 * Le trait d'un pictogramme de fil sur son rond blanc (`TrackIconBadge`) : la
 * couleur du fil foncée de bleu nuit, pour tenir 3:1 sur blanc même sur un vert
 * clair saisi. Le rond ne s'inverse pas en sombre, le trait non plus.
 */
export function trackGlyphColor(colorHex: string | null): string {
  return colorHex && parseColor(colorHex)
    ? `color-mix(in srgb, ${colorHex} 70%, var(--color-text-on-inverse-selected))`
    : 'var(--color-text-on-inverse-selected)'
}

/**
 * L'ancre d'une activité dans le déroulé d'une journée (`TrackAgenda`) : un
 * lien de `/programmations` y descend, et l'activité s'y encadre un instant.
 */
export const trackActivityAnchor = (sessionSlug: string): string => `activite-${sessionSlug}`
