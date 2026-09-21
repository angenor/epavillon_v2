<script setup lang="ts">
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()

useHead({ title: t('guide-nego.fermee.titre'), meta: [{ name: 'robots', content: 'noindex' }] })
</script>

<template>
  <main class="gn-fermee">
    <GnPicto nom="lock" :taille="40" />
    <h1 class="gn-fermee__titre">{{ t('guide-nego.fermee.titre') }}</h1>
    <p>{{ t('guide-nego.fermee.texte') }}</p>
    <p class="gn-fermee__editeur">{{ t('guide-nego.fermee.editeur') }}</p>
  </main>
</template>

<style>
[data-app="guide-nego"] .gn-fermee {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: var(--gn-espace-16);
  padding: var(--gn-espace-24) var(--gn-marge-ecran);
  color: var(--gn-texte);
}

[data-app="guide-nego"] .gn-fermee .gn-picto {
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-fermee__titre {
  font-size: var(--gn-taille-28);
  line-height: var(--gn-interligne-28);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-fermee__editeur {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}
</style>
