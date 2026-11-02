<script setup lang="ts">
import type { MeetingAccess } from '~/types/live'

/**
 * LE LIEN DU DIRECT D'UNE JOURNÉE, POUR QUI S'EST INSCRIT SANS COMPTE : celui
 * du courriel. La page demande l'accès avec le jeton et renvoie aussitôt au
 * lien Zoom personnel. Pas encore prêt (409) : elle attend et réessaie seule.
 *
 * Le jeton se lit dans l'adresse, dans le navigateur seulement, et part en
 * en-tête. L'adresse de cette page est fixe : les courriels de l'API y mènent.
 */

definePageMeta({ layout: 'public' })
defineI18nRoute({
  paths: { fr: '/programmations/[edition]/journees/[track]/direct', en: '/programmes/[edition]/days/[track]/live' },
})

const route = useRoute()
const api = useApi()
const { t } = useI18n()
const { tr } = useI18nText()
const localePath = useLocalePath()
const { dayLong, time, zoneLabel } = useDateTime()
const k = (key: string, params: Record<string, unknown> = {}) => t(`track.live.${key}`, params)

const editionSlug = computed(() => String(route.params.edition))
const trackSlug = computed(() => String(route.params.track))

const { data: page, status: pageStatus, error: pageError, refresh: refreshPage } = await useAsyncData(
  () => `journee-direct-${editionSlug.value}-${trackSlug.value}`,
  () => api.sessions.trackPage(editionSlug.value, trackSlug.value),
)
const dayStyle = computed(() => trackDayStyle(page.value?.track.kind === 'special_day' ? page.value.track.color_hex : null))
const dayTo = computed(() => localePath({ name: 'track-edition-track', params: { edition: editionSlug.value, track: trackSlug.value } }))
const manageTo = computed(() =>
  localePath({
    name: 'track-edition-track-my-registration',
    params: { edition: editionSlug.value, track: trackSlug.value },
    query: { jeton: token.value ?? undefined },
  }),
)

type State = 'checking' | 'waiting' | 'redirecting' | 'invalid' | 'unregistered' | 'dayCancelled' | 'error'
const state = ref<State>('checking')
const message = ref<string | null>(null)
const token = ref<string | null>(null)
const joinUrl = ref<string | null>(null)
const retryIn = ref(0)

const RETRY_S = 20
let ticker: ReturnType<typeof setInterval> | undefined

function wait(): void {
  state.value = 'waiting'
  retryIn.value = RETRY_S
  clearInterval(ticker)
  ticker = setInterval(() => {
    retryIn.value -= 1
    if (retryIn.value <= 0) void attempt()
  }, 1000)
}

function go(access: MeetingAccess): void {
  if (!access.join_url) {
    message.value = k('noLink')
    state.value = 'error'
    return
  }
  joinUrl.value = access.join_url
  state.value = 'redirecting'
  window.location.replace(access.join_url)
}

async function attempt(): Promise<void> {
  clearInterval(ticker)
  const trackId = page.value?.track.id
  if (!token.value || !trackId) {
    state.value = 'invalid'
    return
  }
  state.value = 'checking'
  message.value = null
  try {
    go(await api.meetings.trackAccess(trackId, token.value))
  } catch (error) {
    if (error instanceof ApiRequestError) {
      if (error.code === 'LIVE_MEETING_NOT_READY') return wait()
      if (error.status === 404 || error.status === 401) {
        state.value = 'invalid'
        return
      }
      if (error.code === 'LIVE_MEETING_REGISTRATION_REQUIRED') {
        message.value = error.message
        state.value = 'unregistered'
        return
      }
    }
    // La réunion de la journée annulée : on le dit plutôt que l'erreur brute.
    const meeting = await api.meetings.forTrack(trackId).catch(() => null)
    if (meeting?.status === 'cancelled') {
      state.value = 'dayCancelled'
      return
    }
    message.value = apiErrorMessage(error, t)
    state.value = 'error'
  }
}

onMounted(() => {
  const value = route.query.jeton
  token.value = typeof value === 'string' && value.trim() ? value.trim() : null
  if (page.value) void attempt()
})
watch(page, (value, previous) => {
  if (value && !previous && import.meta.client) void attempt()
})
onBeforeUnmount(() => clearInterval(ticker))

const when = computed(() => {
  const current = page.value
  const start = current?.opens_at ?? current?.starts_at
  if (!current || !start || !current.ends_at) return ''
  const zone = current.edition.timezone
  return k('when', {
    day: dayLong(start, zone),
    start: time(start, zone),
    end: time(current.ends_at, zone),
    zone: zoneLabel(zone, current.edition.city ?? undefined),
  })
})

useHead(() => ({
  title: page.value ? k('title', { title: tr(page.value.track.title) }) : t('track.notFound.title'),
  meta: [
    { name: 'robots', content: 'noindex' },
    { name: 'referrer', content: 'no-referrer' },
  ],
}))
</script>

<template>
  <div data-direction="condensee" class="min-h-[60vh] font-sans text-text" :style="dayStyle">
    <div class="mx-auto max-w-xl">
      <UiLoadingState v-if="pageStatus === 'pending' && !page" :label="t('track.loading')" variant="form" :lines="3" />

      <UiErrorState v-else-if="pageError" :title="t('track.error.title')" :description="t('track.error.description')" @retry="refreshPage()" />

      <UiEmptyState
        v-else-if="!page"
        icon="calendar"
        :title="t('track.notFound.title')"
        :description="t('track.notFound.description')"
        :action-label="t('track.notFound.back')"
        :action-to="localePath('programme')"
      />

      <article v-else class="overflow-hidden rounded-lg bg-surface-raised shadow-lg">
        <header class="bg-(--color-day-solid) px-5 py-4 text-(--color-day-on-solid) sm:px-6">
          <p class="font-condensed text-[0.95rem] font-semibold tracking-[0.05em] uppercase opacity-85">{{ k('kicker') }}</p>
          <h1 class="mt-0.5 font-condensed text-[1.9rem] leading-[1.05] font-bold text-balance text-inherit">{{ tr(page.track.title) }}</h1>
          <p v-if="when" class="mt-1.5 text-sm first-letter:uppercase">{{ when }}</p>
        </header>

        <div class="flex flex-col items-start gap-3 p-5 sm:p-6" aria-live="polite">
          <ClientOnly>
            <p v-if="state === 'checking'" class="flex items-center gap-2 text-sm" role="status">
              <UiSpinner size="1rem" />{{ k('checking') }}
            </p>

            <template v-else-if="state === 'waiting'">
              <p class="font-condensed text-[1.25rem] font-bold tracking-[0.03em] text-warning uppercase">{{ k('waiting.title') }}</p>
              <p class="text-sm leading-relaxed">{{ k('waiting.description') }}</p>
              <p class="text-sm text-text-muted tabular-nums" role="status">{{ t('track.live.waiting.retry', { seconds: retryIn }, retryIn) }}</p>
              <UiButton variant="secondary" icon="refresh" @click="attempt">{{ k('waiting.now') }}</UiButton>
            </template>

            <template v-else-if="state === 'redirecting'">
              <p class="flex items-center gap-2 text-sm" role="status"><UiSpinner size="1rem" />{{ k('redirecting') }}</p>
              <UiButton v-if="joinUrl" variant="live" icon="video" :href="joinUrl">{{ k('open') }}</UiButton>
            </template>

            <template v-else-if="state === 'invalid'">
              <p class="font-condensed text-[1.25rem] font-bold tracking-[0.03em] text-text-secondary uppercase">{{ k('invalid.title') }}</p>
              <p class="text-sm leading-relaxed">{{ k('invalid.description') }}</p>
              <UiButton variant="secondary" :to="dayTo">{{ k('toDay') }}</UiButton>
            </template>

            <template v-else-if="state === 'unregistered'">
              <p class="font-condensed text-[1.25rem] font-bold tracking-[0.03em] text-text-secondary uppercase">{{ k('unregistered') }}</p>
              <p class="text-sm leading-relaxed">{{ message }}</p>
              <UiButton variant="signup" :to="dayTo">{{ k('register') }}</UiButton>
            </template>

            <template v-else-if="state === 'dayCancelled'">
              <p class="font-condensed text-[1.25rem] font-bold tracking-[0.03em] text-danger uppercase">{{ k('dayCancelled.title') }}</p>
              <p class="text-sm leading-relaxed">{{ k('dayCancelled.description') }}</p>
              <UiButton variant="secondary" :to="dayTo">{{ k('toDay') }}</UiButton>
            </template>

            <template v-else>
              <UiAlert intent="danger" :message="message ?? k('error')" />
              <div class="flex flex-wrap gap-x-5 gap-y-2">
                <UiButton icon="refresh" @click="attempt">{{ k('retry') }}</UiButton>
                <UiButton variant="link" :to="manageTo">{{ k('manage') }}</UiButton>
              </div>
            </template>

            <template #fallback>
              <p class="flex items-center gap-2 text-sm"><UiSpinner size="1rem" />{{ k('checking') }}</p>
            </template>
          </ClientOnly>
        </div>
      </article>
    </div>
  </div>
</template>
