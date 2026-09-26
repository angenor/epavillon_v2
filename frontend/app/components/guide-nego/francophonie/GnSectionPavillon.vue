<script setup lang="ts">
/** La section Pavillon de l'onglet Francophonie. Vide à l'étape 4 : l'étape 5 réécrit ce seul composant. */
const { t } = useI18n()
</script>

<template>
  <GnEtatVide picto="franco" :titre="t('gn-section-pavillon.titre')" :texte="t('gn-section-pavillon.texte')" />
</template>
