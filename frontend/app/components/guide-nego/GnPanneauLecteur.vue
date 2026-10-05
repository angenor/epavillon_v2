<script setup lang="ts">
/**
 * Une feuille posée sur le lecteur : le sommaire (04 · 04), la recherche (04 · 05). Le texte
 * reste dessous, à sa page : refermer y ramène sans rien recharger ni recaler.
 */
defineProps<{ titre: string; sousTitre?: string }>()

const ouvert = defineModel<boolean>({ required: true })

const { t } = useI18n()
</script>

<template>
  <GnFeuilleBasse v-model="ouvert" :titre="titre" :sous-titre="sousTitre" :fermeture="t('gn-panneau-lecteur.fermer')">
    <div class="gn-panneau-lecteur">
      <slot />
    </div>
  </GnFeuilleBasse>
</template>

<style>
/* Une hauteur plancher : la feuille de recherche ne saute pas à chaque lettre tapée. */
[data-app="guide-nego"] .gn-panneau-lecteur {
  flex: none;
  min-height: 40dvh;
}
</style>
