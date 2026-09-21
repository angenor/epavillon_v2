<script setup lang="ts">
/**
 * Mise en page de Guide Négo — la borne de son système de design.
 *
 * Tout ce que l'application dessine vit sous l'élément qui porte `data-app` :
 * jetons, thème, styles, et `#gn-portail`, seule cible de téléportation.
 */
import '~/assets/guide-nego/police.css'
import '~/assets/guide-nego/theme.css'
import '~/assets/guide-nego/mesures.css'
import '~/assets/guide-nego/base.css'

useHead({ htmlAttrs: { lang: 'fr' } })
</script>

<template>
  <div data-app="guide-nego">
    <slot />
    <div id="gn-portail" />
  </div>
</template>
