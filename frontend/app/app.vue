<script setup lang="ts">
/**
 * Racine de l'application.
 *
 * Trois responsabilités, et rien d'autre :
 *  · poser `data-theme` sur `<html>` DÈS LE RENDU SERVEUR — le choix vient d'un
 *    cookie, ce qui évite qu'une page s'affiche en clair avant de basculer en
 *    sombre ; l'attribut est absent en mode « système », laissant la media
 *    query `prefers-color-scheme` décider ;
 *  · poser les attributs `lang` / `dir` et les liens alternatifs de langue ;
 *  · choisir le layout. Le layout `public` est le défaut ; une page passe au
 *    back-office avec `definePageMeta({ layout: 'admin' })`.
 */
const { t } = useI18n()
const route = useRoute()
const preferences = usePreferencesStore()
const localeHead = useLocaleHead()

const layoutName = computed(() => {
  const declared = route.meta.layout
  if (declared === false) return false
  return declared ?? 'public'
})

useHead(() => ({
  htmlAttrs: {
    ...localeHead.value.htmlAttrs,
    'data-theme': preferences.themeAttribute ?? undefined,
  },
  link: [...(localeHead.value.link ?? [])],
  meta: [...(localeHead.value.meta ?? [])],
  titleTemplate: (title?: string) =>
    title ? `${title} — ${t('nav.site.name')}` : t('nav.site.name'),
}))
</script>

<template>
  <NuxtLayout :name="layoutName">
    <NuxtPage />
  </NuxtLayout>
</template>
