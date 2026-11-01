<script setup lang="ts">
/**
 * PAGE PUBLIQUE D'UNE ÉDITION, écran A3.
 *
 * ── L'ORDRE EST LE CONTENU ──────────────────────────────────────────────────
 *
 * Une organisation qui arrive ici veut savoir si elle peut encore déposer, et
 * jusqu'à quand. Tout ce qui retarde cette réponse la fait partir.
 *
 *   1. DE QUOI S'AGIT-IL ?                 le bandeau : titre, quatre faits
 *   2. QUE PUIS-JE FAIRE, ET JUSQU'À QUAND ? la colonne de droite : le billet
 *                                           de l'appel, puis les échéances
 *   3. DE QUOI PARLE-T-ON ?                 la présentation, le direct, la
 *                                           programmation, les journées
 *   4. À QUELLES CONDITIONS ?               conditions et critères de l'appel
 *
 * ── DIRECTION CONDENSÉE (01/11) ─────────────────────────────────────────────
 *
 * Maquette : docs/maquettes/direction-condensee/evenement.html. Le bandeau dit
 * de quoi il s'agit (titre, quatre faits) ; le billet de l'appel, dans la
 * colonne de droite, remonte de 72 px sur le bandeau et reste COLLÉ sur grand
 * écran avec les échéances : on lit la page sans perdre de vue le temps qui
 * reste. Il porte le seul bouton principal de la page. Sous `lg`, la colonne
 * suit le bandeau, sans chevauchement.
 *
 * LA PROGRAMMATION N'EST PAS ICI. Elle a sa page, parce qu'elle porte un
 * sélecteur d'édition : présentée sous le titre de la COP31, elle affichait
 * aussi le programme du cycle PACO. Il ne reste qu'un renvoi, qui part avec
 * l'édition de cette page déjà sélectionnée.
 *
 * ── CE QUE LA PAGE CHARGE, ET POURQUOI EN UNE FOIS ──────────────────────────
 *
 * Dix lectures, groupées en un seul `useAsyncData` : l'édition, sa série, sa
 * bannière, son pays, son appel, les critères de cet appel, ses journées
 * spéciales et leurs photos, la liste des éditions publiques, et les séances
 * publiées. Elles
 * partent en parallèle et le rendu serveur les attend toutes: un écran qui se
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
 * `defineI18nRoute`. Le chemin du fichier, lui, reste `pages/event/[slug].vue`,
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

    // UNE SEULE VAGUE. Le nom du pays vient de l'édition et la grille vient de
    // l'appel : ni l'un ni l'autre ne demande un aller-retour de plus. Recharger
    // les deux cent quarante-neuf pays pour en nommer un que la réponse porte
    // déjà, c'était une seconde d'attente offerte à chaque visiteur.
    const [series, images, call, tracks, editions, schedule, published] = await Promise.all([
      api.events.series(),
      api.events.images(edition.id),
      api.events.call(edition.id),
      api.events.tracks(edition.id),
      api.events.publicList(),
      api.sessions.schedule(edition.id),
      // Seulement pour la photo des cartes : manquée, elles gardent leur aplat.
      api.sessions.scheduleTracks(edition.id).catch(() => []),
    ])

    return {
      edition,
      series: series.find((entry) => entry.id === edition.series_id) ?? null,
      images,
      country: edition.country_name,
      call,
      criteria: call?.criteria ?? [],
      // Grille éteinte : l'évaluation se fait sur 20, et le site n'en dit rien.
      showCriteria: Boolean(call?.uses_scoring_grid && call.criteria.length > 0),
      // Seuls les fils PUBLIÉS sont montrés : `published_at` est ce qui ouvre la
      // page publique d'une journée spéciale, et une journée en préparation
      // n'engage pas encore l'IFDD.
      tracks: tracks.filter((track) => track.published_at !== null),
      trackCovers: Object.fromEntries(published.map((track) => [track.slug, track.cover])),
      editions,
      schedule,
      // Jours qui portent au moins une activité PUBLIÉE, et non tous les jours
      // ouvrables de l'édition : c'est le volume du programme qu'annonce le
      // renvoi, pas la durée de la conférence, déjà lisible dans l'en-tête.
      programmeDayCount: new Set(
        schedule.map((session) => dayKeyInZone(session.starts_at, edition.timezone)),
      ).size,
    }
  },
  { watch: [slug], lazy: true },
)

/** Nombre d'activités par journée spéciale : dit ce que le fil contient vraiment. */
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

// Le direct se lit dans les séances déjà chargées : aucune lecture de plus.
const { isLive, declareFrom } = useLiveSession()
watch(() => data.value?.schedule, (schedule) => schedule && declareFrom(schedule), { immediate: true })
const liveSession = computed(() => data.value?.schedule.find((session) => isLive(session.id)) ?? null)

const programmeTo = computed(() => {
  const edition = data.value?.edition
  if (!edition) return localePath('/programmations')
  return edition.programme_published_at !== null
    ? localePath({ name: 'programme', query: { edition: edition.slug } })
    : localePath('/programmations')
})

useHead(() => ({
  title: data.value ? tr(data.value.edition.title) : t('event.public.notFound.title'),
}))
</script>

<template>
  <div data-direction="condensee" class="full-bleed -mt-8 -mb-8 bg-surface text-text sm:-mt-10 sm:-mb-10">
    <UiLoadingState v-if="status === 'pending'" :label="t('event.public.loading')">
      <EventPublicSkeleton />
    </UiLoadingState>

    <div v-else-if="error" class="mx-auto max-w-[1280px] px-4 py-10 sm:px-6">
      <UiErrorState
        :title="t('common.states.error.title')"
        :description="t('common.states.error.description')"
        @retry="refresh()"
      />
    </div>

    <!-- Édition inconnue : ce n'est pas une erreur technique, et le dire comme
         telle enverrait le visiteur chercher une panne. -->
    <div v-else-if="!data" class="mx-auto max-w-[1280px] px-4 py-10 sm:px-6">
      <UiEmptyState
        icon="calendar"
        :title="t('event.public.notFound.title')"
        :description="t('event.public.notFound.description')"
        :action-label="t('common.states.notFound.action')"
        :action-to="localePath('/')"
      />
    </div>

    <template v-else>
      <EventHero
        :edition="data.edition"
        :series="data.series"
        :images="data.images"
        :country="data.country ? tr(data.country) : null"
        :has-aside="Boolean(data.call)"
      />

      <div class="mx-auto max-w-[1280px] px-4 pb-16 sm:px-6 lg:pb-22">
        <!-- La colonne de droite vient d'abord dans le flux : sous `lg`, l'appel
             et ses échéances suivent le bandeau. La marge négative n'existe qu'à
             partir de `lg` : sur un élément de grille, elle raccourcirait sa
             ligne et ferait remonter la suite dessous. -->
        <div
          class="grid gap-10 pt-6 lg:items-start lg:gap-12 lg:pt-0"
          :class="data.call ? 'lg:grid-cols-[minmax(0,1fr)_380px]' : ''"
        >
          <aside
            v-if="data.call"
            class="relative z-10 flex flex-col gap-4 lg:sticky lg:top-[calc(var(--nav-height)+1rem)] lg:col-start-2 lg:row-start-1 lg:-mt-[72px]"
            :aria-label="t('event.public.aside')"
          >
            <EventHeroCall
              :call="data.call"
              :edition="data.edition"
              :submit-to="localePath('proposal-form')"
              :programme-to="programmeTo"
              :programme-published="data.edition.programme_published_at !== null"
              :criteria-href="data.showCriteria ? '#criteres' : null"
            />
            <EventMilestones :edition="data.edition" :call="data.call" />
          </aside>

          <div class="flex min-w-0 flex-col gap-14 lg:col-start-1 lg:row-start-1 lg:pt-14">
            <EventPresentation :edition="data.edition" />

            <EventProgrammeLiveLead
              v-if="liveSession"
              :session="liveSession"
              :edition-slug="data.edition.slug"
              :timezone="data.edition.timezone"
              :edition-image="editionPhoto(data.images)"
              compact
            />

            <EventProgrammeLink
              :edition="data.edition"
              :to="programmeTo"
              :schedule="data.schedule"
              :tracks="data.tracks"
              :session-count="data.schedule.length"
              :day-count="data.programmeDayCount"
            />

            <EventSpecialDays
              :tracks="data.tracks"
              :edition-slug="data.edition.slug"
              :timezone="data.edition.timezone"
              :session-counts="trackCounts"
              :covers="data.trackCovers"
            />

            <EventCallDetails :call="data.call" :edition="data.edition" />

            <EventCriteria v-if="data.showCriteria" :criteria="data.criteria" :call="data.call" />
          </div>
        </div>
      </div>
    </template>
  </div>
</template>
