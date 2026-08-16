<script setup lang="ts">
/**
 * PAGE D'AMORÇAGE — provisoire.
 *
 * Le prompt A0.1 ne demande aucune page : les écrans sont l'objet des prompts
 * suivants, et l'accueil public reviendra au prompt A3. Cette page existe pour
 * une raison technique : sans aucune route, Nuxt ne monte ni routeur, ni layout,
 * ni i18n d'URL — `npm run build` réussirait sans rien exercer, et `make check`
 * ne prouverait rien. Elle n'utilise que ce que le socle fournit et sera
 * REMPLACÉE, pas complétée.
 */
// Aucun `title` ici : le gabarit d'`app.vue` pose déjà le nom du site.
const { t } = useI18n()
</script>

<template>
  <div class="max-w-2xl">
    <p class="text-sm uppercase tracking-wide text-text-subtle">
      {{ t('nav.site.ownerShort') }} · {{ t('nav.site.parentShort') }}
    </p>
    <h1 class="mt-2 text-3xl">{{ t('nav.site.name') }}</h1>
    <p class="mt-3 text-lg text-text-muted">{{ t('nav.site.tagline') }}</p>

    <p
      class="mt-8 rounded-lg border border-border bg-surface-raised px-4 py-3 text-sm text-text-muted"
    >
      {{ t('nav.maintenance.description') }}
    </p>
  </div>
</template>
