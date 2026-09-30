<script setup lang="ts">
import type {
  ProgrammeData,
  ProgrammeFilterState,
  ProgrammeStripDay,
  ProgrammeThemeOption,
  ProgrammeView,
} from '~/types/event-programme'
import type { PublicEditionRow, PublicScheduleRow } from '~/types/views'
import type { IsoDate } from '~/types/shared'
import type { LocationQueryRaw } from 'vue-router'

/**
 * Le corps de `/programmations`, sous le bandeau qui nomme l'édition : bande
 * des jours, filtres, puis la semaine ou la liste du jour. Direction « affiche »
 * arbitrée le 30/09 — jetons `poster`, bornés à ces écrans.
 *
 * Les deux vues lisent le même programme et les mêmes filtres. L'URL porte
 * l'édition, la vue et le jour : un lien vers « la liste du 12 novembre » se
 * colle dans un courriel. Un programme chargé est gardé : revenir à une
 * édition déjà vue ne rappelle pas l'API.
 */

interface Props {
  edition: PublicEditionRow
  initial: ProgrammeData
  editions: PublicEditionRow[]
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:edition': [edition: PublicEditionRow] }>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { zoneLabel, zoneOffsetShort } = useDateTime()
const api = useApi()
const route = useRoute()
const router = useRouter()
const localePath = useLocalePath()
const visitedId = useState<string | null>('programme-visited', () => null)

const editionById = computed(() => new Map(props.editions.map((edition) => [edition.id, edition])))
const selectedId = ref(props.edition.id)
const selectedEdition = computed(() => editionById.value.get(selectedId.value) ?? props.edition)

const loaded = reactive(new Map<string, ProgrammeData>([[props.edition.id, props.initial]]))
const loading = ref(false)
const failed = ref(false)
const data = computed<ProgrammeData>(() => loaded.get(selectedId.value) ?? { schedule: [], days: [], rooms: [] })

async function load(eventId: string): Promise<void> {
  if (loaded.has(eventId)) return
  loading.value = true
  failed.value = false
  try {
    const [schedule, days, rooms] = await Promise.all([
      api.sessions.schedule(eventId),
      api.events.days(eventId),
      api.events.rooms(eventId),
    ])
    loaded.set(eventId, { schedule, days, rooms })
  } catch {
    failed.value = true
  } finally {
    loading.value = false
  }
}

const EMPTY_FILTERS: ProgrammeFilterState = { themes: [], search: '', streamedOnly: false, hidePast: false }

async function select(eventId: string): Promise<void> {
  if (eventId === selectedId.value) return
  await load(eventId)
  if (failed.value) return
  selectedId.value = eventId
  emit('update:edition', selectedEdition.value)
  filters.value = { ...EMPTY_FILTERS }
  day.value = null
}

defineExpose({ select })

const view = ref<ProgrammeView>(route.query.vue === 'liste' ? 'list' : 'week')
const filters = ref<ProgrammeFilterState>({ ...EMPTY_FILTERS })
const day = ref<IsoDate | null>(typeof route.query.jour === 'string' ? route.query.jour : null)

const timezone = computed(() => selectedEdition.value.timezone)
const zone = computed(() =>
  t('programme.board.zone', {
    zone: zoneLabel(timezone.value, selectedEdition.value.city ?? undefined),
    offset: zoneOffsetShort(timezone.value, selectedEdition.value.starts_at),
  }),
)

const dayOf = (session: PublicScheduleRow): IsoDate => dayKeyInZone(session.starts_at, timezone.value)

const programmeDays = computed(() => [...new Set(data.value.schedule.map(dayOf))].sort())

/** Le jour ouvert : celui de l'URL s'il porte des activités, sinon le premier. */
const selectedDay = computed<IsoDate>(() => {
  const days = programmeDays.value
  return day.value && days.includes(day.value) ? day.value : (days[0] ?? dayKeyInZone(selectedEdition.value.starts_at, timezone.value))
})

function matches(session: PublicScheduleRow): boolean {
  const current = filters.value
  if (current.themes.length && !session.theme_codes.some((code) => current.themes.includes(code))) return false
  if (current.streamedOnly && !session.is_streamed) return false
  if (current.hidePast && session.temporal_state === 'past') return false
  const needle = foldText(current.search.trim())
  if (!needle) return true
  return foldText([tr(session.title), session.organization_name ?? '', session.organization_acronym ?? ''].join(' ')).includes(needle)
}

// L'horloge ne vit que dans le navigateur : l'heure du rendu serveur serait périmée à l'hydratation.
const today = ref<IsoDate | null>(null)
const nowMinutes = ref<number | null>(null)
let clock: ReturnType<typeof setInterval> | undefined

function tick(): void {
  today.value = dayKeyInZone(Date.now(), timezone.value)
  nowMinutes.value = minutesInZone(Date.now(), timezone.value)
}

onMounted(() => {
  tick()
  clock = setInterval(tick, 60_000)
  if (!day.value && today.value && programmeDays.value.includes(today.value)) day.value = today.value
  syncQuery()
})
onBeforeUnmount(() => clearInterval(clock))
watch(timezone, tick)

const stripDays = computed<ProgrammeStripDay[]>(() =>
  programmeDays.value.map((date, index) => {
    const kept = data.value.schedule.filter((session) => dayOf(session) === date && matches(session))
    const previous = programmeDays.value[index - 1]
    return {
      date,
      count: kept.length,
      colors: kept.map((session) => session.themes[0]?.color ?? null),
      isToday: date === today.value,
      gapBefore: previous ? daysBetween(previous, date) - 1 : 0,
    }
  }),
)

const weekDays = computed(() => {
  const start = weekStart(selectedDay.value)
  return programmeDays.value.filter((date) => weekStart(date) === start)
})

const weekSessions = computed(() => {
  const days = new Set(weekDays.value)
  return data.value.schedule.filter((session) => days.has(dayOf(session)))
})

const daySessions = computed(() =>
  data.value.schedule
    .filter((session) => dayOf(session) === selectedDay.value)
    .sort((a, b) => a.starts_at.localeCompare(b.starts_at)),
)

const range = computed(() => hourRange(data.value.schedule.map((session) => spanOf(session, timezone.value))))

const neighbour = (step: number): IsoDate | null => {
  const index = programmeDays.value.indexOf(selectedDay.value)
  return programmeDays.value[index + step] ?? null
}

const themeOptions = computed<ProgrammeThemeOption[]>(() => {
  const seen = new Map<string, ProgrammeThemeOption>()
  for (const session of data.value.schedule) {
    for (const theme of session.themes) {
      const entry = seen.get(theme.code)
      if (entry) entry.count += 1
      else seen.set(theme.code, { code: theme.code, label: tr(theme.label), color: theme.color, count: 1 })
    }
  }
  return [...seen.values()].sort((a, b) => a.label.localeCompare(b.label, locale.value))
})

const resultLabel = computed(() => {
  const scope = view.value === 'week' ? weekSessions.value : daySessions.value
  return t(view.value === 'week' ? 'programme.filters.resultWeek' : 'programme.filters.resultDay', {
    count: scope.filter(matches).length,
    total: scope.length,
  })
})

function syncQuery(): void {
  const query: LocationQueryRaw = { ...route.query, edition: selectedEdition.value.slug }
  if (view.value === 'list') query.vue = 'liste'
  else delete query.vue
  if (day.value) query.jour = day.value
  else delete query.jour
  void router.replace({ query })
}

watch([selectedId, view, day], syncQuery)

const isPublished = computed(() => selectedEdition.value.programme_published_at !== null)

const viewButton = (active: boolean) =>
  active ? 'bg-poster-ink text-poster-on-ink-accent' : 'bg-poster-paper-raised text-poster-ink'
</script>

<template>
  <section
    class="full-bleed -mb-8 bg-poster-paper text-poster-ink sm:-mb-10"
    aria-labelledby="programmation-titre"
  >
    <div class="mx-auto flex w-full max-w-[1440px] flex-col gap-5 px-4 py-10 sm:px-6 lg:px-12">
      <div class="flex flex-wrap items-baseline justify-between gap-x-6 gap-y-2">
        <h2
          id="programmation-titre"
          class="font-poster text-[clamp(2.25rem,5vw,3rem)] leading-none font-black tracking-[-0.01em] uppercase font-stretch-[68%]"
        >
          {{ t('programme.board.title') }}
        </h2>
        <p class="inline-flex items-center gap-2 font-poster-mono text-sm text-poster-ink-muted">
          <UiIcon name="clock" size="1rem" />
          {{ zone }}
        </p>
      </div>

      <UiErrorState
        v-if="failed"
        compact
        :title="t('programme.error.title')"
        :description="t('programme.error.description')"
        @retry="load(selectedId)"
      />

      <UiLoadingState v-else-if="loading" variant="card" :lines="3" :label="t('programme.loading')" />

      <UiEmptyState
        v-else-if="!isPublished"
        icon="calendar"
        :title="t('programme.unpublished.title')"
        :description="t('programme.unpublished.description')"
        :action-label="t('programme.unpublished.backToEvent')"
        :action-to="localePath(`/evenements/${selectedEdition.slug}`)"
      />

      <UiEmptyState
        v-else-if="!data.schedule.length"
        icon="calendar"
        :title="t('programme.empty.title')"
        :description="t('programme.empty.description')"
      />

      <template v-else>
        <EventProgrammeDayStrip
          :days="stripDays"
          :selected="selectedDay"
          :week="view === 'week' ? weekDays : [selectedDay]"
          :timezone="timezone"
          @select="day = $event"
        />

        <EventProgrammeFilters v-model="filters" :themes="themeOptions" :result-label="resultLabel">
          <template #view>
            <div
              class="flex overflow-hidden rounded-md border-2 border-poster-ink shadow-poster"
              role="group"
              :aria-label="t('programme.views.label')"
            >
              <button
                type="button"
                class="inline-flex h-13 flex-1 cursor-pointer items-center justify-center gap-2 px-5 font-bold whitespace-nowrap"
                :class="viewButton(view === 'week')"
                :aria-pressed="view === 'week'"
                @click="view = 'week'"
              >
                <UiIcon name="grid" size="1.125rem" />
                {{ t('programme.views.week') }}
              </button>
              <button
                type="button"
                class="inline-flex h-13 flex-1 cursor-pointer items-center justify-center gap-2 border-l-2 border-poster-ink px-5 font-bold whitespace-nowrap"
                :class="viewButton(view === 'list')"
                :aria-pressed="view === 'list'"
                @click="view = 'list'"
              >
                <UiIcon name="list" size="1.125rem" />
                {{ t('programme.views.list') }}
              </button>
            </div>
          </template>
        </EventProgrammeFilters>

        <div class="mt-3">
          <EventProgrammeWeek
            v-if="view === 'week'"
            :days="weekDays"
            :sessions="weekSessions"
            :matches="matches"
            :timezone="timezone"
            :range="range"
            :selected="selectedDay"
            :today="today"
            :now-minutes="nowMinutes"
            :visited-id="visitedId"
            :edition-slug="selectedEdition.slug"
          />
          <EventProgrammeDayList
            v-else
            :day="selectedDay"
            :sessions="daySessions"
            :matches="matches"
            :timezone="timezone"
            :zone="zone"
            :previous="neighbour(-1)"
            :next="neighbour(1)"
            :visited-id="visitedId"
            :edition-slug="selectedEdition.slug"
            @select="day = $event"
            @reset="filters = { ...EMPTY_FILTERS }"
          />
        </div>
      </template>
    </div>
  </section>
</template>
