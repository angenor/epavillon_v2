<script setup lang="ts">
import type { ButtonVariant, Size } from '~/types/ui'

/**
 * Bouton — cinq variantes, trois tailles, tous les états.
 *
 * SIX ÉTATS, TOUS TRAITÉS : repos, survol, focus clavier, actif (enfoncé),
 * désactivé, chargement. Ce sont les deux derniers qu'on oublie, et ce sont eux
 * qui font mauvaise impression : un bouton qui ne dit pas qu'il travaille se
 * fait cliquer trois fois, et un dossier se dépose trois fois.
 *
 * POIDS VISUEL — une page ne porte qu'UN bouton principal. Le secondaire borde,
 * le discret (`ghost`) sert les actions de barre d'outils, le `link` ne sert que
 * lorsqu'une action doit se fondre dans une phrase. `danger` est réservé à ce
 * qui détruit ou refuse : la couleur distingue, elle ne décore pas.
 *
 * FOCUS — l'anneau est celui de `main.css`, commun à toute la plateforme. Il
 * n'est pas redéfini ici : un anneau par composant, ce sont autant de façons de
 * ne plus se voir sur un fond donné.
 *
 * POLYMORPHE — `to` rend un `NuxtLink` (navigation interne), `href` un `<a>`
 * (lien externe, avec son `rel` de sécurité), sinon un `<button>`. Un lien qui
 * navigue ne doit jamais être un `<button>` : ni clic milieu, ni ouverture dans
 * un onglet.
 */

interface Props {
  variant?: ButtonVariant
  size?: Size
  /** Type HTML. `button` par défaut : un bouton dans un formulaire ne soumet pas par accident. */
  type?: 'button' | 'submit' | 'reset'
  /** Navigation interne — rend un `NuxtLink`. Passer par `localePath()` en amont. */
  to?: string
  /** Lien externe — rend un `<a>` avec `rel="noopener noreferrer"`. */
  href?: string
  /** Icône avant le libellé (nom de `UiIcon`). */
  icon?: string
  /** Icône après le libellé — chevrons, flèches de navigation. */
  iconTrailing?: string
  /**
   * Bouton réduit à son icône. `label` devient alors OBLIGATOIRE : il porte le
   * nom accessible et l'infobulle.
   */
  iconOnly?: boolean
  /** Nom accessible, requis quand `iconOnly` est vrai. */
  label?: string
  disabled?: boolean
  /**
   * Le bouton travaille : tourniquet à la place de l'icône, `aria-busy`, et
   * clics ignorés. La largeur ne bouge pas — un bouton qui rétrécit pendant
   * qu'il travaille déplace tout ce qui l'entoure.
   */
  loading?: boolean
  /** Pleine largeur — formulaires en colonne, écrans étroits. */
  block?: boolean
  /** Bouton actif dans un groupe (barre d'outils, bascule de vue). */
  pressed?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  variant: 'primary',
  size: 'md',
  type: 'button',
})

defineEmits<{ click: [event: MouseEvent] }>()

const isInert = computed(() => props.disabled || props.loading)

const tag = computed(() => {
  if (props.to) return resolveComponent('NuxtLink')
  if (props.href) return 'a'
  return 'button'
})

/**
 * Base commune : la mise en page, la transition et le tracé. Aucune couleur —
 * elles arrivent par la variante, toutes en jetons de rôle.
 */
const BASE =
  'inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border font-medium ' +
  'no-underline transition-colors duration-150 select-none ' +
  'disabled:cursor-not-allowed aria-disabled:cursor-not-allowed'

const VARIANTS: Record<ButtonVariant, string> = {
  // Aplat cyan. `--color-accent-solid` tient 4,23:1 face au blanc en thème clair,
  // et le thème sombre bascule sur le cyan de charte avec un texte sombre.
  primary:
    'border-transparent bg-accent-solid text-accent-contrast ' +
    'hover:bg-accent-solid-hover active:bg-accent-hover ' +
    'disabled:bg-border disabled:text-text-subtle aria-disabled:bg-border aria-disabled:text-text-subtle',

  // Contour franc : lisible sans peser autant que l'aplat.
  secondary:
    'border-border-strong bg-surface-raised text-text ' +
    'hover:bg-surface-hover hover:border-text-subtle active:bg-surface-sunken ' +
    'disabled:bg-surface disabled:text-text-subtle disabled:border-border ' +
    'aria-disabled:bg-surface aria-disabled:text-text-subtle aria-disabled:border-border',

  // Sans contour au repos : réservé aux barres d'outils, où dix boutons bordés
  // deviendraient un grillage.
  ghost:
    'border-transparent bg-transparent text-text-muted ' +
    'hover:bg-surface-hover hover:text-text active:bg-surface-sunken ' +
    'disabled:text-text-subtle disabled:hover:bg-transparent ' +
    'aria-disabled:text-text-subtle aria-disabled:hover:bg-transparent',

  // Détruire, refuser, retirer. Jamais « enregistrer ».
  danger:
    'border-transparent bg-danger-solid text-danger-contrast ' +
    'hover:brightness-95 active:brightness-90 ' +
    'disabled:bg-border disabled:text-text-subtle aria-disabled:bg-border aria-disabled:text-text-subtle',

  // Action qui se fond dans une phrase. Souligné au survol, comme un lien.
  link:
    'border-transparent bg-transparent px-0! text-text-link underline-offset-4 ' +
    'hover:underline hover:text-text-link-hover active:text-text-link-hover ' +
    'disabled:text-text-subtle disabled:no-underline aria-disabled:text-text-subtle',
}

/** Hauteurs de frappe : 32 / 38 / 44 px. La plus petite reste cliquable au doigt
 *  dans une barre d'outils dense, la moyenne est le défaut des formulaires. */
const SIZES: Record<Size, string> = {
  sm: 'h-8 px-3 text-sm',
  md: 'h-[2.375rem] px-4 text-sm',
  lg: 'h-11 px-5 text-base',
}

/** Version carrée pour les boutons réduits à leur icône. */
const ICON_ONLY_SIZES: Record<Size, string> = {
  sm: 'h-8 w-8 p-0',
  md: 'h-[2.375rem] w-[2.375rem] p-0',
  lg: 'h-11 w-11 p-0',
}

const classes = computed(() => [
  BASE,
  VARIANTS[props.variant],
  props.iconOnly ? ICON_ONLY_SIZES[props.size] : SIZES[props.size],
  props.block ? 'w-full' : '',
  // `pressed` marque l'option retenue d'un groupe : même traitement que la
  // surface sélectionnée du reste de l'interface, pour ne pas inventer un signal.
  props.pressed ? 'bg-surface-selected text-accent border-accent-border' : '',
])

/** Taille d'icône accordée au corps de texte du bouton. */
const iconSize = computed(() => (props.size === 'lg' ? '1.15em' : '1.05em'))
</script>

<template>
  <component
    :is="tag"
    :to="props.to"
    :href="props.href"
    :type="props.to || props.href ? undefined : props.type"
    :target="props.href ? '_blank' : undefined"
    :rel="props.href ? 'noopener noreferrer' : undefined"
    :class="classes"
    :disabled="tag === 'button' ? isInert : undefined"
    :aria-disabled="tag !== 'button' && isInert ? 'true' : undefined"
    :tabindex="tag !== 'button' && isInert ? -1 : undefined"
    :aria-busy="props.loading ? 'true' : undefined"
    :aria-pressed="props.pressed === undefined ? undefined : props.pressed"
    :aria-label="props.iconOnly ? props.label : undefined"
    :title="props.iconOnly ? props.label : undefined"
    @click="(event: MouseEvent) => (isInert ? event.preventDefault() : $emit('click', event))"
  >
    <!-- Le tourniquet REMPLACE l'icône : la largeur du bouton ne change pas
         quand il se met à travailler. -->
    <UiSpinner v-if="props.loading" :size="iconSize" />
    <UiIcon v-else-if="props.icon" :name="props.icon" :size="iconSize" />

    <span v-if="!props.iconOnly" :class="props.loading && !props.icon ? 'opacity-90' : ''">
      <slot>{{ props.label }}</slot>
    </span>

    <UiIcon v-if="props.iconTrailing && !props.loading" :name="props.iconTrailing" :size="iconSize" />
  </component>
</template>
