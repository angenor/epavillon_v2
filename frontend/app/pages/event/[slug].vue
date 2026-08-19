<script setup lang="ts">
/**
 * PAGE PUBLIQUE D'UNE ÉDITION — écran A3.
 *
 * ── L'ORDRE EST LE CONTENU ──────────────────────────────────────────────────
 *
 * Une organisation qui arrive ici veut savoir si elle peut encore déposer, et
 * jusqu'à quand. Tout ce qui retarde cette réponse la fait partir.
 *
 *   1. DE QUOI S'AGIT-IL, ET QUE PUIS-JE FAIRE ?  le bandeau — titre, faits, et
 *      le PANNEAU D'ACTION : rebours, échéance, dépôt
 *   2. QUELLES ÉCHÉANCES, À QUELLES CONDITIONS ?  la colonne de droite
 *   3. DE QUOI PARLE-T-ON ?                       la présentation, les journées
 *   4. ET APRÈS ?                                 la programmation, les critères
 *
 * ── LE DÉPÔT EST REMONTÉ DANS LE BANDEAU (19/08) ────────────────────────────
 *
 * Il vivait à mi-page, dans l'encart d'appel : sur un portable, le premier écran
 * ne montrait que le titre de la conférence, et il fallait défiler pour trouver
 * la seule chose qu'on vient faire. Le bandeau porte désormais l'action. LA PAGE
 * NE PORTE QU'UN SEUL BOUTON PRINCIPAL, ET IL N'EN EXISTE PAS DE SECOND
 * EXEMPLAIRE : `EventCallDetails` a été allégé d'autant, et garde ce qu'on vient
 * VÉRIFIER — conditions, prolongation, consignes.
 *
 * ── DEUX COLONNES, PARCE QUE LES DEUX SE LISENT ENSEMBLE ────────────────────
 *
 * Les échéances et les conditions de l'appel tiennent dans une colonne COLLANTE
 * à droite : elles restent visibles pendant qu'on lit la présentation et les
 * journées spéciales, c'est-à-dire au moment précis où l'on se demande s'il
 * reste du temps. Sous `lg`, la colonne repasse en tête du flux — l'ordre des
 * priorités ne change pas parce que l'écran rétrécit.
 *
 * LA PROGRAMMATION N'EST PAS ICI. Elle a sa page, parce qu'elle porte un
 * sélecteur d'édition : présentée sous le titre de la COP31, elle affichait
 * aussi le programme du cycle PACO. Il ne reste qu'un renvoi, qui part avec
 * l'édition de cette page déjà sélectionnée.
 *
 * ── CE QUE LA PAGE CHARGE, ET POURQUOI EN UNE FOIS ──────────────────────────
 *
 * Neuf lectures, groupées en un seul `useAsyncData` : l'édition, sa série, sa
 * bannière, son pays, son appel, les critères de cet appel, ses journées
 * spéciales, la liste des éditions publiques, et les séances publiées. Elles
 * partent en parallèle et le rendu serveur les attend toutes — un écran qui se
 * remplit par morceaux fait sauter la mise en page trois fois de suite.
 *
 * Les séances ne sont pas affichées ici, mais elles restent nécessaires : ce
 * sont elles qui donnent le nombre d'activités rattachées à chaque journée
 * spéciale, et le volume annoncé par le renvoi vers la programmation.
 *
 * Le jour où l'API répond, la moitié de ces appels disparaîtront : la bannière
 * et le pays de l'hôte appartiennent à la réponse de `GET /events/:slug`
 * (obligation inscrite au prompt B3).
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
  <div class="flex flex-col">
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
        :has-action="Boolean(data.call)"
      >
        <template #action="{ tone }">
          <EventHeroCall
            :call="data.call"
            :edition="data.edition"
            :tone="tone"
            :submit-to="localePath('proposal-form')"
            criteria-href="#criteres"
          />
        </template>
      </EventHero>

      <div class="mt-12 grid gap-12 lg:grid-cols-12 lg:gap-x-12">
        <!-- LA COLONNE DE DROITE VIENT EN PREMIER DANS LE FLUX. Sous `lg`, les
             échéances et les conditions se lisent avant la présentation : c'est
             l'ordre des priorités, et il ne change pas parce que l'écran
             rétrécit. `lg:order-2` le rétablit visuellement à partir de là. -->
        <aside v-if="data.call" class="lg:order-2 lg:col-span-4">
          <div class="flex flex-col gap-6 lg:sticky lg:top-24">
            <EventMilestones :edition="data.edition" :call="data.call" />
            <EventCallDetails :call="data.call" :edition="data.edition" />
          </div>
        </aside>

        <div
          class="flex flex-col gap-14 lg:order-1"
          :class="data.call ? 'lg:col-span-8' : 'lg:col-span-12'"
        >
          <EventPresentation :edition="data.edition" />

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
        </div>
      </div>
    </template>
  </div>
</template>
