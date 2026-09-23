<script setup lang="ts">
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()

useHead({ title: t('guide-nego.ressources.titre') })
</script>

<template>
  <GnEcran
    :titre="t('guide-nego.ressources.titre')"
    :sous-titre="t('guide-nego.ressources.sous-titre')"
  >
    <GnLigneReglage
      :libelle="t('guide-nego.ressources.documents')"
      picto="doc"
      vers="/guide-nego/ressources/documents"
    />
    <GnLigneReglage
      :libelle="t('guide-nego.ressources.mes-documents')"
      picto="download"
      vers="/guide-nego/ressources/mes-documents"
    />
    <GnLigneReglage
      :libelle="t('guide-nego.ressources.profil')"
      picto="user"
      vers="/guide-nego/ressources/reglages"
      derniere
    />
  </GnEcran>
</template>
