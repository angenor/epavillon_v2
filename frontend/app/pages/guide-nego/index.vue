<script setup lang="ts">
import { CLE_OUVERTURE_VUE, lireCle } from '~/utils/guide-nego/stockage'

definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()

// La première venue passe par l'écran d'ouverture ; ensuite, l'accueil s'ouvre seul.
onMounted(() => {
  if (!lireCle(CLE_OUVERTURE_VUE)) void navigateTo('/guide-nego/ouverture', { replace: true })
})

useHead({ title: t('guide-nego.accueil.titre') })
</script>

<template>
  <GnEcran :titre="t('guide-nego.accueil.titre')">
    <GnEtatVide
      picto="calendar"
      :titre="t('guide-nego.accueil.vide.titre')"
      :texte="t('guide-nego.accueil.vide.texte')"
    />
  </GnEcran>
</template>
