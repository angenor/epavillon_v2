<script setup lang="ts">
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()

useHead({ title: t('guide-nego.negociations.titre') })
</script>

<template>
  <GnEcran :titre="t('guide-nego.negociations.titre')">
    <GnEtatVide
      picto="nego"
      :titre="t('guide-nego.negociations.vide.titre')"
      :texte="t('guide-nego.negociations.vide.texte')"
    />
  </GnEcran>
</template>
