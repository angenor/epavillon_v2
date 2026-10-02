<script setup lang="ts">
import type { PublicSessionStream } from '~/types/live'

/**
 * LA PAGE D'UNE ACTIVITÉ — celle qu'ouvre un clic dans la semaine ou la liste
 * du jour. Même direction « affiche » que le programme (30/09).
 *
 * L'adresse porte l'édition et l'activité par leurs slugs : une séance n'a de
 * slug unique que dans son édition. Une activité inconnue et une activité non
 * publiée donnent la même réponse, comme l'API.
 *
 * Le direct, les questions et les autres activités du jour se chargent à part :
 * leur absence ne coûte jamais la page.
 */

definePageMeta({ layout: 'public' })
defineI18nRoute({ paths: { fr: '/programmations/[edition]/[session]', en: '/programmes/[edition]/[session]' } })

const route = useRoute()
const api = useApi()
const { t, locale } = useI18n()
const { tr } = useI18nText()
const { dayLong, time } = useDateTime()
const localePath = useLocalePath()
const { state: stateOf } = useProgrammeSession()
const visitedId = useState<string | null>('programme-visited', () => null)

const editionSlug = computed(() => String(route.params.edition))
const sessionSlug = computed(() => String(route.params.session))

const { data, status, error, refresh } = await useAsyncData(
  () => `activite-${editionSlug.value}-${sessionSlug.value}`,
  async () => {
    const edition = (await api.events.publicList()).find((entry) => entry.slug === editionSlug.value)
    if (!edition) return null
    const detail = await api.pavillon.activite(edition.id, sessionSlug.value)
    return detail ? { edition, detail } : null
  },
)

if (import.meta.server && status.value === 'success' && !data.value) setResponseStatus(useRequestEvent()!, 404)

const edition = computed(() => data.value?.edition ?? null)
const detail = computed(() => data.value?.detail ?? null)
const session = computed(() => detail.value?.session ?? null)
const state = computed(() => (session.value ? stateOf(session.value) : 'upcoming'))
const timezone = computed(() => session.value?.timezone ?? 'UTC')

const { data: schedule } = useLazyAsyncData(
  () => `activite-programme-${edition.value?.id ?? 'aucune'}`,
  async () => (edition.value ? api.sessions.schedule(edition.value.id).catch(() => []) : []),
  { default: () => [], watch: [edition] },
)

const sameDay = computed(() => {
  const current = session.value
  if (!current) return []
  const day = dayKeyInZone(current.starts_at, current.timezone)
  return (schedule.value ?? [])
    .filter((entry) => dayKeyInZone(entry.starts_at, current.timezone) === day)
    .sort((a, b) => a.starts_at.localeCompare(b.starts_at))
})

const streams = ref<PublicSessionStream[]>([])
const now = ref<number | null>(null)
let clock: ReturnType<typeof setInterval> | undefined

async function loadStreams(): Promise<void> {
  const current = session.value
  if (!current?.is_streamed || (state.value !== 'live' && state.value !== 'ongoing')) return
  streams.value = await api.live.sessionStreams(current.id).catch(() => [])
}

onMounted(() => {
  now.value = Date.now()
  clock = setInterval(() => (now.value = Date.now()), 30_000)
  if (session.value) visitedId.value = session.value.id
  void loadStreams()
})
onBeforeUnmount(() => clearInterval(clock))
watch(session, (current) => {
  streams.value = []
  if (current) visitedId.value = current.id
  void loadStreams()
})

const homeTime = computed(() => {
  if (now.value === null || !session.value) return null
  const visitor = Intl.DateTimeFormat().resolvedOptions().timeZone
  if (!visitor || visitor === session.value.timezone) return null
  const range = `${time(session.value.starts_at, visitor)} — ${time(session.value.ends_at, visitor)}`
  const sameInstant = range === `${time(session.value.starts_at, session.value.timezone)} — ${time(session.value.ends_at, session.value.timezone)}`
  return sameInstant ? null : { range, zone: t('activity.hero.yourZone', { zone: timeZoneCityLabel(visitor) }) }
})

const languages = computed(() =>
  (session.value?.language_codes ?? []).map((code) => {
    try {
      return new Intl.DisplayNames([locale.value], { type: 'language' }).of(code) ?? code
    } catch {
      return code
    }
  }),
)

const description = computed(() => (detail.value?.description ? tr(detail.value.description) : ''))
const summary = computed(() => (session.value?.summary ? tr(session.value.summary) : ''))
const dayLabel = computed(() => (session.value ? dayLong(session.value.starts_at, timezone.value) : ''))
const programmeTo = computed(() =>
  localePath({ name: 'programme', query: edition.value ? { edition: edition.value.slug, jour: session.value ? dayKeyInZone(session.value.starts_at, timezone.value) : undefined } : {} }),
)
const showQuestions = computed(
  () => Boolean(detail.value?.allows_questions) && state.value !== 'cancelled' && state.value !== 'past',
)

useHead(() => ({
  title: session.value ? tr(session.value.title) : t('activity.notFound.title'),
  meta: summary.value ? [{ name: 'description', content: summary.value }] : [],
}))
</script>

<template>
  <div class="full-bleed -mt-8 -mb-8 min-h-[75vh] bg-poster-paper text-poster-ink sm:-mt-10 sm:-mb-10">
    <div class="mx-auto flex w-full max-w-[1440px] flex-col px-4 pt-7 pb-16 sm:px-6 lg:px-12">
      <div class="flex flex-wrap items-center gap-x-6 gap-y-3">
        <NuxtLink
          :to="programmeTo"
          class="inline-flex h-11 items-center gap-2 rounded-md border-2 border-poster-ink bg-poster-paper-raised px-4 font-bold shadow-poster-sm transition-transform hover:-translate-y-0.5"
        >
          <UiIcon name="arrow-left" size="1rem" />
          {{ t('activity.back') }}
        </NuxtLink>
        <p v-if="edition" class="font-poster-mono text-sm text-poster-ink-muted">
          {{ t('activity.trail', { edition: edition.acronym ?? edition.edition_label ?? tr(edition.title), day: dayLabel }) }}
        </p>
      </div>

      <UiLoadingState v-if="status === 'pending' && !data" class="mt-10" variant="card" :lines="4" :label="t('activity.loading')" />

      <UiErrorState
        v-else-if="error"
        class="mt-10"
        :title="t('activity.error.title')"
        :description="t('activity.error.description')"
        @retry="refresh()"
      />

      <UiEmptyState
        v-else-if="!session || !edition || !detail"
        class="mt-10"
        icon="calendar"
        :title="t('activity.notFound.title')"
        :description="t('activity.notFound.description')"
        :action-label="t('activity.back')"
        :action-to="localePath('programme')"
      />

      <template v-else>
        <!-- Sur mobile, les deux colonnes s'effacent (`contents`) pour que le billet remonte sous le titre. -->
        <div class="mt-8 flex flex-col gap-10 lg:grid lg:grid-cols-[minmax(0,1fr)_26.5rem] lg:items-start lg:gap-10">
          <div class="contents min-w-0 lg:flex lg:flex-col lg:gap-10">
            <ActivityHero
              class="order-1"
              :session="session"
              :edition="edition"
              :state="state"
              :home-time="homeTime"
              @started="refresh()"
            />

            <ActivityStage class="order-3" :session="session" :state="state" :streams="streams" />

            <section v-if="summary || description" class="order-3" aria-labelledby="a-propos-titre">
              <h2 id="a-propos-titre" class="mb-5 border-b-4 border-poster-ink pb-2.5 font-poster text-[2.375rem] leading-none font-black uppercase font-stretch-[68%]">
                {{ t('activity.about') }}
              </h2>
              <p v-if="summary" class="max-w-[48rem] text-xl leading-normal font-medium">{{ summary }}</p>
              <p v-if="description" class="mt-4 max-w-[48rem] leading-relaxed whitespace-pre-line text-poster-ink-muted">{{ description }}</p>
            </section>

            <div class="order-3 flex flex-col gap-10 empty:hidden">
              <ActivityPeople :speakers="detail.speakers" :organizations="detail.organizations" />
            </div>
          </div>

          <div class="contents lg:flex lg:flex-col lg:gap-8">
            <ActivityTicket class="order-2" :session="session" :edition="edition" :state="state" :languages="languages" :now="now" />
            <ClientOnly>
              <ActivityQuestions v-if="showQuestions" class="order-4" :session-id="session.id" />
            </ClientOnly>
            <ActivitySameDay
              class="order-4"
              :sessions="sameDay"
              :current-id="session.id"
              :edition-slug="edition.slug"
              :timezone="timezone"
              :day-label="dayLabel"
            />
          </div>
        </div>
      </template>
    </div>
  </div>
</template>
