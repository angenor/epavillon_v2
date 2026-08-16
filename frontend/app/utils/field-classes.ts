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
 *
 * TROIS SIGNAUX, TROIS PROPRIÉTÉS DE TRAIT — le guide de style ne joue pas la
 * couleur seule, il fait porter chaque état par une propriété différente :
 * · l'ÉPAISSEUR dit l'erreur      — 2 px (`--border-medium`) contre 1 px ;
 * · le STYLE dit la lecture seule — pointillés contre trait plein, ce qui la
 *   distingue à l'œil du désactivé, qui garde un trait plein plus pâle ;
 * · la COULEUR dit le focus       — bordure `--color-focus` et halo de 3 px.
 * Une personne qui ne distingue pas les couleurs lit encore les trois.
 *
 * UNE SEULE CLASSE PAR PROPRIÉTÉ. Deux utilitaires Tailwind qui visent la même
 * propriété (`border-border-strong` et `border-danger`, `border-solid` et
 * `border-dashed`) ne se départagent PAS par l'ordre d'écriture dans l'attribut
 * mais par leur ordre dans la feuille générée : les cumuler revient à tirer au
 * sort l'état affiché. D'où les branches exclusives ci-dessous, qui n'émettent
 * jamais qu'une couleur, qu'une épaisseur et qu'un style de trait.
 *
 * LE HALO PLUTÔT QUE L'ANNEAU SEUL. L'anneau de focus global de `main.css`
 * s'applique ici comme partout ailleurs — il n'est ni supprimé ni redéfini. Mais
 * sur un champ, la bordure change DÉJÀ de couleur au focus : l'anneau seul se
 * confond avec elle. `--shadow-focus` l'en détache, et c'est exactement l'usage
 * pour lequel ce jeton a été créé.
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

/**
 * Hauteurs accordées à celles de `UiButton` : un champ et son bouton s'alignent.
 * Une hauteur MINIMALE, pas fixe — un `textarea` et un champ dont le texte se
 * replie doivent pouvoir grandir sans que la règle saute. La cible tactile de
 * 44 px vaut pour `md` et `lg` ; `sm` descend à 40 px et ne se justifie que dans
 * une barre d'outils dense sur écran large.
 */
const SIZES: Record<Size, string> = {
  sm: 'min-h-(--target-compact) py-1.5 text-sm',
  md: 'min-h-(--target-min) py-2 text-sm',
  lg: 'min-h-(--target-min) py-2.5 text-base',
}

export function fieldControlClasses(options: FieldControlOptions = {}): string[] {
  const { hasError, disabled, readonly, size = 'md', leadingIcon, trailingIcon } = options

  // Le désactivé prime sur la lecture seule, qui prime sur l'erreur : un champ
  // hors du parcours n'a plus à réclamer une correction qu'on ne peut pas faire.
  const isReadonly = Boolean(readonly) && !disabled

  return [
    'w-full rounded-md bg-surface-raised text-text',
    'transition-[border-color,box-shadow] duration-(--duration-fast)',
    // Le texte d'invite reste du texte : `--color-text-muted` tient 7,44:1, là où
    // un gris plus pâle ferait passer une invite pour une valeur déjà saisie.
    'placeholder:text-text-muted',
    SIZES[size],
    leadingIcon ? 'pl-9' : 'pl-3',
    trailingIcon ? 'pr-9' : 'pr-3',

    // ÉPAISSEUR — « un champ en erreur porte `aria-invalid="true"` et une bordure
    // de 2 px : la couleur seule ne signale jamais un état » (guide de style).
    hasError && !disabled
      ? 'border-(length:--border-medium)'
      : 'border-(length:--border-thin)',

    // STYLE — pointillés pour la lecture seule, et pour elle seule.
    isReadonly ? 'border-dashed' : 'border-solid',

    // COULEUR — une branche par état, jamais deux classes concurrentes.
    // `--color-border-strong` tient 3,59:1 en thème clair et 4,54:1 en thème
    // sombre : un contour de champ doit rester visible.
    disabled
      ? 'border-border'
      : isReadonly
        ? 'border-border'
        : hasError
          ? 'border-danger hover:border-danger focus:border-danger'
          : 'border-border-strong hover:border-text-secondary focus:border-focus',

    // Halo de focus, en plus de l'anneau global. Posé sur `:focus` et non
    // `:focus-visible` : un champ atteint à la souris doit se voir aussi, c'est
    // lui qui va recevoir la frappe.
    'focus:shadow-(--shadow-focus)',

    // Désactivé : hors du parcours, valeur non soumise.
    disabled ? 'cursor-not-allowed bg-surface-sunken text-text-muted' : '',

    // Lecture seule : focalisable, copiable et SOUMISE — d'où le curseur de
    // texte conservé. Le fond en retrait et les pointillés disent l'un « pas
    // modifiable », l'autre « pas désactivé pour autant ».
    isReadonly ? 'bg-surface-sunken text-text-muted' : '',
  ].filter(Boolean)
}

/** Position d'une icône posée à l'intérieur d'un contrôle. */
export const FIELD_ICON_CLASSES =
  'pointer-events-none absolute top-1/2 -translate-y-1/2 text-text-subtle'
