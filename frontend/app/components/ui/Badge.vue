<script setup lang="ts">
import type { Intent } from '~/types/ui'

/**
 * Pastille — un état, une catégorie, un compteur. Statique : elle ne se clique
 * pas et ne se retire pas. Pour un filtre retirable, c'est `UiChip`.
 *
 * LA COULEUR DISTINGUE, ELLE NE DÉCORE PAS. Cinq intentions, pas une de plus, et
 * chacune correspond à une nature d'information. Une pastille grise n'est pas un
 * échec de conception : c'est le cas ordinaire.
 *
 * COULEUR VENUE DE LA BASE — `dotColor`. Les thématiques d'activité, les
 * journées spéciales et les jours remarquables portent une couleur en base
 * (`taxonomy_terms.color_hex`, `programme_tracks.color_hex`) : c'est une DONNÉE,
 * saisie au back-office, et rien ne garantit son contraste. Elle est donc rendue
 * en POINT à côté du libellé, jamais en fond de texte. Le texte, lui, garde les
 * jetons du thème — et reste lisible en clair comme en sombre, quelle que soit
 * la couleur choisie par l'administrateur. C'est la seule façon d'honorer à la
 * fois « aucune couleur en dur » et « la couleur vient de la base ».
 */

interface Props {
  intent?: Intent
  /** Libellé, quand on ne passe pas par le créneau par défaut. */
  label?: string
  /** Icône de tête (nom de `UiIcon`). */
  icon?: string
  /**
   * Couleur venue de la BASE, rendue en point. Ni fond, ni texte : voir
   * l'en-tête. `null` ou absente, aucun point n'est rendu.
   */
  dotColor?: string | null
  /** Fond teinté plutôt que simple contour — à réserver aux états, pas aux catégories. */
  solid?: boolean
  size?: 'sm' | 'md'
}

const props = withDefaults(defineProps<Props>(), { intent: 'neutral', size: 'md' })

/** Contour + fond très pâle : lisible sans peser dans une liste dense. */
const SUBTLE: Record<Intent, string> = {
  neutral: 'border-border bg-surface-sunken text-text-muted',
  info: 'border-info-border bg-info-surface text-info',
  success: 'border-success-border bg-success-surface text-success',
  warning: 'border-warning-border bg-warning-surface text-warning',
  danger: 'border-danger-border bg-danger-surface text-danger',
}

/** Aplat : réservé à ce qui doit se voir depuis l'autre bout de l'écran. */
const SOLID: Record<Intent, string> = {
  neutral: 'border-transparent bg-text-muted text-surface-raised',
  info: 'border-transparent bg-info-solid text-info-contrast',
  success: 'border-transparent bg-success-solid text-success-contrast',
  warning: 'border-transparent bg-warning-solid text-warning-contrast',
  danger: 'border-transparent bg-danger-solid text-danger-contrast',
}

const classes = computed(() => [
  'inline-flex items-center gap-1.5 rounded-full border font-medium whitespace-nowrap',
  props.size === 'sm' ? 'px-2 py-0.5 text-xs' : 'px-2.5 py-1 text-xs',
  props.solid ? SOLID[props.intent] : SUBTLE[props.intent],
])
</script>

<template>
  <span :class="classes">
    <span
      v-if="props.dotColor"
      class="size-2 shrink-0 rounded-full ring-1 ring-black/10"
      :style="{ backgroundColor: props.dotColor }"
      aria-hidden="true"
    />
    <UiIcon v-else-if="props.icon" :name="props.icon" size="0.9em" />
    <slot>{{ props.label }}</slot>
  </span>
</template>
