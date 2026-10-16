<script setup lang="ts">
/**
 * PAGE D'ATTENTE de la plateforme fermée (drapeau `platform.maintenance`).
 *
 * Atteinte par le middleware global `maintenance`, jamais par un lien. Le logo et
 * « Plateforme en maintenance », rien d'autre (arbitré le 16/10) : ni raison, ni
 * date, ni chemin vers la connexion des administrateurs, qui passent par
 * `/connexion` directement.
 *
 * 503 au rendu serveur : un moteur de recherche lit une indisponibilité
 * passagère et garde les pages indexées, là où un 200 le ferait indexer cette
 * page à la place de toutes les autres.
 */
definePageMeta({ layout: false })

const { t } = useI18n()

if (import.meta.server) {
  setResponseStatus(useRequestEvent()!, 503)
  useResponseHeader('Retry-After').value = '3600'
}

useHead({
  title: t('maintenance.platform.title'),
  meta: [{ name: 'robots', content: 'noindex' }],
})
</script>

<template>
  <main class="flex min-h-screen flex-col items-center justify-center gap-8 bg-surface px-4 text-center text-text">
    <img
      :src="assetUrl('/logos/svg/epavillon.svg')"
      :alt="t('nav.site.name')"
      class="h-32 w-auto dark:hidden"
      width="100"
      height="128"
    >
    <img
      :src="assetUrl('/logos/svg/epavillon-inverse.svg')"
      :alt="t('nav.site.name')"
      class="hidden h-32 w-auto dark:block"
      width="100"
      height="128"
    >
    <h1 class="font-display text-3xl text-text">{{ t('maintenance.platform.title') }}</h1>
  </main>
</template>
