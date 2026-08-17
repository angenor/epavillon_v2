<script setup lang="ts">
/**
 * ACCUEIL — il conduit à l'édition en cours.
 *
 * La page d'amorçage du prompt A0.1 annonçait qu'elle serait REMPLACÉE par
 * l'écran A3, pas complétée : c'est fait ici.
 *
 * POURQUOI UNE REDIRECTION PLUTÔT QU'UNE PAGE DE PLUS. La page publique d'une
 * édition (`/evenements/<slug>`) répond déjà aux quatre questions qu'un visiteur
 * se pose, appel à propositions compris. En composer une seconde, presque
 * identique, sur `/`, garantirait deux mises en page divergentes dans six mois.
 * L'adresse canonique reste celle qui porte le slug : elle se partage, elle se
 * cite, et elle survit au passage à l'édition suivante.
 *
 * L'ÉDITION EN COURS EST CHOISIE PAR LES DONNÉES, jamais par une constante :
 * la première édition de conférence qui n'est pas terminée, à défaut la plus
 * récente. Le jour où la COP32 est annoncée, l'accueil la suit sans qu'on
 * touche au code.
 *
 * LE FRAGMENT EST CONSERVÉ : `/#programmation` doit atteindre la programmation
 * de l'édition en cours, sans quoi les entrées de la barre de navigation
 * mèneraient au haut de la page.
 */

const api = useApi()
const route = useRoute()
const localePath = useLocalePath()
const { t } = useI18n()

const { data: target } = await useAsyncData('home-current-edition', async () => {
  const editions = await api.events.publicList()
  if (!editions.length) return null

  const now = Date.now()
  // `publicList()` rend les éditions de la plus récente à la plus ancienne.
  const upcoming = [...editions]
    .filter((edition) => edition.has_pavilion && Date.parse(edition.ends_at) >= now)
    .sort((a, b) => a.starts_at.localeCompare(b.starts_at))

  return (upcoming[0] ?? editions[0])?.slug ?? null
})

if (target.value) {
  await navigateTo(
    { path: localePath(`/evenements/${target.value}`), hash: route.hash, query: route.query },
    { replace: true, redirectCode: 302 },
  )
}
</script>

<template>
  <!-- Rendu seulement si aucune édition n'est publique — une plateforme
       fraîchement installée, ou toutes les éditions en brouillon. Une
       redirection vers une page inexistante serait pire que cet écran. -->
  <UiEmptyState
    icon="calendar"
    :title="t('event.public.noEdition.title')"
    :description="t('event.public.noEdition.description')"
  />
</template>
