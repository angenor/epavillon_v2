<script setup lang="ts">
import type { PublicSessionMeeting } from '~/types/live'

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
const api = useApi()
const { t } = useI18n()
const { tr } = useI18nText()
const { date, timeWithZone } = useDateTime()
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

const meeting = ref<PublicSessionMeeting | null>(null)
const meetingLoaded = ref(false)
const now = ref<number | null>(null)
let clock: ReturnType<typeof setInterval> | undefined

async function loadMeeting(): Promise<void> {
  const current = session.value
  if (!current) return
  meeting.value = current.is_streamed ? await api.meetings.forSession(current.id).catch(() => null) : null
  meetingLoaded.value = true
}

onMounted(() => {
  now.value = Date.now()
  clock = setInterval(() => (now.value = Date.now()), 30_000)
  void loadMeeting()
})
onBeforeUnmount(() => clearInterval(clock))
watch(session, () => void loadMeeting())

const open = computed(() => isMeetingOpen(meeting.value, now.value))
const notYet = computed(() => meeting.value !== null && now.value !== null && now.value < Date.parse(meeting.value.opens_at))
const closedDescription = computed(() => {
  const current = meeting.value
  if (!current || !session.value) return t('activity.room.closed.none')
  if (notYet.value) {
    return t('activity.room.closed.later', {
      date: date(current.opens_at, session.value.timezone),
      time: timeWithZone(current.opens_at, session.value.timezone, zoneLabel.value),
    })
  }
  return t('activity.room.closed.over')
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
          <ActivityZoomRoom class="absolute! inset-0" mode="page" :session="session" :meeting="meeting" :leave-url="leaveUrl" :zone-label="zoneLabel" />
        </div>

        <UiEmptyState
          v-else
          icon="video"
          :title="t('activity.room.closed.title')"
          :description="closedDescription"
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
