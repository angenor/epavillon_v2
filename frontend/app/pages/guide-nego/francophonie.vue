<script setup lang="ts">
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()

useHead({ title: t('guide-nego.francophonie.titre') })
</script>

<template>
  <GnEcran
    :titre="t('guide-nego.francophonie.titre')"
    :sous-titre="t('guide-nego.francophonie.sous-titre')"
    cloche
  >
    <GnEtatVide
      picto="franco"
      :titre="t('guide-nego.francophonie.vide.titre')"
      :texte="t('guide-nego.francophonie.vide.texte')"
    />
  </GnEcran>
</template>
