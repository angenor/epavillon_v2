<script setup lang="ts">
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const { echangesOuverts, pret } = useGnDrapeaux()

// L'onglet disparaît avec le drapeau ; son adresse, forgée ou gardée en favori, doit
// disparaître avec lui.
watchEffect(() => {
  if (pret.value && !echangesOuverts.value) void navigateTo('/guide-nego', { replace: true })
})

useHead({ title: t('guide-nego.echanges.titre') })
</script>

<template>
  <GnEcran :titre="t('guide-nego.echanges.titre')">
    <GnEtatVide
      picto="chat"
      :titre="t('guide-nego.echanges.vide.titre')"
      :texte="t('guide-nego.echanges.vide.texte')"
    />
    <p class="gn-echanges__mention">{{ t('guide-nego.echanges.vide.mention') }}</p>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-echanges__mention {
  padding: var(--gn-espace-12) var(--gn-espace-16);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  text-align: center;
}
</style>
