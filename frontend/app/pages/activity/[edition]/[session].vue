<script setup lang="ts">
import type { PublicSessionStream } from '~/types/live'

/**
 * LA PAGE D'UNE ACTIVITÉ — celle qu'ouvre un clic dans la semaine ou la liste
 * du jour. Bandeau de l'édition, titre en grand, image et billet en léger débord
 * (arbitré le 05/10) ; pendant le direct, l'image cède la place au lecteur.
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
  const range = `${time(session.value.starts_at, visitor)} → ${time(session.value.ends_at, visitor)}`
  const sameInstant = range === `${time(session.value.starts_at, session.value.timezone)} → ${time(session.value.ends_at, session.value.timezone)}`
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
const hasDescription = computed(() => richTextToPlain(description.value).length > 0)
const summary = computed(() => (session.value?.summary ? tr(session.value.summary) : ''))
const dayLabel = computed(() => (session.value ? dayLong(session.value.starts_at, timezone.value) : ''))
const programmeTo = computed(() =>
  localePath({ name: 'programme', query: edition.value ? { edition: edition.value.slug, jour: session.value ? dayKeyInZone(session.value.starts_at, timezone.value) : undefined } : {} }),
)
const trail = computed(() =>
  edition.value ? t('activity.trail', { edition: edition.value.acronym ?? edition.value.edition_label ?? tr(edition.value.title), day: dayLabel.value }) : '',
)
const programmeListTo = computed(() =>
  localePath({ name: 'programme', query: edition.value ? { edition: edition.value.slug, vue: 'liste', jour: session.value ? dayKeyInZone(session.value.starts_at, timezone.value) : undefined } : {} }),
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
  <div class="full-bleed -mt-8 -mb-8 min-h-[75vh] bg-surface font-sans text-text sm:-mt-10 sm:-mb-10">
    <div v-if="status === 'pending' && !data" class="mx-auto max-w-[1280px] px-4 py-10 sm:px-6">
      <UiLoadingState variant="card" :lines="4" :label="t('activity.loading')" />
    </div>

    <div v-else-if="error" class="mx-auto max-w-[1280px] px-4 py-10 sm:px-6">
      <UiErrorState :title="t('activity.error.title')" :description="t('activity.error.description')" @retry="refresh()" />
    </div>

    <div v-else-if="!session || !edition || !detail" class="mx-auto max-w-[1280px] px-4 py-10 sm:px-6">
      <UiEmptyState
        icon="calendar"
        :title="t('activity.notFound.title')"
        :description="t('activity.notFound.description')"
        :action-label="t('activity.back')"
        :action-to="localePath('programme')"
      />
    </div>

    <template v-else>
      <ActivityHero :session="session" :edition="edition" :state="state" :back-to="programmeTo" :trail="trail" />

      <!-- Sur mobile, les deux colonnes s'effacent (`contents`) pour que le billet suive l'image. -->
      <div class="mx-auto max-w-[1280px] px-4 pb-16 sm:px-6">
        <div class="relative -mt-12 flex flex-col gap-10 lg:-mt-[72px] lg:grid lg:grid-cols-[minmax(0,1fr)_400px] lg:items-start lg:gap-x-8">
          <div class="contents min-w-0 lg:flex lg:flex-col lg:gap-14 lg:pr-8">
            <ActivityStage
              class="order-1 lg:-mr-8"
              :session="session"
              :state="state"
              :streams="streams"
              :cover="session.cover"
              :acronym="session.organization_acronym"
            />

            <section v-if="summary || hasDescription" class="order-4" aria-labelledby="a-propos-titre">
              <h2 id="a-propos-titre" class="border-b border-text pb-3 font-sans text-[26px] font-light text-text">
                <b class="font-bold">{{ t('activity.about') }}</b>
              </h2>
              <p v-if="summary" class="mt-5 text-xl leading-normal font-light">{{ summary }}</p>
              <UiRichContent class="mt-4 max-w-none! text-[17px] leading-relaxed" :html="description" />
            </section>

            <div class="order-5 flex flex-col gap-14 empty:hidden">
              <ActivityPeople
                :speakers="detail.speakers"
                :organizations="detail.organizations"
                :lead-logo="session.organization_logo ?? null"
              />
            </div>
          </div>

          <div class="contents lg:flex lg:flex-col lg:gap-10">
            <div class="order-2">
              <ActivityTicket
                :session="session"
                :edition="edition"
                :state="state"
                :languages="languages"
                :now="now"
                :home-time="homeTime"
                @started="refresh()"
              />
            </div>
            <ClientOnly>
              <ActivityQuestions v-if="showQuestions" class="order-3" :session-id="session.id" />
            </ClientOnly>
            <ActivitySameDay
              class="order-6"
              :sessions="sameDay"
              :current-id="session.id"
              :edition-slug="edition.slug"
              :timezone="timezone"
              :day-label="dayLabel"
              :day-to="programmeListTo"
            />
          </div>
        </div>
      </div>
    </template>
  </div>
</template>
