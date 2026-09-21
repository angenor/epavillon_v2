<script setup lang="ts">
import sprite from '~/assets/guide-nego/pictogrammes.svg'

/** Un pictogramme n'informe jamais seul : le mot qui l'accompagne porte le sens. */
withDefaults(defineProps<{ nom: string; taille?: 16 | 18 | 20 | 24 | 26 | 40 }>(), { taille: 24 })
</script>

<template>
  <svg class="gn-picto" :width="taille" :height="taille" aria-hidden="true" focusable="false">
    <use :href="`${sprite}#gn-${nom}`" />
  </svg>
</template>

<style>
[data-app="guide-nego"] .gn-picto {
  flex: none;
  display: block;
  color: inherit;
}
</style>
