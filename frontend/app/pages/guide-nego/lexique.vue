<script setup lang="ts">
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()

useHead({ title: t('guide-nego.lexique.titre') })
</script>

<template>
  <GnEcran
    :titre="t('guide-nego.lexique.titre')"
    :sous-titre="t('guide-nego.lexique.sous-titre')"
    retour="/guide-nego"
    :onglets="false"
    lexique-ouvert
  >
    <GnEtatVide
      picto="text-size"
      :titre="t('guide-nego.lexique.vide.titre')"
      :texte="t('guide-nego.lexique.vide.texte')"
    />
  </GnEcran>
</template>
