<script setup lang="ts">
/**
 * La loupe de l'en-tête, à gauche de « Aa », sur les écrans racines des onglets : elle
 * ouvre la recherche globale (écart 62 de 05-design.md — la maquette dessine l'écran,
 * pas le chemin qui y mène).
 */
withDefaults(defineProps<{ ouverte?: boolean }>(), { ouverte: false })

const { t } = useI18n()
</script>

<template>
  <NuxtLink
    to="/guide-nego/recherche"
    class="gn-loupe"
    :aria-label="t('gn-loupe.libelle')"
    :aria-current="ouverte ? 'page' : undefined"
  >
    <GnPicto nom="search" :taille="24" />
  </NuxtLink>
</template>

<style>
[data-app="guide-nego"] .gn-loupe {
  flex: none;
  width: var(--gn-bouton-aa);
  height: var(--gn-bouton-aa);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--gn-titre);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-loupe:active {
  background: var(--gn-presse);
}
</style>
