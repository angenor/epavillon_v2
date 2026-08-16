<script setup lang="ts">
/**
 * Squelette de chargement — PREMIER des quatre états d'écran.
 *
 * POURQUOI UN SQUELETTE PLUTÔT QU'UN TOURNIQUET. Un squelette annonce la FORME
 * de ce qui arrive : trois lignes de titre, une grille de six cartes, douze
 * lignes de tableau. La page ne saute pas quand les données arrivent, et
 * l'attente paraît plus courte parce qu'elle est renseignée. Un tourniquet
 * centré ne dit rien et laisse la mise en page se recomposer d'un coup.
 *
 * DIMENSIONS DONNÉES PAR L'APPELANT. Un squelette qui ne ressemble pas à ce
 * qu'il remplace est pire que rien : il déplace le contenu au moment du
 * remplacement, exactement ce qu'il devait éviter.
 *
 * ACCESSIBILITÉ : le squelette est décoratif (`aria-hidden`), et c'est le
 * conteneur qui porte `aria-busy="true"`. Annoncer douze squelettes reviendrait
 * à lire douze fois « chargement ».
 *
 * L'animation est neutralisée par `prefers-reduced-motion` (voir `main.css`) ;
 * les blocs restent alors visibles, sans pulsation.
 */

interface Props {
  /** Largeur CSS. Un pourcentage donne l'aspect irrégulier d'un vrai texte. */
  width?: string
  height?: string
  /** Nombre de lignes ; la dernière est raccourcie, comme un paragraphe réel. */
  lines?: number
  /** Forme : bloc rectangulaire, ligne de texte, pastille ronde. */
  variant?: 'block' | 'text' | 'circle'
  rounded?: string
}

const props = withDefaults(defineProps<Props>(), {
  width: '100%',
  lines: 1,
  variant: 'block',
})

const defaultHeight = computed(() => (props.variant === 'text' ? '0.85rem' : '1.25rem'))

const radius = computed(() => {
  if (props.rounded) return props.rounded
  if (props.variant === 'circle') return 'var(--radius-full)'
  if (props.variant === 'text') return 'var(--radius-sm)'
  return 'var(--radius-md)'
})
</script>

<template>
  <div v-if="props.lines > 1" class="space-y-2" aria-hidden="true">
    <span
      v-for="line in props.lines"
      :key="line"
      class="ui-skeleton block"
      :style="{
        // La dernière ligne est plus courte : un paragraphe ne finit jamais au
        // ras de la marge.
        width: line === props.lines ? '65%' : props.width,
        height: props.height ?? defaultHeight,
        borderRadius: radius,
      }"
    />
  </div>

  <span
    v-else
    class="ui-skeleton block"
    aria-hidden="true"
    :style="{
      width: props.width,
      height: props.height ?? defaultHeight,
      borderRadius: radius,
    }"
  />
</template>

<style scoped>
.ui-skeleton {
  background-color: var(--color-surface-sunken);
  border: 1px solid var(--color-border-subtle);
  animation: ui-skeleton-pulse 1.6s ease-in-out infinite;
}

@keyframes ui-skeleton-pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.55;
  }
}
</style>
