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
 * LE SECONDAIRE EST UN CONTOUR ACCENT, pas un bouton neutre. Un contour gris se
 * lit comme un bouton désactivé et laisse le primaire seul en piste ; le contour
 * cyan pose une vraie seconde action, subordonnée sans être éteinte.
 *
 * TROIS PALIERS SUR LES APLATS : repos, survol, enfoncé. Deux suffiraient à voir
 * que le bouton réagit, mais le troisième est ce qui accuse réception du clic
 * sur une liaison lente — celle d'une salle de conférence. Chaque palier est un
 * JETON : une valeur calculée à la volée (un `brightness` par exemple) échappe au
 * thème sombre et au contrôle de contraste.
 *
 * TAILLES — 44 px pour `md` et `lg` (`--target-min`), le minimum d'une cible
 * visée au doigt. `sm` descend à 40 px (`--target-compact`) et se réserve aux
 * barres d'outils denses sur écran large : LA TAILLE COMPACTE N'EST JAMAIS
 * L'ACTION PRINCIPALE D'UN ÉCRAN MOBILE.
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
   * Le bouton travaille : tourniquet AJOUTÉ au libellé, `aria-busy`, et clics
   * ignorés. Le libellé reste lisible — « Envoi… » sans texte n'est plus un
   * bouton, c'est une énigme.
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
 *
 * GRAISSE 700 sur le libellé : tout le vocabulaire d'action de la plateforme est
 * en gras, et un bouton dont le texte pèse le même poids que le paragraphe
 * voisin ne se repère plus en balayant la page.
 *
 * DÉSACTIVÉ UNIFORME — une opacité et un curseur, les mêmes pour les cinq
 * variantes. Substituer des couleurs variante par variante, comme on le faisait,
 * produit cinq gris différents dont aucun ne dit « désactivé » : l'affaiblissement
 * de l'ensemble le dit mieux, et sans multiplier les cas à vérifier au contraste.
 * Le clic est déjà neutralisé par le gestionnaire ; on ne coupe donc PAS les
 * événements de pointeur, sans quoi `cursor-not-allowed` ne s'afficherait jamais.
 */
const BASE =
  'inline-flex cursor-pointer items-center justify-center gap-2 whitespace-nowrap ' +
  'rounded-md border-(length:--border-thin) border-solid font-bold leading-tight ' +
  'no-underline select-none transition-colors duration-(--duration-fast) ' +
  'disabled:cursor-not-allowed disabled:opacity-[.45] ' +
  'aria-disabled:cursor-not-allowed aria-disabled:opacity-[.45]'

const VARIANTS: Record<ButtonVariant, string> = {
  // Aplat cyan. `--color-accent-solid` tient 4,23:1 face au blanc en thème clair,
  // et le thème sombre bascule sur le cyan de charte avec un texte sombre.
  primary:
    'border-transparent bg-accent-solid text-accent-contrast ' +
    'hover:bg-accent-solid-hover active:bg-accent-active',

  // Contour ACCENT — la seconde action de la page, subordonnée et lisible.
  // L'enfoncé teinte un peu plus le fond que le survol : `color-mix` sur le
  // jeton de rôle, jamais sur une couleur de marque, pour suivre les deux thèmes.
  secondary:
    'border-accent bg-transparent text-accent ' +
    'hover:bg-accent-surface active:bg-[color-mix(in_srgb,var(--color-accent)_18%,transparent)]',

  // Sans contour au repos : réservé aux barres d'outils, où dix boutons bordés
  // deviendraient un grillage.
  ghost:
    'border-transparent bg-transparent text-text-secondary ' +
    'hover:bg-surface-sunken hover:text-text active:bg-neutral-surface',

  // Détruire, refuser, retirer. Jamais « enregistrer ».
  // Trois paliers, comme l'accent — et pris aux jetons, pas calculés : le rouge
  // fonce au survol en thème clair, s'éclaircit en thème sombre où l'aplat porte
  // du texte sombre. Un `brightness()` aurait fait l'inverse dans l'un des deux.
  danger:
    'border-transparent bg-danger-solid text-danger-contrast ' +
    'hover:bg-danger-solid-hover active:bg-danger-solid-active',

  // Action qui se fond dans une phrase. Souligné au survol, comme un lien.
  link:
    'border-transparent bg-transparent px-0! text-text-link underline-offset-4 ' +
    'hover:underline hover:text-text-link-hover active:text-text-link-hover',
}

/**
 * Hauteur MINIMALE et non fixe : un libellé long qui se replie sur deux lignes
 * doit faire grandir le bouton, pas déborder de son cadre.
 */
const SIZES: Record<Size, string> = {
  sm: 'min-h-(--target-compact) px-4 py-1 text-sm',
  md: 'min-h-(--target-min) px-5 py-2 text-sm',
  lg: 'min-h-(--target-min) px-6 py-2.5 text-base',
}

/** Version carrée pour les boutons réduits à leur icône : la cible reste pleine. */
const ICON_ONLY_SIZES: Record<Size, string> = {
  sm: 'min-h-(--target-compact) w-(--target-compact) p-0',
  md: 'min-h-(--target-min) w-(--target-min) p-0',
  lg: 'min-h-(--target-min) w-(--target-min) p-0',
}

/**
 * `pressed` marque l'option retenue d'un groupe (bascule de vue, barre d'outils)
 * et REMPLACE la variante au lieu de s'y ajouter. Superposées, les deux jeux de
 * classes visent les mêmes propriétés, et ce n'est pas l'ordre d'écriture qui
 * tranche mais l'ordre alphabétique de la feuille générée : `bg-transparent`
 * l'emportait sur `bg-surface-selected` et l'option retenue restait invisible.
 * Le traitement est celui de la surface sélectionnée du reste de l'interface —
 * on ne réinvente pas un signal pour les boutons.
 */
const PRESSED =
  'border-accent-border bg-surface-selected text-accent hover:bg-accent-surface'

const classes = computed(() => [
  BASE,
  props.pressed ? PRESSED : VARIANTS[props.variant],
  props.iconOnly ? ICON_ONLY_SIZES[props.size] : SIZES[props.size],
  props.block ? 'w-full' : '',
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
    <!-- Le tourniquet s'ajoute au libellé, qui ne disparaît jamais. Quand le
         bouton porte déjà une icône, il en prend la place : la largeur ne bouge
         donc pas d'un pixel, et rien de ce qui entoure le bouton ne se déplace
         au moment précis où l'on vient de cliquer dessus. -->
    <UiSpinner v-if="props.loading" :size="iconSize" />
    <UiIcon v-else-if="props.icon" :name="props.icon" :size="iconSize" />

    <span v-if="!props.iconOnly">
      <slot>{{ props.label }}</slot>
    </span>

    <UiIcon v-if="props.iconTrailing && !props.loading" :name="props.iconTrailing" :size="iconSize" />
  </component>
</template>
