<script setup lang="ts">
/**
 * LA SALLE ZOOM EN PLEINE PAGE : celle des téléphones et des tablettes, où Zoom
 * recommande sa vue client. Le SDK y ajoute `#zmmtg-root` au document et prend
 * tout l'écran ; on en sort donc par une navigation complète (lien simple,
 * `leaveUrl`), jamais par le routeur, pour ne rien laisser derrière.
 */

definePageMeta({ layout: false })
defineI18nRoute({
  paths: { fr: '/programmations/[edition]/[session]/salle', en: '/programmes/[edition]/[session]/room' },
})

const route = useRoute()
const { t } = useI18n()
const { tr } = useI18nText()
const localePath = useLocalePath()
const requestUrl = useRequestURL()

const { data, status, error, refresh } = await useActivityDetail()

const edition = computed(() => data.value?.edition ?? null)
const session = computed(() => data.value?.detail.session ?? null)
const activityTo = computed(() =>
  localePath({ name: 'activity-edition-session', params: { edition: String(route.params.edition), session: String(route.params.session) } }),
)
const leaveUrl = computed(() => new URL(activityTo.value, requestUrl.origin).href)
const zoneLabel = computed(() => edition.value?.city ?? undefined)

// Une fois entré, on reste, même si la réunion change d'état sous la personne.
const inMeeting = ref(false)
const { meeting, loaded: meetingLoaded } = useSessionMeeting(session, () => inMeeting.value)
const open = computed(
  () => (session.value?.temporal_state !== 'cancelled' && isMeetingJoinable(meeting.value)) || inMeeting.value,
)

// La vue client laisse `#zmmtg-root` par-dessus le site et la réunion tourne : une navigation interne repart de zéro.
const router = useRouter()
onBeforeRouteLeave((to) => {
  if (!document.getElementById('zmmtg-root')) return true
  window.location.assign(router.resolve(to.fullPath).href)
  return false
})

useHead(() => ({
  title: session.value ? t('activity.room.title', { title: tr(session.value.title) }) : t('activity.notFound.title'),
  meta: [{ name: 'robots', content: 'noindex' }],
}))
</script>

<template>
  <div class="flex min-h-dvh flex-col bg-surface font-sans text-text">
    <header class="border-b border-border-subtle bg-surface-raised">
      <div class="mx-auto flex max-w-[1280px] items-center gap-3 px-4 py-2 sm:px-6">
        <a
          :href="activityTo"
          class="inline-flex min-h-(--target-min) shrink-0 items-center gap-2 rounded-md text-sm font-bold text-text-link"
        >
          <UiIcon name="arrow-left" size="1.125rem" />
          {{ t('activity.room.back') }}
        </a>
        <h1 v-if="session" class="min-w-0 flex-1 truncate text-right text-sm font-bold sm:text-base">
          {{ tr(session.title) }}
        </h1>
      </div>
    </header>

    <main class="mx-auto flex w-full max-w-[1280px] flex-1 flex-col px-4 py-6 sm:px-6">
      <UiLoadingState v-if="status === 'pending' && !data" :label="t('activity.loading')">
        <UiSkeletonLoader height="20rem" />
      </UiLoadingState>

      <UiErrorState
        v-else-if="error"
        :title="t('activity.error.title')"
        :description="t('activity.error.description')"
        @retry="refresh()"
      />

      <UiEmptyState
        v-else-if="!session || !edition"
        icon="calendar"
        :title="t('activity.notFound.title')"
        :description="t('activity.notFound.description')"
        :action-label="t('activity.back')"
        :action-to="localePath('programme')"
      />

      <ClientOnly v-else>
        <UiLoadingState v-if="!meetingLoaded" :label="t('activity.room.checking')">
          <UiSkeletonLoader height="20rem" />
        </UiLoadingState>

        <div v-else-if="open && meeting" class="relative min-h-[26rem] flex-1 overflow-hidden rounded-lg shadow-lg">
          <ActivityZoomRoom
            class="absolute! inset-0"
            mode="page"
            :session="session"
            :meeting="meeting"
            :leave-url="leaveUrl"
            :zone-label="zoneLabel"
            @update:in-meeting="(value: boolean) => (inMeeting = value)"
          />
        </div>

        <UiEmptyState
          v-else
          icon="video"
          :title="t('activity.room.none.title')"
          :description="t('activity.room.none.description')"
          :action-label="t('activity.room.back')"
          :action-to="activityTo"
        />

        <template #fallback>
          <UiSkeletonLoader height="20rem" />
        </template>
      </ClientOnly>
    </main>
  </div>
</template>
