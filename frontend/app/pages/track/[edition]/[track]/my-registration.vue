<script setup lang="ts">
import type { DayRegistration } from '~/types/programme/day-registration'

/**
 * L'INSCRIPTION À UNE JOURNÉE, GÉRÉE PAR LE LIEN DU COURRIEL : pour qui s'est
 * inscrit sans compte. Changer ses activités (la fenêtre d'inscription, en
 * modification), se désinscrire, et le jour même rejoindre le direct.
 *
 * Le jeton se lit dans l'adresse, dans le navigateur seulement, et part en
 * en-tête : il ne passe ni par le rendu serveur ni par un état sérialisé.
 * L'adresse de cette page est fixe : les courriels de l'API y mènent.
 */

definePageMeta({ layout: 'public' })
defineI18nRoute({
  paths: { fr: '/programmations/[edition]/journees/[track]/mon-inscription', en: '/programmes/[edition]/days/[track]/my-registration' },
})

const route = useRoute()
const api = useApi()
const { t } = useI18n()
const { tr } = useI18nText()
const localePath = useLocalePath()
const { dayLong, time, zoneLabel } = useDateTime()
const k = (key: string, params: Record<string, unknown> = {}) => t(`track.my-registration.${key}`, params)

const editionSlug = computed(() => String(route.params.edition))
const trackSlug = computed(() => String(route.params.track))

const { data, status: pageStatus, error: pageError, refresh: refreshPage } = await useAsyncData(
  () => `journee-gestion-${editionSlug.value}-${trackSlug.value}`,
  () => api.sessions.trackPage(editionSlug.value, trackSlug.value),
)
const page = computed(() => data.value ?? null)
const { phase: dayPhase } = useTrackDay(page)
const dayStyle = computed(() => trackDayStyle(page.value?.track.kind === 'special_day' ? page.value.track.color_hex : null))
const dayTo = computed(() => localePath({ name: 'track-edition-track', params: { edition: editionSlug.value, track: trackSlug.value } }))

type State = 'loading' | 'invalid' | 'error' | 'ready'
const state = ref<State>('loading')
const loadError = ref<string | null>(null)
const token = ref<string | null>(null)
const registration = ref<DayRegistration | null>(null)
const firstName = ref('')

async function load(): Promise<void> {
  const value = token.value
  if (!value) {
    state.value = 'invalid'
    return
  }
  state.value = 'loading'
  loadError.value = null
  try {
    const managed = await api.trackRegistrations.manage(value)
    if (!managed) {
      state.value = 'invalid'
      return
    }
    registration.value = managed.registration
    firstName.value = managed.first_name
    state.value = 'ready'
  } catch (error) {
    loadError.value = apiErrorMessage(error, t)
    state.value = 'error'
  }
}

onMounted(() => {
  const value = route.query.jeton
  token.value = typeof value === 'string' && value.trim() ? value.trim() : null
  void load()
})

const cancelled = computed(() => registration.value?.status === 'cancelled')
const kept = computed(() => {
  const value = registration.value
  const current = page.value
  if (!value || !current) return []
  const keep = new Set(value.session_ids)
  return current.agenda.flatMap((entry) =>
    entry.kind === 'activity' && keep.has(entry.activity.session.id) && isPickable(entry.activity) ? [entry.activity] : [],
  )
})
const choice = computed(() => (page.value ? hasActivityChoice(page.value) : false))
const following = computed(() => {
  const value = registration.value
  if (!value || !page.value || !hasActivityChoice(page.value)) return ''
  const count = kept.value.length
  return t('track.my-registration.following', { count, total: value.activity_count }, count)
})

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

const liveTo = computed(() =>
  localePath({ name: 'track-edition-track-live', params: { edition: editionSlug.value, track: trackSlug.value }, query: { jeton: token.value ?? undefined } }),
)

const dialogOpen = ref(false)
const dialogTarget = computed(() =>
  page.value && registration.value ? { page: page.value, registration: registration.value, token: token.value } : null,
)

const confirmOpen = ref(false)
const cancelling = ref(false)
const cancelError = ref<string | null>(null)

async function confirmCancel(): Promise<void> {
  const value = registration.value
  if (!value || cancelling.value) return
  cancelling.value = true
  cancelError.value = null
  try {
    registration.value = await api.trackRegistrations.cancel(value.id, token.value)
    confirmOpen.value = false
  } catch (error) {
    cancelError.value = apiErrorMessage(error, t)
  } finally {
    cancelling.value = false
  }
}

useHead(() => ({
  title: page.value ? k('title', { title: tr(page.value.track.title) }) : t('track.notFound.title'),
  // Le jeton est dans l'adresse : ni indexée, ni transmise aux sites qu'on ouvre d'ici.
  meta: [
    { name: 'robots', content: 'noindex' },
    { name: 'referrer', content: 'no-referrer' },
  ],
}))
</script>

<template>
  <div data-direction="condensee" class="min-h-[60vh] font-sans text-text" :style="dayStyle">
    <div class="mx-auto max-w-2xl">
      <UiLoadingState v-if="pageStatus === 'pending' && !page" :label="t('track.loading')" variant="form" :lines="4" />

      <UiErrorState v-else-if="pageError" :title="t('track.error.title')" :description="t('track.error.description')" @retry="refreshPage()" />

      <UiEmptyState
        v-else-if="!page"
        icon="calendar"
        :title="t('track.notFound.title')"
        :description="t('track.notFound.description')"
        :action-label="t('track.notFound.back')"
        :action-to="localePath('programme')"
      />

      <template v-else>
        <NuxtLink :to="dayTo" class="inline-flex min-h-(--target-min) items-center gap-2 text-sm font-bold text-text-link">
          <UiIcon name="arrow-left" size="1.125rem" />{{ k('back') }}
        </NuxtLink>

        <article class="mt-3 overflow-hidden rounded-lg bg-surface-raised shadow-lg">
          <header class="bg-(--color-day-solid) px-5 py-4 text-(--color-day-on-solid) sm:px-6">
            <p class="font-condensed text-[0.95rem] font-semibold tracking-[0.05em] uppercase opacity-85">{{ k('kicker') }}</p>
            <h1 class="mt-0.5 font-condensed text-[1.9rem] leading-[1.05] font-bold text-balance text-inherit">{{ tr(page.track.title) }}</h1>
            <p v-if="when" class="mt-1.5 text-sm first-letter:uppercase">{{ when }}</p>
          </header>

          <div class="p-5 sm:p-6">
            <ClientOnly>
              <UiLoadingState v-if="state === 'loading'" variant="form" :lines="3" :label="k('loading')" />

              <div v-else-if="state === 'invalid'" class="flex flex-col items-start gap-3">
                <p class="font-condensed text-[1.25rem] font-bold tracking-[0.03em] text-text-secondary uppercase">{{ k('invalid.title') }}</p>
                <p class="text-sm leading-relaxed">{{ k('invalid.description') }}</p>
                <UiButton variant="secondary" :to="dayTo">{{ k('invalid.action') }}</UiButton>
              </div>

              <UiErrorState v-else-if="state === 'error'" compact :title="k('error')" :description="loadError ?? undefined" @retry="load" />

              <div v-else-if="cancelled" class="flex flex-col items-start gap-3">
                <p class="font-condensed text-[1.25rem] font-bold tracking-[0.03em] text-text-secondary uppercase">{{ k('cancelled.title') }}</p>
                <p class="text-sm leading-relaxed">{{ k('cancelled.description') }}</p>
                <UiButton v-if="dayPhase !== 'after'" variant="signup" :to="dayTo">{{ k('cancelled.action') }}</UiButton>
              </div>

              <div v-else-if="registration" class="flex flex-col gap-5">
                <div>
                  <p class="flex items-center gap-2 font-condensed text-[1.25rem] font-bold tracking-[0.03em] text-success uppercase">
                    <UiIcon name="check" size="1.25rem" :stroke-width="2.4" />{{ k('registered', { name: firstName }) }}
                  </p>
                  <p v-if="following" class="mt-1.5 text-sm leading-relaxed">{{ following }}</p>
                </div>

                <div v-if="dayPhase === 'during'" class="rounded-md border border-live-border bg-live-surface p-3.5">
                  <p class="text-sm">{{ k('live.lead') }}</p>
                  <UiButton class="mt-2.5 min-h-12!" block variant="live" icon="video" :to="liveTo">{{ k('live.action') }}</UiButton>
                </div>
                <p v-else-if="dayPhase === 'before'" class="rounded-md bg-surface-sunken p-3 text-sm leading-relaxed">{{ k('live.before') }}</p>

                <section v-if="dayPhase !== 'after'" aria-labelledby="mes-activites-titre">
                  <template v-if="choice">
                    <h2 id="mes-activites-titre" class="font-condensed text-[1.15rem] font-bold tracking-[0.03em] text-heading uppercase">
                      {{ k('activities') }}
                    </h2>
                    <ul v-if="kept.length" class="mt-1">
                      <li
                        v-for="activity in kept"
                        :key="activity.session.id"
                        class="grid grid-cols-[54px_minmax(0,1fr)] gap-x-3 border-t border-border-subtle py-2.5 first:border-t-0"
                      >
                        <time :datetime="activity.session.starts_at" class="font-condensed text-[1.15rem] leading-tight font-bold text-(--color-day) tabular-nums">
                          {{ time(activity.session.starts_at, page.edition.timezone) }}
                        </time>
                        <span class="text-[0.92rem] leading-snug font-semibold">{{ tr(activity.session.title) }}</span>
                      </li>
                    </ul>
                    <p v-else class="mt-1.5 text-sm text-text-muted">{{ k('none') }}</p>
                  </template>
                  <div class="flex flex-wrap items-center gap-x-5 gap-y-2" :class="{ 'mt-4': choice }">
                    <UiButton v-if="choice" variant="secondary" icon="edit" @click="dialogOpen = true">{{ k('edit') }}</UiButton>
                    <UiButton variant="link" @click="confirmOpen = true">{{ k('cancel.action') }}</UiButton>
                  </div>
                </section>
              </div>

              <template #fallback>
                <UiLoadingState variant="form" :lines="3" :label="k('loading')" />
              </template>
            </ClientOnly>
          </div>
        </article>

        <ActivityRegistrationDialog
          v-if="dialogTarget"
          v-model:open="dialogOpen"
          :day="dialogTarget"
          :timezone="page.edition.timezone"
          :timezone-label="page.edition.city ?? undefined"
          @day-updated="(value: DayRegistration) => (registration = value)"
        />

        <UiModal v-model:open="confirmOpen" size="sm" :title="k('cancel.title')" :description="k('cancel.description')" :dismissible="!cancelling">
          <UiAlert v-if="cancelError" intent="danger" :message="cancelError" live />
          <template #footer>
            <UiButton variant="ghost" :disabled="cancelling" @click="confirmOpen = false">{{ k('cancel.keep') }}</UiButton>
            <UiButton variant="danger" :loading="cancelling" @click="confirmCancel">{{ k('cancel.confirm') }}</UiButton>
          </template>
        </UiModal>
      </template>
    </div>
  </div>
</template>
