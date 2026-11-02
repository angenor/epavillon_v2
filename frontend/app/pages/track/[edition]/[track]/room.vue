<script setup lang="ts">
/**
 * LA SALLE ZOOM D'UNE JOURNÉE, en pleine page : un seul direct de l'ouverture
 * à la clôture, pour qui s'est inscrit à la journée. Même salle que celle d'une
 * activité (`ActivityZoomRoom` en `page`, avec `day-track-id`) ; on en sort
 * par une navigation complète vers la page de la journée.
 *
 * Inscrite sans compte, la personne n'entre pas ici : son lien personnel lui
 * arrive par courriel (`direct.vue`).
 */

definePageMeta({ layout: false })
defineI18nRoute({
  paths: { fr: '/programmations/[edition]/journees/[track]/salle', en: '/programmes/[edition]/days/[track]/room' },
})

const route = useRoute()
const api = useApi()
const { t } = useI18n()
const { tr } = useI18nText()
const localePath = useLocalePath()
const requestUrl = useRequestURL()

const editionSlug = computed(() => String(route.params.edition))
const trackSlug = computed(() => String(route.params.track))

const { data: page, status, error, refresh } = await useAsyncData(
  () => `journee-salle-${editionSlug.value}-${trackSlug.value}`,
  () => api.sessions.trackPage(editionSlug.value, trackSlug.value),
)

const dayTo = computed(() => localePath({ name: 'track-edition-track', params: { edition: editionSlug.value, track: trackSlug.value } }))
const leaveUrl = computed(() => new URL(dayTo.value, requestUrl.origin).href)
const title = computed(() => (page.value ? tr(page.value.track.title) : ''))
const trackId = computed(() => page.value?.track.id ?? null)
const dayStyle = computed(() => trackDayStyle(page.value?.track.kind === 'special_day' ? page.value.track.color_hex : null))

// Une fois entré, on reste, même si la réunion change d'état sous la personne.
const inMeeting = ref(false)
const { meeting, loaded: meetingLoaded } = useTrackMeeting(
  () => (page.value ? { id: page.value.track.id, eventId: page.value.edition.id } : null),
  () => inMeeting.value,
)
const open = computed(() => isMeetingJoinable(meeting.value) || inMeeting.value)

const day = useDayRegistration(trackId)
const { dialogOpen, highlight } = day
const dialogTarget = computed(() => (page.value ? { page: page.value, registration: day.mine.value, highlight: highlight.value } : null))

// La vue client laisse `#zmmtg-root` par-dessus le site et la réunion tourne : une navigation interne repart de zéro.
const router = useRouter()
onBeforeRouteLeave((to) => {
  if (!document.getElementById('zmmtg-root')) return true
  window.location.assign(router.resolve(to.fullPath).href)
  return false
})

useHead(() => ({
  title: title.value ? t('track.room.title', { title: title.value }) : t('track.notFound.title'),
  meta: [{ name: 'robots', content: 'noindex' }],
}))
</script>

<template>
  <div class="flex min-h-dvh flex-col bg-surface font-sans text-text" :style="dayStyle">
    <header class="border-b border-border-subtle bg-surface-raised">
      <div class="mx-auto flex max-w-[1280px] items-center gap-3 px-4 py-2 sm:px-6">
        <a :href="dayTo" class="inline-flex min-h-(--target-min) shrink-0 items-center gap-2 rounded-md text-sm font-bold text-text-link">
          <UiIcon name="arrow-left" size="1.125rem" />
          {{ t('track.room.back') }}
        </a>
        <h1 v-if="page" class="min-w-0 flex-1 truncate text-right text-sm font-bold sm:text-base">{{ title }}</h1>
      </div>
    </header>

    <main class="mx-auto flex w-full max-w-[1280px] flex-1 flex-col px-4 py-6 sm:px-6">
      <UiLoadingState v-if="status === 'pending' && !page" :label="t('track.loading')">
        <UiSkeletonLoader height="20rem" />
      </UiLoadingState>

      <UiErrorState v-else-if="error" :title="t('track.error.title')" :description="t('track.error.description')" @retry="refresh()" />

      <UiEmptyState
        v-else-if="!page"
        icon="calendar"
        :title="t('track.notFound.title')"
        :description="t('track.notFound.description')"
        :action-label="t('track.notFound.back')"
        :action-to="localePath('programme')"
      />

      <ClientOnly v-else>
        <UiLoadingState v-if="!meetingLoaded" :label="t('track.room.checking')">
          <UiSkeletonLoader height="20rem" />
        </UiLoadingState>

        <div v-else-if="open && meeting" class="relative min-h-[26rem] flex-1 overflow-hidden rounded-lg shadow-lg">
          <ActivityZoomRoom
            class="absolute! inset-0"
            mode="page"
            :day-track-id="page.track.id"
            :meeting="meeting"
            :leave-url="leaveUrl"
            :zone-label="page.edition.city ?? undefined"
            :auto-enter="route.query.entrer === '1'"
            @update:in-meeting="(value: boolean) => (inMeeting = value)"
          />
        </div>

        <UiEmptyState
          v-else
          icon="video"
          :title="t('track.room.none.title')"
          :description="t('track.room.none.description')"
          :action-label="t('track.room.back')"
          :action-to="dayTo"
        />

        <ActivityRegistrationDialog
          v-if="dialogTarget"
          v-model:open="dialogOpen"
          :day="dialogTarget"
          :timezone="page.edition.timezone"
          :timezone-label="page.edition.city ?? undefined"
          @day-registered="day.registeredWith"
          @day-updated="day.updated"
        />

        <template #fallback>
          <UiSkeletonLoader height="20rem" />
        </template>
      </ClientOnly>
    </main>
  </div>
</template>
