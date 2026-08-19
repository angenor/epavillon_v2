<script setup lang="ts">
/**
 * PAGE PUBLIQUE D'UNE ÉDITION — écran A3.
 *
 * Elle répond à quatre questions, DANS CET ORDRE, et l'ordre est le contenu :
 *
 *   1. DE QUOI S'AGIT-IL ?        l'en-tête — titre, dates, lieu, mode, visuel
 *   2. QUELLES ÉCHÉANCES ?        la frise des jalons, puis l'encart d'appel
 *   3. QUE PUIS-JE FAIRE ?        déposer un dossier, lire les critères
 *   4. OÙ CELA SE PASSE-T-IL ?    les journées spéciales, le lien programmation
 *
 * Une organisation qui arrive ici veut savoir si elle peut encore déposer, et
 * jusqu'à quand. Tout ce qui retarde cette réponse — un long texte de
 * présentation, un carrousel — la fait partir. LES QUATRE DATES VIENNENT DONC EN
 * PREMIER : la frise des jalons tient en cent pixels et répond d'un coup d'œil à
 * « où en est-on », avant même l'encart d'appel qui, lui, développe.
 *
 * LA PROGRAMMATION N'EST PLUS ICI. Elle a sa page, parce qu'elle porte un
 * sélecteur d'édition : présentée sous le titre de la COP31, elle affichait aussi
 * le programme du cycle PACO. Il ne reste qu'un renvoi, qui part avec l'édition
 * de cette page déjà sélectionnée.
 *
 * ── CE QUE LA PAGE CHARGE, ET POURQUOI EN UNE FOIS ──────────────────────────
 *
 * Neuf lectures, groupées en un seul `useAsyncData` : l'édition, sa série, sa
 * bannière, son pays, son appel, les critères de cet appel, ses journées
 * spéciales, la liste des éditions publiques, et les séances publiées. Elles
 * partent en parallèle et le rendu serveur les attend toutes — un écran qui se
 * remplit par morceaux fait sauter la mise en page trois fois de suite.
 *
 * Les séances ne sont plus affichées ici, mais elles restent nécessaires : ce
 * sont elles qui donnent le nombre d'activités rattachées à chaque journée
 * spéciale, et le volume annoncé par le renvoi vers la programmation. Les salles,
 * en revanche, ne servaient qu'aux filtres et ne sont plus chargées.
 *
 * Le jour où l'API répond, la moitié de ces appels disparaîtront : la bannière
 * et le pays de l'hôte appartiennent à la réponse de `GET /events/:slug`
 * (obligation inscrite au prompt B3). Les autres restent des ressources
 * distinctes, et c'est très bien ainsi.
 *
 * ── L'ADRESSE EST DANS LA LANGUE, LE FICHIER SUIT LA CONVENTION ─────────────
 *
 * `/evenements/cop31-belem-2027` et `/en/events/cop31-belem-2027`, par
 * `defineI18nRoute`. Le chemin du fichier, lui, reste `pages/event/[slug].vue` —
 * même règle qu'aux écrans d'authentification.
 */

definePageMeta({ layout: 'public' })
defineI18nRoute({ paths: { fr: '/evenements/[slug]', en: '/events/[slug]' } })

const route = useRoute()
const api = useApi()
const localePath = useLocalePath()
const { t } = useI18n()
const { tr } = useI18nText()

const slug = computed(() => String(route.params.slug ?? ''))

const { data, status, error, refresh } = await useAsyncData(
  'event-public-page',
  async () => {
    const edition = await api.events.bySlug(slug.value)
    if (!edition) return null

    const [series, images, countries, call, tracks, editions, schedule] = await Promise.all([
      api.events.series(),
      api.events.images(edition.id),
      api.reference.countries(),
      api.events.call(edition.id),
      api.events.tracks(edition.id),
      api.events.publicList(),
      api.sessions.schedule(edition.id),
    ])

    // Les critères dépendent de l'appel : deuxième vague, et seulement s'il y en
    // a un. Une édition sans pavillon n'ouvre pas d'appel (règle métier n° 5).
    const criteria = call ? await api.calls.criteria(call.id) : []

    return {
      edition,
      series: series.find((entry) => entry.id === edition.series_id) ?? null,
      images,
      country: countries.find((entry) => entry.id === edition.country_id)?.name ?? null,
      call,
      criteria,
      // Seuls les fils PUBLIÉS sont montrés : `published_at` est ce qui ouvre la
      // page publique d'une journée spéciale, et une journée en préparation
      // n'engage pas encore l'IFDD.
      tracks: tracks.filter((track) => track.published_at !== null),
      editions,
      schedule,
      // Jours qui portent au moins une activité PUBLIÉE — et non tous les jours
      // ouvrables de l'édition : c'est le volume du programme qu'annonce le
      // renvoi, pas la durée de la conférence, déjà lisible dans l'en-tête.
      programmeDayCount: new Set(
        schedule.map((session) => dayKeyInZone(session.starts_at, edition.timezone)),
      ).size,
    }
  },
  { watch: [slug] },
)

/** Nombre d'activités par journée spéciale — dit ce que le fil contient vraiment. */
const trackCounts = computed<Record<string, number>>(() => {
  const counts: Record<string, number> = {}
  for (const session of data.value?.schedule ?? []) {
    for (const track of session.tracks) {
      const matching = data.value?.tracks.find((entry) => entry.slug === track.slug)
      if (matching) counts[matching.id] = (counts[matching.id] ?? 0) + 1
    }
  }
  return counts
})

useHead(() => ({
  title: data.value ? tr(data.value.edition.title) : t('event.public.notFound.title'),
}))
</script>

<template>
  <div class="flex flex-col gap-12">
    <UiLoadingState
      v-if="status === 'pending'"
      variant="card"
      :lines="3"
      :label="t('event.public.loading')"
    />

    <UiErrorState
      v-else-if="error"
      :title="t('common.states.error.title')"
      :description="t('common.states.error.description')"
      @retry="refresh()"
    />

    <!-- Édition inconnue : ce n'est pas une erreur technique, et le dire comme
         telle enverrait le visiteur chercher une panne. -->
    <UiEmptyState
      v-else-if="!data"
      icon="calendar"
      :title="t('event.public.notFound.title')"
      :description="t('event.public.notFound.description')"
      :action-label="t('common.states.notFound.action')"
      :action-to="localePath('/')"
    />

    <template v-else>
      <EventHero
        :edition="data.edition"
        :series="data.series"
        :images="data.images"
        :country="data.country ? tr(data.country) : null"
      />

      <!-- LES ÉCHÉANCES D'ABORD. Quatre dates, cent pixels : c'est la première
           chose qu'on vient chercher, et l'encart d'appel qui suit ne fait que
           développer celle qui presse. -->
      <EventMilestones v-if="data.call" :edition="data.edition" :call="data.call" />

      <EventCallBanner
        :call="data.call"
        :edition="data.edition"
        criteria-href="#criteres"
        :submit-to="localePath('proposal-form')"
      />

      <EventSpecialDays
        :tracks="data.tracks"
        :timezone="data.edition.timezone"
        :zone-label="data.edition.city ?? undefined"
        :session-counts="trackCounts"
      />

      <EventProgrammeLink
        :edition="data.edition"
        :session-count="data.schedule.length"
        :day-count="data.programmeDayCount"
      />

      <EventCriteria :criteria="data.criteria" :call="data.call" />
    </template>
  </div>
</template>
