<script setup lang="ts">
import type { RouteLocationRaw } from 'vue-router'

/**
 * « Se tient aussi au Pavillon » (01 · 4 decies) : l'étiquette bordée, avec l'épingle.
 * Touchée, elle ouvre l'activité liée (`vers`) ; à défaut, la section Pavillon.
 * La cible fait 48 px, le cadre dessiné reste celui d'une étiquette.
 */
withDefaults(defineProps<{ vers?: RouteLocationRaw }>(), {
  vers: () => ({ path: '/guide-nego/francophonie', query: { section: 'pavillon' } }),
})

const { t } = useI18n()
</script>

<template>
  <NuxtLink :to="vers" class="gn-etiquette-pavillon" :aria-label="t('gn-etiquette-pavillon.ouvrir')">
    <GnEtiquette picto="pin" :texte="t('gn-etiquette-pavillon.texte')" />
  </NuxtLink>
</template>

<style>
[data-app="guide-nego"] .gn-etiquette-pavillon {
  align-self: flex-start;
  min-height: var(--gn-bouton-rond);
  display: inline-flex;
  align-items: center;
  color: inherit;
  text-decoration: none;
}

[data-app="guide-nego"] .gn-etiquette-pavillon:active .gn-etiquette {
  background: var(--gn-presse);
}
</style>
