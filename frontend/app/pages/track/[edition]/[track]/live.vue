<script setup lang="ts">
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
    query: { jeton: typeof route.query.jeton === 'string' ? route.query.jeton : undefined },
  }),
)

const trackId = computed(() => page.value?.track.id ?? null)
const requestAccess = (token: string) => api.meetings.trackAccess(trackId.value ?? '', token)
async function isCancelled(): Promise<boolean> {
  if (!trackId.value) return false
  return (await api.meetings.forTrack(trackId.value).catch(() => null))?.status === 'cancelled'
}

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

        <EventLiveLinkEntry messages="track.live" :ready="trackId !== null" :request-access="requestAccess" :is-cancelled="isCancelled" :back-to="dayTo">
          <template #error-actions>
            <UiButton variant="link" :to="manageTo">{{ k('manage') }}</UiButton>
          </template>
        </EventLiveLinkEntry>
      </article>
    </div>
  </div>
</template>
