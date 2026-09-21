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
    <GnEtatVide
      picto="doc"
      :titre="t('guide-nego.ressources.vide.titre')"
      :texte="t('guide-nego.ressources.vide.texte')"
    />
    <GnLigneReglage
      :libelle="t('guide-nego.ressources.profil')"
      picto="user"
      vers="/guide-nego/ressources/reglages"
      derniere
    />
  </GnEcran>
</template>
