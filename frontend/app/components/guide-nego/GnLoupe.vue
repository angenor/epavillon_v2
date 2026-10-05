<script setup lang="ts">
/**
 * La loupe de l'en-tête, sur les écrans racines des onglets : elle ouvre la recherche
 * globale (écart 62 de 05-design.md).
 */
withDefaults(defineProps<{ ouverte?: boolean }>(), { ouverte: false })

const { t } = useI18n()
</script>

<template>
  <GnBoutonRond vers="/guide-nego/recherche" picto="search" :libelle="t('gn-loupe.libelle')" :actif="ouverte" />
</template>
