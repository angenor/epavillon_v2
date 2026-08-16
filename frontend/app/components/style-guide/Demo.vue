<script setup lang="ts">
/**
 * Un exemple du guide de style : un titre, une note, un rendu réel.
 *
 * LE RENDU EST LE VRAI COMPOSANT, pas une capture ni une reproduction. C'est ce
 * qui fait du guide un test de non-régression visuelle : une retouche de jeton
 * ou de composant se voit ici immédiatement, sur tous ses états à la fois.
 *
 * LA NOTE EXPLIQUE LE CHOIX, pas la mécanique. « Une page ne porte qu'un bouton
 * principal » est utile ; « ce bouton est bleu » ne l'est pas.
 *
 * `surface` pose le rendu sur le fond de page plutôt que sur la surface haute :
 * à utiliser pour les composants qui portent déjà leur propre fond (cartes,
 * bandeaux), sans quoi on ne voit plus leur limite.
 */

interface Props {
  title: string
  /** Ce que l'exemple démontre, et la décision qu'il porte. */
  note?: string
  /** Rendu sur le fond de page, sans cadre intérieur. */
  surface?: boolean
  /** Étale le rendu sur toute la largeur, sans marges intérieures. */
  flush?: boolean
}

const props = defineProps<Props>()
</script>

<template>
  <div>
    <div class="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
      <h3 class="font-display text-base text-text">{{ props.title }}</h3>
      <slot name="meta" />
    </div>
    <p v-if="props.note" class="mt-1 max-w-prose text-sm text-text-muted">{{ props.note }}</p>

    <div
      class="mt-3 rounded-lg border border-border"
      :class="[
        props.surface ? 'bg-surface' : 'bg-surface-raised',
        props.flush ? 'overflow-hidden' : 'p-4',
      ]"
    >
      <slot />
    </div>
  </div>
</template>
