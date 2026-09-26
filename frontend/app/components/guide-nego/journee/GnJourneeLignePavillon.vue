<script setup lang="ts">
/** La ligne Pavillon de « Ma journée ». Vide à l'étape 4 : l'étape 5 réécrit ce seul composant. */
defineProps<{ maintenant: Date }>()

const { t } = useI18n()
</script>

<template>
  <GnJourneeLigneVide
    picto="flag"
    :origine="t('gn-journee-ligne-pavillon.origine')"
    :texte="t('gn-journee-ligne-pavillon.vide')"
    vers="/guide-nego/francophonie?section=pavillon"
  />
</template>
