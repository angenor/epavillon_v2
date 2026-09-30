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
 * Le corps de `/programmations`, sous le bandeau qui nomme l'édition : une
 * barre d'une ligne, puis la semaine ou la liste. Direction « affiche »
 * arbitrée le 30/09 — jetons `poster`, bornés à ces écrans. Les filtres ne
 * passent jamais devant le tableau (30/09).
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

const EMPTY_FILTERS: ProgrammeFilterState = { themes: [], search: '' }

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

const list = useTemplateRef<{ scrollToDay: (date: IsoDate, smooth?: boolean) => void }>('list')
const strip = useTemplateRef<HTMLElement>('strip')
const stickyOffset = ref(0)

function measure(): void {
  const root = getComputedStyle(document.documentElement)
  const nav = parseFloat(root.getPropertyValue('--nav-height')) * parseFloat(root.fontSize)
  stickyOffset.value = (Number.isFinite(nav) ? nav : 0) + (strip.value?.offsetHeight ?? 0)
}

async function showInList(date: IsoDate, smooth = true): Promise<void> {
  day.value = date
  view.value = 'list'
  await nextTick()
  measure()
  list.value?.scrollToDay(date, smooth)
}

onMounted(() => {
  tick()
  clock = setInterval(tick, 60_000)
  const fromUrl = day.value
  if (!day.value && today.value && programmeDays.value.includes(today.value)) day.value = today.value
  syncQuery()
  measure()
  window.addEventListener('resize', measure)
  if (view.value === 'list' && fromUrl) void showInList(fromUrl, false)
})
onBeforeUnmount(() => {
  clearInterval(clock)
  window.removeEventListener('resize', measure)
})
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

const weeks = computed(() => [...new Set(programmeDays.value.map(weekStart))])
const weekIndex = computed(() => weeks.value.indexOf(weekStart(selectedDay.value)))

function moveWeek(step: -1 | 1): void {
  const target = weeks.value[weekIndex.value + step]
  const first = programmeDays.value.find((date) => weekStart(date) === target)
  if (first) day.value = first
}

/** Un clic sur un jour de la semaine : l'onglet de la colonne sur mobile, sa liste ailleurs. */
function onWeekDay(date: IsoDate): void {
  if (window.matchMedia('(min-width: 48rem)').matches) void showInList(date)
  else day.value = date
}

const weekLabel = computed(() => {
  const days = weekDays.value
  const format = new Intl.DateTimeFormat(locale.value, { day: 'numeric', month: 'long', timeZone: 'UTC' })
  const first = new Date(`${days[0]}T12:00:00Z`)
  const last = new Date(`${days[days.length - 1]}T12:00:00Z`)
  return days.length > 1 ? format.formatRange(first, last) : format.format(first)
})

const weekSummary = computed(() =>
  t('programme.week.summary', {
    week: weekIndex.value + 1,
    weeks: weeks.value.length,
    count: weekSessions.value.filter(matches).length,
    total: weekSessions.value.length,
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

const sortedSchedule = computed(() => [...data.value.schedule].sort((a, b) => a.starts_at.localeCompare(b.starts_at)))

const range = computed(() => hourRange(data.value.schedule.map((session) => spanOf(session, timezone.value))))


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

const resultCount = computed(() => data.value.schedule.filter(matches).length)

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
    <div class="mx-auto flex w-full max-w-[1440px] flex-col gap-4 px-4 pt-7 pb-12 sm:px-6 lg:px-12">
      <UiErrorState
        v-if="failed"
        compact
        :title="t('programme.error.title')"
        :description="t('programme.error.description')"
        @retry="load(selectedId)"
      />

      <UiLoadingState v-else-if="loading" variant="card" :lines="3" :label="t('programme.loading')" />

      <template v-else>
        <EventProgrammeFilters
          v-if="isPublished && data.schedule.length"
          v-model="filters"
          :themes="themeOptions"
          :result-count="resultCount"
        >
          <template #title>
            <h2
              id="programmation-titre"
              class="mr-auto basis-full font-poster text-[2rem] leading-none font-black tracking-[-0.01em] uppercase font-stretch-[68%] sm:basis-auto sm:text-4xl"
            >
              {{ t('programme.board.title') }}
            </h2>
          </template>
          <template #view>
            <div class="flex overflow-hidden rounded-md border-2 border-poster-ink" role="group" :aria-label="t('programme.views.label')">
              <button
                type="button"
                class="inline-flex h-10 cursor-pointer items-center gap-1.5 px-3 text-sm font-bold whitespace-nowrap sm:px-3.5"
                :class="viewButton(view === 'week')"
                :aria-pressed="view === 'week'"
                @click="view = 'week'"
              >
                <UiIcon name="grid" size="1rem" class="hidden sm:block" />
                {{ t('programme.views.week') }}
              </button>
              <button
                type="button"
                class="inline-flex h-10 cursor-pointer items-center gap-1.5 border-l-2 border-poster-ink px-3 text-sm font-bold whitespace-nowrap sm:px-3.5"
                :class="viewButton(view === 'list')"
                :aria-pressed="view === 'list'"
                @click="showInList(selectedDay)"
              >
                <UiIcon name="list" size="1rem" class="hidden sm:block" />
                {{ t('programme.views.list') }}
              </button>
            </div>
          </template>
        </EventProgrammeFilters>
        <h2 v-else id="programmation-titre" class="font-poster text-4xl leading-none font-black uppercase font-stretch-[68%]">
          {{ t('programme.board.title') }}
        </h2>

        <UiEmptyState
          v-if="!isPublished"
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

        <EventProgrammeWeek
          v-else-if="view === 'week'"
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
          :label="weekLabel"
          :summary="weekSummary"
          :zone="zone"
          :has-previous="weekIndex > 0"
          :has-next="weekIndex < weeks.length - 1"
          @week="moveWeek"
          @day="onWeekDay"
        />

        <div v-else>
          <div ref="strip" class="sticky top-(--nav-height) z-20 -mx-1 border-b-2 border-poster-ink bg-poster-paper px-1 pt-2">
            <EventProgrammeDayStrip :days="stripDays" :selected="selectedDay" :timezone="timezone" @select="showInList" />
            <p class="pb-2 font-poster-mono text-xs text-poster-ink-muted">{{ zone }}</p>
          </div>
          <EventProgrammeDayList
            ref="list"
            :days="programmeDays"
            :sessions="sortedSchedule"
            :matches="matches"
            :timezone="timezone"
            :visited-id="visitedId"
            :edition-slug="selectedEdition.slug"
            :sticky-offset="stickyOffset"
            @reading="day = $event"
            @reset="filters = { ...EMPTY_FILTERS }"
          />
        </div>
      </template>
    </div>
  </section>
</template>
