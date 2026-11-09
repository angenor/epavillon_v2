<script setup lang="ts">
/**
 * LE LIEN DU DIRECT D'UNE ACTIVITÉ, POUR QUI S'EST INSCRIT SANS COMPTE : celui
 * du courriel. L'adresse de cette page est fixe : les courriels de l'API y mènent.
 */

definePageMeta({ layout: 'public' })
defineI18nRoute({
  paths: { fr: '/programmations/[edition]/[session]/direct', en: '/programmes/[edition]/[session]/live' },
})

const route = useRoute()
const api = useApi()
const { t } = useI18n()
const { tr } = useI18nText()
const localePath = useLocalePath()
const { dayLong, time, zoneLabel } = useDateTime()
const k = (key: string, params: Record<string, unknown> = {}) => t(`activity.live.${key}`, params)

const { data, status, error, refresh } = await useActivityDetail()
const session = computed(() => data.value?.detail.session ?? null)
const dayStyle = trackDayStyle(null)
const activityTo = computed(() =>
  localePath({ name: 'activity-edition-session', params: { edition: String(route.params.edition), session: String(route.params.session) } }),
)
const requestAccess = (token: string) => api.meetings.access(session.value?.id ?? '', token)

const when = computed(() => {
  const current = session.value
  if (!current) return ''
  return k('when', {
    day: dayLong(current.starts_at, current.timezone),
    start: time(current.starts_at, current.timezone),
    end: time(current.ends_at, current.timezone),
    zone: zoneLabel(current.timezone, data.value?.edition.city ?? undefined),
  })
})

useHead(() => ({
  title: session.value ? k('title', { title: tr(session.value.title) }) : t('activity.notFound.title'),
  meta: [
    { name: 'robots', content: 'noindex' },
    { name: 'referrer', content: 'no-referrer' },
  ],
}))
</script>

<template>
  <div data-direction="condensee" class="min-h-[60vh] font-sans text-text" :style="dayStyle">
    <div class="mx-auto max-w-xl">
      <UiLoadingState v-if="status === 'pending' && !session" :label="t('activity.loading')" variant="form" :lines="3" />

      <UiErrorState v-else-if="error" :title="t('activity.error.title')" :description="t('activity.error.description')" @retry="refresh()" />

      <UiEmptyState
        v-else-if="!session"
        icon="calendar"
        :title="t('activity.notFound.title')"
        :description="t('activity.notFound.description')"
        :action-label="k('back')"
        :action-to="localePath('programme')"
      />

      <article v-else class="overflow-hidden rounded-lg bg-surface-raised shadow-lg">
        <header class="bg-(--color-day-solid) px-5 py-4 text-(--color-day-on-solid) sm:px-6">
          <p class="font-condensed text-[0.95rem] font-semibold tracking-[0.05em] uppercase opacity-85">{{ k('kicker') }}</p>
          <h1 class="mt-0.5 font-condensed text-[1.9rem] leading-[1.05] font-bold text-balance text-inherit">{{ tr(session.title) }}</h1>
          <p v-if="when" class="mt-1.5 text-sm first-letter:uppercase">{{ when }}</p>
        </header>

        <EventLiveLinkEntry messages="activity.live" :ready="session !== null" :request-access="requestAccess" :back-to="activityTo" />
      </article>
    </div>
  </div>
</template>
