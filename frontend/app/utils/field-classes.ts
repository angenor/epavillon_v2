import type { Size } from '~/types/ui'

/**
 * Habillage commun des contrôles de saisie — `Input`, `Textarea`, `Select`,
 * `SearchInput`, `DatePicker`.
 *
 * Factorisé ici pour une raison précise : un champ en erreur et un champ ordinaire
 * doivent différer par la BORDURE ET par le message, jamais par la seule couleur.
 * Écrire cette règle cinq fois, c'est la voir se perdre au quatrième composant.
 *
 * Les états couverts, dans l'ordre où ils se rencontrent : vide et rempli
 * (identiques au trait près — c'est le `placeholder` qui les distingue), survol,
 * focus clavier, erreur, désactivé, lecture seule.
 */

export interface FieldControlOptions {
  hasError?: boolean
  disabled?: boolean
  readonly?: boolean
  size?: Size
  /** Espace réservé à gauche pour une icône (recherche, calendrier). */
  leadingIcon?: boolean
  /** Espace réservé à droite (bouton d'effacement, chevron, unité). */
  trailingIcon?: boolean
}

/** Hauteurs accordées à celles de `UiButton` : un champ et son bouton s'alignent. */
const SIZES: Record<Size, string> = {
  sm: 'h-8 text-sm',
  md: 'h-[2.375rem] text-sm',
  lg: 'h-11 text-base',
}

export function fieldControlClasses(options: FieldControlOptions = {}): string[] {
  const { hasError, disabled, readonly, size = 'md', leadingIcon, trailingIcon } = options

  return [
    'w-full rounded-md border bg-surface-raised text-text',
    'transition-colors duration-150',
    'placeholder:text-text-subtle',
    SIZES[size],
    leadingIcon ? 'pl-9' : 'pl-3',
    trailingIcon ? 'pr-9' : 'pr-3',

    // Le trait porte l'état. `--color-border-strong` tient 3,59:1 en thème clair
    // et 4,54:1 en thème sombre : un contour de champ doit rester visible.
    hasError ? 'border-danger' : 'border-border-strong',
    hasError ? 'hover:border-danger' : 'hover:border-text-subtle',

    // Le focus est celui de `main.css` — un seul anneau pour toute la
    // plateforme. On ajoute seulement une bordure accentuée, qui distingue le
    // champ focalisé de ses voisins une fois l'anneau posé.
    hasError ? 'focus:border-danger' : 'focus:border-accent',
    'focus:outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus',

    // Désactivé : hors du parcours, valeur non soumise.
    disabled ? 'cursor-not-allowed bg-surface-sunken text-text-subtle border-border' : '',

    // Lecture seule : focalisable et copiable, mais visiblement non modifiable.
    // Le fond en retrait le dit sans retirer le champ du parcours.
    readonly && !disabled ? 'bg-surface-sunken text-text-muted cursor-default' : '',
  ].filter(Boolean)
}

/** Position d'une icône posée à l'intérieur d'un contrôle. */
export const FIELD_ICON_CLASSES =
  'pointer-events-none absolute top-1/2 -translate-y-1/2 text-text-subtle'
