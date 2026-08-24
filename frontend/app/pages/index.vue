<script setup lang="ts">
import type { EditionPeriod } from '~/types/home'

/**
 * ACCUEIL PUBLIC — `/`.
 *
 * ── LA REDIRECTION DU 17/08 EST RÉVOQUÉE ────────────────────────────────────
 *
 * Cette page redirigeait vers `/evenements/<slug>`. L'argument tenait : la page
 * d'une édition répondait déjà aux questions du visiteur, et deux mises en page
 * presque identiques divergent en six mois.
 *
 * Ce qui l'a périmé, c'est que la plateforme porte désormais PLUSIEURS éditions
 * vivantes en même temps — une COP, un cycle de webinaires, une édition
 * régionale — plus une vitrine éditoriale administrable (`content.highlights`)
 * qui n'appartient à aucune d'elles. Rediriger vers une seule édition, c'était
 * choisir pour le visiteur et rendre les autres invisibles.
 *
 * DEUX CHOSES SURVIVENT À LA RÉVOCATION, et ce sont les deux qu'il aurait été
 * facile de perdre :
 *   · l'ancre `#appel-a-propositions`, câblée dans le pied de page de CHAQUE
 *     écran (`layouts/public.vue`) — voir `HomeCallSection`, qui la rend même
 *     lorsqu'aucun appel n'est ouvert ;
 *   · le choix de l'édition en cours PAR LES DONNÉES. Il n'est plus fait ici :
 *     `api.home.screen()` rend `currentEdition` déjà choisie, selon la même
 *     règle qu'appliquait cette page — première édition à pavillon non terminée,
 *     à défaut la plus récente.
 *
 * `/evenements/<slug>` reste l'adresse canonique d'une édition ; l'accueil y
 * conduit, depuis le bandeau, le panneau, l'appel et l'historique.
 *
 * ── PAS DE `defineI18nRoute` ────────────────────────────────────────────────
 *
 * La racine n'est pas traduite : `/` et `/en` mènent au même écran. Lui donner
 * des chemins localisés créerait une seconde adresse d'accueil, que rien ne
 * référence.
 *
 * ── UNE SEULE REQUÊTE ───────────────────────────────────────────────────────
 *
 * `api.home.screen()` rend le bandeau, les épingles, les prochaines séances,
 * l'historique complet, les volumes de programme et l'édition en cours. Quatre
 * vues du modèle assemblées côté client, ce seraient quatre états de chargement
 * à composer et surtout quatre instants de mesure : un bandeau annonçant un
 * appel ouvert pendant que la section d'appel le dit clos n'est pas un défaut
 * d'affichage, c'est une réponse assemblée en quatre fois.
 *
 * Le filtre de l'historique ne coûte donc aucune requête : il recompose ce qui
 * est déjà en main (`utils/edition-history.ts`).
 */

const api = useApi()
const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const localePath = useLocalePath()

const { data, status, error, refresh } = await useAsyncData('home-screen', () => api.home.screen())

/**
 * L'ÉTAT DU FILTRE VIT DANS L'URL, en français comme partout ailleurs
 * (`?periode=a-venir`). Il se partage, il survit au retour arrière, et il est
 * rendu par le serveur — un onglet choisi dans une `ref` ne fait rien de tout
 * cela.
 */
const period = computed<EditionPeriod>(() => periodFromQuery(route.query.periode))

function setPeriod(next: EditionPeriod): void {
  const query = { ...route.query }
  const value = queryForPeriod(next)
  if (value === null) delete query.periode
  else query.periode = value
  void router.replace({ query })
}

/**
 * L'accueil est public : un refus de périmètre n'y a rien à faire. On le traite
 * quand même — c'est le quatrième état exigé de tout écran, et une API mal
 * configurée qui répondrait 403 doit le DIRE plutôt que se taire.
 */
const isForbidden = computed(() => isForbiddenError(error.value))

useHead(() => ({
  title: t('home.head.title'),
  meta: [{ name: 'description', content: t('home.head.description') }],
}))
</script>

<template>
  <div class="flex flex-col gap-16">
    <!--
      LE TITRE DE LA PAGE EST INVISIBLE, ET C'EST UN CHOIX.

      Un document a besoin d'un `h1` stable. Le seul texte de grande taille de
      cet écran est une citation éditoriale qui change toutes les sept secondes :
      en faire le titre du document donnerait une page dont le nom dépend de
      l'instant où on la charge. Chaque section porte en revanche son `h2`
      visible, et c'est par eux qu'on navigue.
    -->
    <h1 class="sr-only">{{ t('home.title') }}</h1>

    <UiLoadingState
      v-if="status === 'pending'"
      variant="card"
      :lines="3"
      :label="t('home.loading')"
    />

    <UiForbiddenState v-else-if="isForbidden" :action-to="localePath('/programmations')" />

    <UiErrorState
      v-else-if="error"
      :title="t('home.error.title')"
      :description="t('home.error.description')"
      @retry="refresh()"
    />

    <!-- Une plateforme fraîchement installée, ou tout en brouillon. Ce n'est pas
         une panne, et le dire comme telle enverrait chercher un problème qui
         n'existe pas. -->
    <UiEmptyState
      v-else-if="!data"
      icon="calendar"
      :title="t('home.empty.title')"
      :description="t('home.empty.description')"
    />

    <template v-else>
      <HomeShowcase :slides="data.hero">
        <template #aside>
          <HomeAsidePanel
            :sessions="data.upcomingSessions"
            :editions="data.editions"
            :stats="data.stats"
            :now="data.generated_at"
          />
        </template>
      </HomeShowcase>

      <HomeCallSection :edition="data.currentEdition" />

      <HomeEditionHistory
        :editions="data.editions"
        :stats="data.stats"
        :period="period"
        @update:period="setPeriod"
      />
    </template>
  </div>
</template>
